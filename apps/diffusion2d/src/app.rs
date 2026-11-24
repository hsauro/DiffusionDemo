//! UI and input for the 2D diffusion painter. All simulation lives in `rd-core`.

use egui::{
    Align2, Color32, ColorImage, ComboBox, CursorIcon, FontId, Key, Modifiers, Pos2, Rect, Sense,
    Slider, SliderClamping, Stroke, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use rd_core::{Boundary, BrushMode, Grid, StepPlan, add_gaussian_spot, diffuse, plan_frame};

const GRID_SIZES: [usize; 3] = [100, 200, 300];
const DEFAULT_GRID: usize = 200;

/// "Step" advances the time one frame at 60 fps would.
const STEP_SECONDS: f64 = 1.0 / 60.0;

/// The "Drop a spot" preset: amount 25 with sigma 2 gives a peak of about 1.0.
const SPOT_AMOUNT: f32 = 25.0;
const SPOT_SIGMA: f32 = 2.0;

/// Fixed top of the colour scale when "Auto scale" is off.
const FIXED_SCALE_MAX: f32 = 1.0;

const COLOUR_BAR_WIDTH: f32 = 18.0;
const COLOUR_BAR_GUTTER: f32 = 70.0; // bar plus its numbers

pub struct DiffusionApp {
    grid: Grid,
    texture: Option<TextureHandle>,

    // Parameters.
    d: f64,
    speed: f64,
    brush_radius: f32,
    brush_strength: f32,
    boundary: Boundary,
    grid_n: usize,
    auto_scale: bool,

    // Run state.
    playing: bool,
    time: f64,
    last_plan: StepPlan,
    fps: f64,

    /// Pointer position (grid coordinates) of the last dab of the current stroke.
    last_paint: Option<(f32, f32)>,
    /// Where the canvas was drawn last frame, for the hover readout.
    canvas_rect: Option<Rect>,
}

impl DiffusionApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            grid: Grid::new(DEFAULT_GRID),
            texture: None,
            d: 1.0,
            speed: 20.0,
            brush_radius: 8.0,
            brush_strength: 0.3,
            boundary: Boundary::NoFlux,
            grid_n: DEFAULT_GRID,
            auto_scale: false,
            playing: false,
            time: 0.0,
            last_plan: StepPlan {
                substeps: 0,
                dt: 0.0,
                advance: 0.0,
                limited: false,
            },
            fps: 0.0,
            last_paint: None,
            canvas_rect: None,
        }
    }

    fn clear(&mut self) {
        self.grid.clear();
        self.time = 0.0;
        self.last_paint = None;
    }

    fn drop_spot(&mut self) {
        self.clear();
        let c = (self.grid.n - 1) as f32 / 2.0;
        add_gaussian_spot(&mut self.grid, c, c, SPOT_SIGMA, SPOT_AMOUNT);
        self.playing = false;
    }

    /// Advance the simulation by a frame lasting `real_seconds` of real time.
    fn advance(&mut self, real_seconds: f64) {
        let plan = plan_frame(self.d, self.speed, real_seconds);
        let (d, dt) = (self.d as f32, plan.dt as f32);
        for _ in 0..plan.substeps {
            diffuse(&mut self.grid, d, dt, self.boundary);
        }
        self.time += plan.advance;
        self.last_plan = plan;
    }

    fn colour_scale_max(&self) -> f32 {
        if self.auto_scale {
            let m = self.grid.max();
            if m > 0.0 { m } else { FIXED_SCALE_MAX }
        } else {
            FIXED_SCALE_MAX
        }
    }

    fn sigma(&self) -> f32 {
        self.brush_radius / 2.0
    }

    /// Keyboard shortcuts, unless a text field (e.g. the D number box) has focus.
    /// The keys are consumed so a focused button does not also react to Space.
    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (space, step, clear) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Space),
                i.consume_key(Modifiers::NONE, Key::S),
                i.consume_key(Modifiers::NONE, Key::C),
            )
        });
        if space {
            self.playing = !self.playing;
        }
        if step && !self.playing {
            self.advance(STEP_SECONDS);
        }
        if clear {
            self.clear();
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().slider_width = 160.0;
        ui.add_space(6.0);
        ui.heading("Diffusion");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            let label = if self.playing { "Pause" } else { "Play" };
            if ui.button(label).on_hover_text("Space").clicked() {
                self.playing = !self.playing;
            }
            if ui
                .add_enabled(!self.playing, egui::Button::new("Step"))
                .on_hover_text("S: advance one frame's worth of time")
                .clicked()
            {
                self.advance(STEP_SECONDS);
            }
            if ui.button("Clear").on_hover_text("C").clicked() {
                self.clear();
            }
        });

        ui.separator();
        ui.label("Diffusion constant D (cells\u{b2}/time)");
        ui.add(
            Slider::new(&mut self.d, 0.01..=10.0)
                .logarithmic(true)
                .clamping(SliderClamping::Always)
                .max_decimals(3),
        );
        ui.label("Speed (time units per second)");
        ui.add(
            Slider::new(&mut self.speed, 1.0..=200.0)
                .logarithmic(true)
                .clamping(SliderClamping::Always)
                .max_decimals(1),
        );
        ui.label("Brush radius (cells)");
        ui.add(Slider::new(&mut self.brush_radius, 2.0..=30.0).max_decimals(1));
        ui.label("Brush strength (per dab)");
        ui.add(Slider::new(&mut self.brush_strength, 0.05..=1.0).max_decimals(2));

        ui.separator();
        egui::Grid::new("choices")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("Boundary");
                ComboBox::from_id_salt("boundary")
                    .selected_text(self.boundary.label())
                    .show_ui(ui, |ui| {
                        for bc in Boundary::ALL {
                            ui.selectable_value(&mut self.boundary, bc, bc.label());
                        }
                    });
                ui.end_row();

                ui.label("Grid size");
                let before = self.grid_n;
                ComboBox::from_id_salt("grid_size")
                    .selected_text(format!("{0} \u{d7} {0}", self.grid_n))
                    .show_ui(ui, |ui| {
                        for n in GRID_SIZES {
                            ui.selectable_value(&mut self.grid_n, n, format!("{n} \u{d7} {n}"));
                        }
                    });
                if self.grid_n != before {
                    self.grid = Grid::new(self.grid_n);
                    self.clear();
                }
                ui.end_row();
            });
        ui.checkbox(&mut self.auto_scale, "Auto scale colours");

        ui.separator();
        if ui
            .button("Drop a spot")
            .on_hover_text(
                "One Gaussian spot (amount 25, \u{3c3} = 2) at the centre; pauses at t = 0",
            )
            .clicked()
        {
            self.drop_spot();
        }

        ui.separator();
        ui.weak("Left drag: paint");
        ui.weak("Right drag or Shift + drag: erase");
        ui.weak("Space: play/pause  \u{b7}  S: step  \u{b7}  C: clear");
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.monospace(format!("t = {:.2}", self.time));
            ui.separator();
            ui.monospace(format!("total = {}", significant(self.grid.total(), 7)));
            ui.separator();
            ui.monospace(format!(
                "max u = {}",
                significant(f64::from(self.grid.max()), 4)
            ));
            ui.separator();
            let substeps = if self.playing {
                self.last_plan.substeps
            } else {
                0
            };
            ui.monospace(format!("substeps/frame = {substeps}"));
            ui.separator();
            if self.playing {
                ui.monospace(format!("{:.0} fps", self.fps));
            } else {
                ui.monospace("paused");
            }
            if self.playing && self.last_plan.limited {
                ui.separator();
                ui.colored_label(ui.visuals().warn_fg_color, "running slower than requested");
            }
            if let Some((i, j)) = self.hovered_cell(ui.ctx()) {
                ui.separator();
                let u = self.grid.get(i, j);
                ui.monospace(format!(
                    "cell ({i}, {j})  u = {}",
                    significant(f64::from(u), 4)
                ));
            }
        });
    }

    /// Cell under the pointer, using last frame's canvas placement.
    fn hovered_cell(&self, ctx: &egui::Context) -> Option<(usize, usize)> {
        let rect = self.canvas_rect?;
        let p = ctx.input(|i| i.pointer.hover_pos())?;
        if !rect.contains(p) {
            return None;
        }
        let n = self.grid.n;
        let cell = rect.width() / n as f32;
        let i = (((p.x - rect.min.x) / cell) as usize).min(n - 1);
        let j = (((p.y - rect.min.y) / cell) as usize).min(n - 1);
        Some((i, j))
    }

    fn canvas(&mut self, ui: &mut egui::Ui, real_seconds: f64) {
        let ctx = ui.ctx().clone();
        let n = self.grid.n;

        // Largest square that fits next to the colour bar, centred.
        let avail = ui.available_rect_before_wrap();
        let side = (avail.width() - COLOUR_BAR_GUTTER)
            .min(avail.height())
            .max(64.0);
        let left = avail.min.x + ((avail.width() - side - COLOUR_BAR_GUTTER) / 2.0).max(0.0);
        let top = avail.min.y + ((avail.height() - side) / 2.0).max(0.0);
        let rect = Rect::from_min_size(pos2(left, top), Vec2::splat(side));
        let cell = side / n as f32;
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        self.canvas_rect = Some(rect);

        let to_grid = |p: Pos2| {
            (
                (p.x - rect.min.x) / cell - 0.5,
                (p.y - rect.min.y) / cell - 0.5,
            )
        };

        // 1. Painting.
        let mut painting = false;
        if response.is_pointer_button_down_on() {
            let (primary, secondary, shift, pos) = ctx.input(|i| {
                (
                    i.pointer.primary_down(),
                    i.pointer.secondary_down(),
                    i.modifiers.shift,
                    i.pointer.interact_pos(),
                )
            });
            let mode = if secondary || (primary && shift) {
                Some(BrushMode::Erase)
            } else if primary {
                Some(BrushMode::Add)
            } else {
                None
            };
            if let (Some(mode), Some(p)) = (mode, pos) {
                let at = to_grid(p);
                let from = self.last_paint.unwrap_or(at);
                let sigma = self.sigma();
                rd_core::stamp_line(&mut self.grid, from, at, sigma, self.brush_strength, mode);
                self.last_paint = Some(at);
                painting = true;
            }
        }
        if !painting {
            self.last_paint = None;
        }

        // 2. Substeps.
        if self.playing {
            self.advance(real_seconds);
            let dt = real_seconds.max(1e-6);
            self.fps = if self.fps > 0.0 {
                0.9 * self.fps + 0.1 / dt
            } else {
                1.0 / dt
            };
        } else {
            self.fps = 0.0;
        }

        // 3. Texture, once per frame.
        let scale_max = self.colour_scale_max();
        let inv = 1.0 / scale_max;
        let pixels = self
            .grid
            .u
            .iter()
            .map(|&v| {
                let [r, g, b] = rd_core::viridis(v * inv);
                Color32::from_rgb(r, g, b)
            })
            .collect();
        let image = ColorImage::new([n, n], pixels);
        let texture = match &mut self.texture {
            Some(t) => {
                t.set(image, TextureOptions::NEAREST);
                t
            }
            None => self.texture.insert(ctx.load_texture(
                "concentration",
                image,
                TextureOptions::NEAREST,
            )),
        };

        // 4. Draw.
        let painter = ui.painter_at(avail);
        painter.image(
            texture.id(),
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        if let Some(p) = response.hover_pos() {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
            let r = self.brush_radius * cell;
            painter.circle_stroke(p, r, Stroke::new(2.5, Color32::from_black_alpha(140)));
            painter.circle_stroke(p, r, Stroke::new(1.0, Color32::WHITE));
        }
        let bar = Rect::from_min_size(
            pos2(rect.max.x + 12.0, rect.min.y),
            vec2(COLOUR_BAR_WIDTH, side),
        );
        colour_bar(&painter, bar, scale_max, ui.visuals().text_color());

        // Keep painting while the button is held still, and refresh the status
        // bar after any change made while paused.
        if painting || self.playing {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for DiffusionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let real_seconds = f64::from(ctx.input(|i| i.stable_dt));

        let before = (self.time, self.grid_n, self.playing);
        self.handle_keys(&ctx);

        egui::Panel::left("controls")
            .resizable(false)
            .default_size(220.0)
            .show(ui, |ui| self.controls(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.add_space(2.0);
            self.status_bar(ui);
            ui.add_space(2.0);
        });
        egui::CentralPanel::default().show(ui, |ui| self.canvas(ui, real_seconds));

        // A button press or key changed state after the status bar was drawn:
        // draw once more so the status bar is current.
        if (self.time, self.grid_n, self.playing) != before {
            ctx.request_repaint();
        }
    }
}

/// Vertical viridis bar with ticks at 0, max/2 and max.
fn colour_bar(painter: &egui::Painter, bar: Rect, max: f32, text: Color32) {
    const STRIPS: usize = 128;
    let h = bar.height() / STRIPS as f32;
    for k in 0..STRIPS {
        let t = (k as f32 + 0.5) / STRIPS as f32;
        let [r, g, b] = rd_core::viridis(t);
        let y1 = bar.max.y - k as f32 * h;
        let strip = Rect::from_x_y_ranges(bar.x_range(), (y1 - h - 0.5)..=y1);
        painter.rect_filled(strip, 0.0, Color32::from_rgb(r, g, b));
    }
    for (frac, value) in [(0.0, 0.0), (0.5, max / 2.0), (1.0, max)] {
        let y = bar.max.y - frac * bar.height();
        painter.line_segment(
            [pos2(bar.max.x, y), pos2(bar.max.x + 4.0, y)],
            Stroke::new(1.0, text),
        );
        painter.text(
            pos2(bar.max.x + 6.0, y),
            Align2::LEFT_CENTER,
            significant(f64::from(value), 3),
            FontId::monospace(12.0),
            text,
        );
    }
}

/// Format `v` with about `digits` significant digits.
fn significant(v: f64, digits: i32) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let decimals = (digits - 1 - v.abs().log10().floor() as i32).clamp(0, 8) as usize;
    format!("{v:.decimals$}")
}

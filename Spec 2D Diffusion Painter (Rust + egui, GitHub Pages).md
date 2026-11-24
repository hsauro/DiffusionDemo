# Spec: 2D Diffusion Painter (Rust + egui, GitHub Pages)

Nov 24, 2026 · @Herbert Sauro

## Purpose

Build a browser app in which students paint concentration onto a 2D grid with the mouse and watch it diffuse, with the diffusion constant set by a slider. It is written in Rust with egui, compiled to WebAssembly and served from GitHub Pages.

This is the first of several interactive tools for an undergraduate computational systems biology class. Later tools add reaction terms (Brusselator, Gray–Scott), so the simulation core and the page layout should be easy to reuse. Students use it in lecture and on their own laptops; they should not need to install anything.

The teaching points it must make visible:

- A painted spot spreads and flattens; its total amount stays constant.
- Spreading distance grows as the square root of time: twice as far takes four times as long.
- A larger diffusion constant spreads faster but gives the same shapes.

## Scope

v1 is pure diffusion of one species on a square grid, with painting, a diffusion slider, and run controls.

**In scope**

- One species, concentration u(x, y, t) on an N × N grid (default N = 200).
- Paint and erase with the mouse while the simulation runs or is paused.
- Sliders for diffusion constant, brush size and brush strength; a simulation-speed control.
- Play/pause, single step, clear, and a choice of boundary condition.
- Readouts of simulated time and total amount.
- Runs in current Chrome, Firefox and Safari on a laptop; also runs natively with `cargo run` for development.

**Not in v1**

- Reaction terms, more than one species, or anisotropic diffusion.
- Saving or loading states, images or video export.
- Touch-screen support beyond what egui gives by default.
- GPU compute shaders. The CPU is fast enough at this grid size; revisit only if a later tool needs larger grids.

## Model and numerics

Solve the diffusion equation with an explicit five-point finite-difference scheme, taking enough substeps per frame to stay stable.

```latex
\frac{\partial u}{\partial t} = D\left(\frac{\partial^2 u}{\partial x^2} + \frac{\partial^2 u}{\partial y^2}\right)
```

**Grid and units.** N × N cells, spacing Δx = 1, so lengths are in cells and D is in cells² per unit time. Store u as `f32` in a flat `Vec` (row-major), with two buffers swapped each step. No allocation inside the time loop.

**Update.** For each cell: u\_new = u + r (u\_E + u\_W + u\_N + u\_S − 4u), with r = D Δt / Δx².

**Stability.** The scheme is stable for r ≤ 1/4. Keep r ≤ 0.2:

1. Each frame advances simulated time by T\_frame = speed × (real seconds since last frame), with the real interval capped at 1/30 s so a background tab does not jump.
2. Substeps n = ceil(D × T\_frame / 0.2), at least 1, at most 100.
3. Δt = T\_frame / n.
4. If n hits the cap, advance only 100 × 0.2 / D instead and show “running slower than requested” in the status bar.

**Boundaries.** A dropdown with three choices:

| Boundary | Ghost-cell rule | What students see |
| --- | --- | --- |
| No flux (default) | ghost = edge cell | total amount stays constant |
| Periodic | ghost = opposite edge | a spot leaving one side enters the other |
| Absorbing | ghost = 0 | total amount falls as material leaves |

**Total amount.** Sum u over all cells in `f64`. With no-flux boundaries it must stay constant to within 1 part in 10⁵ over 10⁴ steps (f32 round-off).

**Ranges.**

| Control | Range | Default | Scale |
| --- | --- | --- | --- |
| D | 0.01–10 cells²/time | 1 | logarithmic |
| Speed | 1–200 time units per second | 20 | logarithmic |
| Grid N | 100, 200, 300 | 200 | dropdown; changing it clears the grid |

## Interface

One window: a control panel on the left, the square canvas filling the rest, and a status bar along the bottom.

**Canvas**

- Draw the grid as an egui texture, one texel per cell, scaled to the largest square that fits, with nearest-neighbour filtering so cells stay crisp.
- Colour with viridis. Colour scale runs from 0 to a maximum that is fixed by default (1.0), with an “Auto scale” checkbox that uses the current maximum instead. Show a colour bar with its numbers beside the canvas.
- Update the texture once per frame, after the substeps.

**Painting**

- Left drag adds material: u += strength × exp(−d² / 2σ²) for cells within 2σ of the pointer, where σ = brush radius / 2. A press without moving adds one dab per frame while held.
- Right drag (or Shift + left drag, for trackpads) erases: u ×= 1 − exp(−d² / 2σ²).
- Between frames, stamp along the line from the last pointer position to the new one at spacing σ, so fast strokes are continuous rather than dotted.
- Clamp u ≥ 0 after painting.
- Painting works while running and while paused.
- Show the brush outline as a circle under the pointer.

**Control panel**, top to bottom

| Control | Widget | Notes |
| --- | --- | --- |
| Play / Pause | button, also Space | starts paused with an empty grid |
| Step | button, also S | advances one frame’s worth of time |
| Clear | button, also C | sets u = 0 and time = 0 |
| Diffusion D | log slider + number box | changes take effect immediately |
| Speed | log slider | simulated time per real second |
| Brush radius | slider, 2–30 cells | default 8 |
| Brush strength | slider, 0.05–1 per dab | default 0.3 |
| Boundary | dropdown | No flux, Periodic, Absorbing |
| Grid size | dropdown | 100, 200, 300 |
| Auto scale | checkbox | off by default |
| Preset | button “Drop a spot” | puts one Gaussian of amount 25 (peak about 1.0) at the centre, σ = 2, and pauses at t = 0 |

**Status bar:** simulated time t, total amount, maximum u, substeps per frame, and frames per second. When hovering over the canvas, also show the cell (i, j) and its u.

The “Drop a spot” preset is for the square-root-of-time demonstration: run it at two values of D and compare how far the spot has spread at the same t.

## Architecture

One Cargo workspace: a simulation crate with no GUI dependency, and one thin eframe app per tool.

```
DiffusionDemo/
  Cargo.toml                 # workspace
  crates/rd-core/            # grid, Laplacian, boundaries, brush, colormaps; no egui
  apps/diffusion2d/          # eframe app: UI and input only
    src/main.rs
    index.html               # trunk entry point
  site/index.html            # landing page linking to each tool
  .github/workflows/pages.yml
```

**rd-core**

- `Grid { n, u: Vec<f32>, scratch: Vec<f32> }` with `total() -> f64` and `max() -> f32`.
- `fn diffuse(grid: &mut Grid, d: f32, dt: f32, bc: Boundary)` doing one explicit step; `Boundary` is an enum of the three choices.
- `fn stamp(grid: &mut Grid, x: f32, y: f32, sigma: f32, strength: f32, mode: BrushMode)` where `BrushMode` is `Add` or `Erase`.
- `fn viridis(t: f32) -> [u8; 3]` from a 256-entry lookup table.
- Plain loops over a flat slice. Handle the edges with explicit loops rather than branching inside the inner loop.

**apps/diffusion2d**

- Start from the official `eframe_template`, which already builds for native and web and includes a GitHub Pages workflow.
- The app struct holds the grid, parameters, run state and last pointer position. Each `update()` call: handle input and painting, run the substeps if playing, rebuild the `ColorImage` into a persistent `TextureHandle`, draw the panels.
- Call `ctx.request_repaint()` while playing; when paused, repaint only on input.

Later tools add their reaction terms to `rd-core` and get their own folder under `apps/`.

## Build and deployment

Build with trunk and deploy from a GitHub Actions workflow to GitHub Pages; each tool lives under its own path.

**Local development**

- Native: `cargo run -p diffusion2d --release`.
- Web: `trunk serve` inside `apps/diffusion2d`, then open the printed localhost address.

**Deployed URLs**

- Landing page: `https://<owner>.github.io/DiffusionDemo/`
- This tool: `https://<owner>.github.io/DiffusionDemo/diffusion2d/`

**Workflow** (`.github/workflows/pages.yml`, on push to `main`)

1. Install stable Rust with the `wasm32-unknown-unknown` target, and install trunk.
2. For each app: `trunk build --release --public-url /DiffusionDemo/<app>/ --dist ../../dist/<app>`.
3. Copy `site/index.html` to `dist/index.html`.
4. Upload `dist` with `actions/upload-pages-artifact` and publish with `actions/deploy-pages`.

In the repository settings, set the Pages source to “GitHub Actions”. Cache the cargo registry and target folder to keep builds short.

The landing page is plain HTML: a title, one line per tool with a link and a one-sentence description, and a link to the course page.

## Testing and acceptance

The simulation is tested against physics in `rd-core` unit tests; the app is accepted by a short manual checklist.

**Unit tests in rd-core** (`cargo test -p rd-core`, run in CI before deploy)

1. Conservation: random initial u, no-flux and periodic boundaries, 10⁴ steps at r = 0.2. Relative change in total < 10⁻⁵.
2. Absorbing boundary: total never increases from one step to the next.
3. Spreading: a spot at the centre of a 201 × 201 grid, D = 1, run to t = 50. The variance along each axis equals its starting value plus 2Dt to within 1%.
4. Symmetry: that same spot stays symmetric under x and y reflection to within 10⁻⁶.
5. Maximum principle: with r ≤ 0.25 and no painting, max u never increases and min u never goes below 0.
6. Brush: one Add stamp increases the total by strength × 2πσ² to within 5% when the brush is away from the edges.
7. Substep rule: for every D and speed on the slider ranges, the chosen Δt gives r ≤ 0.2.

**Manual acceptance**

- [ ] At N = 200, D = 1, speed 20: about 60 frames per second in Chrome on a typical student laptop. At N = 300, D = 10: at least 30.
- [ ] Fast mouse strokes leave continuous lines.
- [ ] With no-flux boundaries the total in the status bar stays fixed while the colours spread.
- [ ] With absorbing boundaries the total falls once material reaches an edge.
- [ ] “Drop a spot” at D = 1 and D = 4: at t = 50, the D = 4 spot is about twice as wide (standard deviation about 20 cells against 10).
- [ ] Works in current Chrome, Firefox and Safari, and with a trackpad using Shift + drag to erase.
- [ ] The deployed page loads with no console errors, and the landing page links to it.

## Later tools

Not part of this job. They are listed only so the `rd-core` API stays general enough for them: two species, per-cell parameters and reaction terms.

| Tool | Adds to rd-core |
| --- | --- |
| Brusselator 2D | two species with separate D; reaction step; noise seeding |
| Prepattern painter | a painted per-cell parameter map (b inside and outside a shape) |
| Gray–Scott | feed and kill rates; self-replicating spots |

//! Simulation core for the reaction-diffusion teaching tools.
//!
//! This crate has no GUI dependency. It holds the concentration grid, the
//! explicit diffusion step with its boundary conditions, the paint brush,
//! the substep (stability) rule and the colour map. Each tool under `apps/`
//! is a thin eframe front end over these pieces.
//!
//! # Conventions
//!
//! * The grid is `n` x `n` cells with spacing dx = 1, so lengths are in cells
//!   and the diffusion constant D is in cells^2 per unit time.
//! * Cell `(i, j)` is column `i` (x, left to right) and row `j` (y, top to
//!   bottom). It is stored at index `j * n + i` (row-major).
//! * In continuous coordinates the centre of cell `(i, j)` is at
//!   `(x, y) = (i, j)`. The brush takes positions in these coordinates.

mod brush;
mod colormap;
mod diffusion;
mod grid;
mod stepping;

pub use brush::{BRUSH_CUTOFF_SIGMAS, BrushMode, add_gaussian_spot, stamp, stamp_line};
pub use colormap::viridis;
pub use diffusion::{Boundary, diffuse};
pub use grid::Grid;
pub use stepping::{MAX_FRAME_SECONDS, MAX_SUBSTEPS, R_MAX, StepPlan, plan_frame, plan_time};

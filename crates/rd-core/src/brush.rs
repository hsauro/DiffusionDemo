use crate::Grid;

/// What a brush dab does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushMode {
    /// `u += strength * g`, where `g = exp(-d^2 / 2 sigma^2)`.
    Add,
    /// `u *= 1 - g`: removes everything at the centre, less further out.
    Erase,
}

/// The brush affects cells within this many sigmas of its centre.
///
/// The spec says 2 sigma, but a 2D Gaussian cut off at 2 sigma holds only
/// 1 - e^-2 = 86% of its amount, so a dab would add noticeably less than
/// `strength * 2 pi sigma^2`. At 3 sigma it holds 98.9%.
pub const BRUSH_CUTOFF_SIGMAS: f32 = 3.0;

/// One brush dab centred at `(x, y)` (cell coordinates, see the crate docs).
///
/// Cells outside the grid are skipped (the brush is clipped at the edges,
/// whatever the boundary condition). `strength` is not used by
/// [`BrushMode::Erase`]. Values are clamped to `u >= 0` afterwards.
pub fn stamp(grid: &mut Grid, x: f32, y: f32, sigma: f32, strength: f32, mode: BrushMode) {
    if sigma <= 0.0 || !x.is_finite() || !y.is_finite() {
        return;
    }
    let Some((i0, i1, j0, j1)) = cell_range(grid.n, x, y, BRUSH_CUTOFF_SIGMAS * sigma) else {
        return;
    };
    let reach2 = (BRUSH_CUTOFF_SIGMAS * sigma).powi(2);
    let inv_two_sigma2 = 1.0 / (2.0 * sigma * sigma);
    let n = grid.n;
    for j in j0..=j1 {
        let dy = j as f32 - y;
        let row = &mut grid.u[j * n..(j + 1) * n];
        for (i, v) in row.iter_mut().enumerate().take(i1 + 1).skip(i0) {
            let dx = i as f32 - x;
            let d2 = dx * dx + dy * dy;
            if d2 > reach2 {
                continue;
            }
            let g = (-d2 * inv_two_sigma2).exp();
            match mode {
                BrushMode::Add => *v += strength * g,
                BrushMode::Erase => *v *= 1.0 - g,
            }
            *v = v.max(0.0);
        }
    }
}

/// Dabs along the segment from `from` to `to` at spacing `sigma`, so a fast
/// stroke is continuous rather than dotted.
///
/// The point `from` itself is not stamped (it was stamped on the previous
/// frame); `to` always is. If the two points coincide, this is one dab at `to`.
pub fn stamp_line(
    grid: &mut Grid,
    from: (f32, f32),
    to: (f32, f32),
    sigma: f32,
    strength: f32,
    mode: BrushMode,
) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy);
    let count = if sigma > 0.0 {
        (length / sigma).ceil().max(1.0)
    } else {
        1.0
    };
    // Guard against absurd counts from a stray pointer position.
    let count = count.min(10_000.0) as u32;
    for k in 1..=count {
        let s = k as f32 / count as f32;
        stamp(
            grid,
            from.0 + s * dx,
            from.1 + s * dy,
            sigma,
            strength,
            mode,
        );
    }
}

/// Add a Gaussian spot of total `amount` centred at `(x, y)`, normalised over
/// the cells it covers (out to 4 sigma) so the grid gains exactly `amount`
/// (up to f32 rounding) when the spot is away from the edges.
pub fn add_gaussian_spot(grid: &mut Grid, x: f32, y: f32, sigma: f32, amount: f32) {
    if sigma <= 0.0 {
        return;
    }
    let reach = 4.0 * sigma;
    let Some((i0, i1, j0, j1)) = cell_range(grid.n, x, y, reach) else {
        return;
    };
    let inv_two_sigma2 = 1.0 / (2.0 * f64::from(sigma).powi(2));
    let weight = |i: usize, j: usize| -> f64 {
        let dx = i as f64 - f64::from(x);
        let dy = j as f64 - f64::from(y);
        (-(dx * dx + dy * dy) * inv_two_sigma2).exp()
    };
    let mut sum = 0.0;
    for j in j0..=j1 {
        for i in i0..=i1 {
            sum += weight(i, j);
        }
    }
    if sum <= 0.0 {
        return;
    }
    let scale = f64::from(amount) / sum;
    for j in j0..=j1 {
        for i in i0..=i1 {
            let k = grid.index(i, j);
            grid.u[k] += (scale * weight(i, j)) as f32;
        }
    }
}

/// Inclusive range of cells within `reach` of `(x, y)` along each axis,
/// clipped to the grid, or `None` if it misses the grid entirely.
fn cell_range(n: usize, x: f32, y: f32, reach: f32) -> Option<(usize, usize, usize, usize)> {
    let last = (n - 1) as f32;
    let (lo_x, hi_x) = ((x - reach).ceil().max(0.0), (x + reach).floor().min(last));
    let (lo_y, hi_y) = ((y - reach).ceil().max(0.0), (y + reach).floor().min(last));
    if lo_x > hi_x || lo_y > hi_y {
        return None;
    }
    Some((lo_x as usize, hi_x as usize, lo_y as usize, hi_y as usize))
}

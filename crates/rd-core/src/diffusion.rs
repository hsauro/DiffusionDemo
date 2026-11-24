use crate::Grid;

/// Boundary condition, applied through a ghost cell just outside each edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Boundary {
    /// Ghost = edge cell. Nothing crosses the edge; the total is conserved.
    #[default]
    NoFlux,
    /// Ghost = cell on the opposite edge. The grid wraps around (a torus).
    Periodic,
    /// Ghost = 0. Material reaching the edge is lost.
    Absorbing,
}

impl Boundary {
    pub const ALL: [Self; 3] = [Self::NoFlux, Self::Periodic, Self::Absorbing];

    pub fn label(self) -> &'static str {
        match self {
            Self::NoFlux => "No flux",
            Self::Periodic => "Periodic",
            Self::Absorbing => "Absorbing",
        }
    }
}

/// One explicit five-point step of du/dt = D (u_xx + u_yy) with dx = 1:
///
/// `u_new = u + r (u_W + u_E + u_N + u_S - 4u)`, with `r = D dt`.
///
/// Stable for `r <= 1/4`; callers should keep `r <= 0.2` (see
/// [`crate::plan_time`]). Writes into `grid.scratch` and swaps the buffers,
/// so it does not allocate.
pub fn diffuse(grid: &mut Grid, d: f32, dt: f32, bc: Boundary) {
    let n = grid.n;
    // The arithmetic is done in f64 and rounded once to f32 per cell. Several
    // f32 roundings per cell drifted the total by about 1.1e-5 over 10^4
    // steps (more than the spec allows); one rounding per cell keeps the
    // conservation test well inside 1e-5.
    let r = f64::from(d) * f64::from(dt);
    let u = &grid.u;
    let out = &mut grid.scratch;

    // Interior: every neighbour exists, so there are no branches in the loop.
    for j in 1..n - 1 {
        let up = &u[(j - 1) * n..j * n];
        let mid = &u[j * n..(j + 1) * n];
        let down = &u[(j + 1) * n..(j + 2) * n];
        let row = &mut out[j * n..(j + 1) * n];
        for i in 1..n - 1 {
            let c = f64::from(mid[i]);
            let sum = f64::from(mid[i - 1])
                + f64::from(mid[i + 1])
                + f64::from(up[i])
                + f64::from(down[i]);
            row[i] = (c + r * (sum - 4.0 * c)) as f32;
        }
    }

    // Edges: top and bottom rows, then the left and right columns between them.
    let edge = |i: usize, j: usize| -> f32 {
        let c = f64::from(u[j * n + i]);
        let w = neighbour(u, n, i, j, -1, 0, bc);
        let e = neighbour(u, n, i, j, 1, 0, bc);
        let no = neighbour(u, n, i, j, 0, -1, bc);
        let s = neighbour(u, n, i, j, 0, 1, bc);
        (c + r * (w + e + no + s - 4.0 * c)) as f32
    };
    for i in 0..n {
        out[i] = edge(i, 0);
        out[(n - 1) * n + i] = edge(i, n - 1);
    }
    for j in 1..n - 1 {
        out[j * n] = edge(0, j);
        out[j * n + n - 1] = edge(n - 1, j);
    }

    std::mem::swap(&mut grid.u, &mut grid.scratch);
}

/// Value of the neighbour of `(i, j)` at offset `(di, dj)`, or the ghost value
/// the boundary condition gives when that neighbour is outside the grid.
#[inline]
fn neighbour(u: &[f32], n: usize, i: usize, j: usize, di: isize, dj: isize, bc: Boundary) -> f64 {
    let size = n as isize;
    let ii = i as isize + di;
    let jj = j as isize + dj;
    if (0..size).contains(&ii) && (0..size).contains(&jj) {
        return f64::from(u[jj as usize * n + ii as usize]);
    }
    let ghost = match bc {
        Boundary::NoFlux => u[j * n + i],
        Boundary::Absorbing => 0.0,
        Boundary::Periodic => {
            let wi = ii.rem_euclid(size) as usize;
            let wj = jj.rem_euclid(size) as usize;
            u[wj * n + wi]
        }
    };
    f64::from(ghost)
}

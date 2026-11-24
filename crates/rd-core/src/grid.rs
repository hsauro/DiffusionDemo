/// One species on an `n` x `n` grid.
///
/// `u` holds the concentration; `scratch` is the second buffer that the time
/// step writes into before the two are swapped, so stepping never allocates.
#[derive(Clone, Debug)]
pub struct Grid {
    pub n: usize,
    pub u: Vec<f32>,
    pub scratch: Vec<f32>,
}

impl Grid {
    /// An empty (all-zero) grid. `n` must be at least 3.
    pub fn new(n: usize) -> Self {
        assert!(n >= 3, "grid must be at least 3 x 3, got {n}");
        Self {
            n,
            u: vec![0.0; n * n],
            scratch: vec![0.0; n * n],
        }
    }

    /// Set every cell to zero.
    pub fn clear(&mut self) {
        self.u.fill(0.0);
    }

    /// Flat index of cell `(i, j)` (column `i`, row `j`).
    #[inline]
    pub fn index(&self, i: usize, j: usize) -> usize {
        j * self.n + i
    }

    /// Concentration in cell `(i, j)`.
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f32 {
        self.u[self.index(i, j)]
    }

    /// Total amount: the sum of `u` over all cells, accumulated in `f64`.
    pub fn total(&self) -> f64 {
        self.u.iter().map(|&v| f64::from(v)).sum()
    }

    /// Largest value of `u`.
    pub fn max(&self) -> f32 {
        self.u.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    /// Smallest value of `u`.
    pub fn min(&self) -> f32 {
        self.u.iter().copied().fold(f32::INFINITY, f32::min)
    }
}

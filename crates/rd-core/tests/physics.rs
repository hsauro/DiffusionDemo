//! Physics tests for the diffusion core (spec, "Unit tests in rd-core").

use rd_core::{
    Boundary, BrushMode, Grid, MAX_FRAME_SECONDS, R_MAX, add_gaussian_spot, diffuse, plan_frame,
    stamp,
};

/// Small deterministic generator so the tests need no `rand` dependency.
struct XorShift(u64);

impl XorShift {
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn random_grid(n: usize, seed: u64) -> Grid {
    let mut rng = XorShift(seed);
    let mut grid = Grid::new(n);
    for v in &mut grid.u {
        *v = rng.next_f32();
    }
    grid
}

/// Spot used by the spreading and symmetry tests: centre of a 201 x 201 grid.
fn centred_spot() -> Grid {
    let mut grid = Grid::new(201);
    add_gaussian_spot(&mut grid, 100.0, 100.0, 2.0, 25.0);
    grid
}

/// Variance of the distribution along x and along y about its mean.
fn variances(grid: &Grid) -> (f64, f64) {
    let n = grid.n;
    let (mut m, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for j in 0..n {
        for i in 0..n {
            let v = f64::from(grid.get(i, j));
            m += v;
            sx += v * i as f64;
            sy += v * j as f64;
        }
    }
    let (cx, cy) = (sx / m, sy / m);
    let (mut vx, mut vy) = (0.0, 0.0);
    for j in 0..n {
        for i in 0..n {
            let v = f64::from(grid.get(i, j));
            vx += v * (i as f64 - cx).powi(2);
            vy += v * (j as f64 - cy).powi(2);
        }
    }
    (vx / m, vy / m)
}

/// 1. Conservation with no-flux and periodic boundaries.
#[test]
fn conservation_no_flux_and_periodic() {
    for bc in [Boundary::NoFlux, Boundary::Periodic] {
        let mut grid = random_grid(64, 0x9E37_79B9_7F4A_7C15);
        let start = grid.total();
        for _ in 0..10_000 {
            diffuse(&mut grid, 1.0, 0.2, bc);
        }
        let rel = ((grid.total() - start) / start).abs();
        assert!(rel < 1e-5, "{bc:?}: relative change in total {rel:e}");
    }
}

/// 2. Absorbing boundary: the total never increases from one step to the next.
#[test]
fn absorbing_total_never_increases() {
    let mut grid = random_grid(64, 12345);
    let mut previous = grid.total();
    for step in 0..10_000 {
        diffuse(&mut grid, 1.0, 0.2, Boundary::Absorbing);
        let now = grid.total();
        assert!(
            now <= previous,
            "total rose at step {step}: {previous} -> {now}"
        );
        previous = now;
    }
}

/// 3. Spreading: variance along each axis = starting variance + 2 D t.
#[test]
fn variance_grows_as_two_d_t() {
    let d = 1.0;
    let dt = 0.2;
    let steps = 250; // t = 50
    let mut grid = centred_spot();
    let (vx0, vy0) = variances(&grid);
    for _ in 0..steps {
        diffuse(&mut grid, d, dt, Boundary::NoFlux);
    }
    let t = f64::from(dt) * steps as f64;
    let (vx, vy) = variances(&grid);
    for (axis, v0, v) in [("x", vx0, vx), ("y", vy0, vy)] {
        let want = v0 + 2.0 * f64::from(d) * t;
        let err = ((v - want) / want).abs();
        assert!(
            err < 0.01,
            "{axis}: variance {v:.3}, expected {want:.3} ({err:.2e})"
        );
    }
}

/// 4. Symmetry: the centred spot stays symmetric under x and y reflection.
#[test]
fn centred_spot_stays_symmetric() {
    let mut grid = centred_spot();
    for _ in 0..250 {
        diffuse(&mut grid, 1.0, 0.2, Boundary::NoFlux);
    }
    let n = grid.n;
    for j in 0..n {
        for i in 0..n {
            let v = grid.get(i, j);
            let rx = grid.get(n - 1 - i, j);
            let ry = grid.get(i, n - 1 - j);
            assert!(
                (v - rx).abs() < 1e-6,
                "x reflection at ({i}, {j}): {v} vs {rx}"
            );
            assert!(
                (v - ry).abs() < 1e-6,
                "y reflection at ({i}, {j}): {v} vs {ry}"
            );
        }
    }
}

/// 5. Maximum principle: with r <= 1/4, max u never increases and min u never
///    drops below 0. (The max is allowed a few f32 ulps of rounding.)
#[test]
fn maximum_principle() {
    for bc in Boundary::ALL {
        for r in [0.1_f32, 0.2, 0.25] {
            let mut grid = random_grid(48, 777);
            let mut max = grid.max();
            for step in 0..2_000 {
                diffuse(&mut grid, 1.0, r, bc);
                let new_max = grid.max();
                assert!(
                    new_max <= max * (1.0 + 4.0 * f32::EPSILON),
                    "{bc:?} r={r}: max rose at step {step}: {max} -> {new_max}"
                );
                let min = grid.min();
                assert!(min >= 0.0, "{bc:?} r={r}: min {min} < 0 at step {step}");
                max = new_max;
            }
        }
    }
}

/// 6. Brush: one Add stamp away from the edges adds strength * 2 pi sigma^2.
#[test]
fn brush_adds_expected_amount() {
    for (sigma, strength) in [(1.0_f32, 0.3_f32), (4.0, 0.3), (7.5, 1.0), (15.0, 0.05)] {
        let mut grid = Grid::new(201);
        stamp(&mut grid, 100.3, 99.6, sigma, strength, BrushMode::Add);
        let want = f64::from(strength) * 2.0 * std::f64::consts::PI * f64::from(sigma).powi(2);
        let got = grid.total();
        let err = ((got - want) / want).abs();
        assert!(
            err < 0.05,
            "sigma {sigma}: added {got:.4}, expected {want:.4} ({err:.3})"
        );
    }
}

/// 7. Substep rule: for every D and speed on the slider ranges, r <= 0.2.
#[test]
fn substep_rule_keeps_r_below_limit() {
    let log_steps = |lo: f64, hi: f64, k: usize| -> Vec<f64> {
        (0..=k)
            .map(|s| lo * (hi / lo).powf(s as f64 / k as f64))
            .collect()
    };
    let frame_times = [1e-4, 1.0 / 240.0, 1.0 / 60.0, MAX_FRAME_SECONDS, 0.5, 10.0];
    for d in log_steps(0.01, 10.0, 60) {
        for speed in log_steps(1.0, 200.0, 60) {
            for real in frame_times {
                let plan = plan_frame(d, speed, real);
                assert!(plan.substeps >= 1, "D={d} speed={speed}: no substeps");
                // r as the simulation computes it, in f32.
                let r = f64::from(d as f32 * plan.dt as f32);
                assert!(
                    r <= R_MAX * (1.0 + 1e-6),
                    "D={d} speed={speed} real={real}: r = {r}"
                );
                let asked = speed * real.min(MAX_FRAME_SECONDS);
                assert!(plan.advance <= asked * (1.0 + 1e-12), "advanced too far");
                assert_eq!(plan.limited, plan.advance < asked * (1.0 - 1e-12));
            }
        }
    }
}

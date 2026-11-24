//! How much simulated time to advance per frame, and in how many substeps.

/// Largest r = D dt / dx^2 we allow. The scheme is stable up to 1/4.
pub const R_MAX: f64 = 0.2;

/// Most substeps we will run in one frame.
pub const MAX_SUBSTEPS: u32 = 100;

/// Longest real-time interval one frame may account for, so that a tab that
/// was in the background does not jump ahead when it comes back.
pub const MAX_FRAME_SECONDS: f64 = 1.0 / 30.0;

/// What one frame should do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepPlan {
    /// Number of diffusion steps to run this frame.
    pub substeps: u32,
    /// Time step for each of them.
    pub dt: f64,
    /// Simulated time the frame advances: `substeps * dt`.
    pub advance: f64,
    /// True when the substep cap was hit, so the frame advances less
    /// simulated time than was asked for ("running slower than requested").
    pub limited: bool,
}

impl StepPlan {
    const NOTHING: Self = Self {
        substeps: 0,
        dt: 0.0,
        advance: 0.0,
        limited: false,
    };
}

/// Plan a frame from the speed (simulated time per real second) and the real
/// time since the last frame, which is capped at [`MAX_FRAME_SECONDS`].
pub fn plan_frame(d: f64, speed: f64, real_seconds: f64) -> StepPlan {
    let real = real_seconds.clamp(0.0, MAX_FRAME_SECONDS);
    plan_time(d, speed * real)
}

/// Plan the substeps needed to advance `t_frame` of simulated time with
/// diffusion constant `d`, keeping r = D dt <= [`R_MAX`]:
///
/// 1. n = ceil(D t_frame / 0.2), at least 1, at most [`MAX_SUBSTEPS`].
/// 2. dt = t_frame / n.
/// 3. If n would exceed the cap, advance only `MAX_SUBSTEPS * 0.2 / D`.
pub fn plan_time(d: f64, t_frame: f64) -> StepPlan {
    if t_frame <= 0.0 || !t_frame.is_finite() || d < 0.0 || !d.is_finite() {
        return StepPlan::NOTHING;
    }
    let needed = (d * t_frame / R_MAX).ceil();
    if needed > f64::from(MAX_SUBSTEPS) {
        let dt = R_MAX / d;
        StepPlan {
            substeps: MAX_SUBSTEPS,
            dt,
            advance: f64::from(MAX_SUBSTEPS) * dt,
            limited: true,
        }
    } else {
        let substeps = (needed as u32).max(1);
        StepPlan {
            substeps,
            dt: t_frame / f64::from(substeps),
            advance: t_frame,
            limited: false,
        }
    }
}

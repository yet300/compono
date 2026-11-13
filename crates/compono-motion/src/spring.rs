//! Springs, integrated in fixed steps.

use crate::math::sqrt;

/// The integration step, in seconds.
pub(crate) const STEP: f32 = 1.0 / 1000.0;
/// No spring is followed further than this, in seconds.
pub(crate) const LIMIT: f32 = 10.0;
/// A spring has settled once it stays this close to its target.
pub(crate) const SETTLED: f32 = 1e-3;

/// A spring from 0 to 1 with mass 1, as Compose and Material give it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    /// 1 is critically damped; below 1 overshoots.
    pub damping_ratio: f32,
    /// Stiffness, with mass 1, in 1/s².
    pub stiffness: f32,
}

impl Spring {
    /// Material 3 standard default spatial spring: damping 0.9, stiffness 700.
    /// Source: `SpringDefaultSpatialDamping` and `SpringDefaultSpatialStiffness` in
    /// `StandardMotionTokens.kt`, Compose Material 3 1.10.0-alpha05.
    pub const MATERIAL_SPATIAL: Self = Self {
        damping_ratio: 0.9,
        stiffness: 700.0,
    };
    /// A `UINavigationController` push: response 0.29 s, critically damped.
    /// Apple does not document it. Measured frame by frame on three pushes in the
    /// iOS 26 simulator, which fit responses of 0.28, 0.29 and 0.296 s. It is about
    /// twice as fast as SwiftUI's `Animation.default` (response 0.55 s).
    pub const CUPERTINO: Self = Self::from_response(0.29, 1.0);

    /// Creates a spring from SwiftUI's form: the period of the undamped spring in
    /// seconds, and the damping ratio. The stiffness is `(2π / response)²`.
    #[must_use]
    pub const fn from_response(response_secs: f32, damping_ratio: f32) -> Self {
        let omega = if response_secs > 0.0 {
            core::f32::consts::TAU / response_secs
        } else {
            0.0
        };
        Self {
            damping_ratio,
            stiffness: omega * omega,
        }
    }

    /// Returns the position and velocity `t` seconds after leaving 0 toward 1 with
    /// `velocity`, in distances per second. `t` is limited to 10 seconds.
    #[must_use]
    pub fn position(self, t: f32, velocity: f32) -> (f32, f32) {
        self.advance(0.0, velocity, t)
    }

    /// Returns the time after which the spring, leaving 0 at rest, stays within
    /// 0.1 % of its target. A spring that has not settled after 10 seconds gives 10.
    #[must_use]
    pub fn settle_time(self) -> f32 {
        let (mut x, mut v, mut t, mut last_out) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        // 10 000 steps of one millisecond, so the loop is bounded.
        for _ in 0..steps(LIMIT) {
            (x, v) = self.step(x, v, STEP);
            t += STEP;
            if (x - 1.0).abs() > SETTLED || x.is_nan() {
                last_out = t;
            }
        }
        if last_out >= LIMIT - STEP {
            LIMIT
        } else {
            last_out
        }
    }

    /// Advances `(x, v)` by `t` seconds, at most `LIMIT`, in fixed steps.
    pub(crate) fn advance(self, x: f32, v: f32, t: f32) -> (f32, f32) {
        if self.stiffness.is_nan() || self.stiffness <= 0.0 {
            return (x, v);
        }
        let t = if t > LIMIT { LIMIT } else { t };
        let whole = steps(t);
        let (mut x, mut v) = (x, v);
        for _ in 0..whole {
            (x, v) = self.step(x, v, STEP);
        }
        let rest = t - whole as f32 * STEP;
        if rest > 0.0 {
            (x, v) = self.step(x, v, rest);
        }
        (x, v)
    }

    /// Advances `(x, v)` by one fourth-order Runge-Kutta step of `dt` seconds.
    /// At one millisecond it stays within 1e-6 of the closed form.
    pub(crate) fn step(self, x: f32, v: f32, dt: f32) -> (f32, f32) {
        let damping = 2.0 * self.damping_ratio * sqrt(self.stiffness);
        let accel = |x: f32, v: f32| -self.stiffness * (x - 1.0) - damping * v;
        let half = dt / 2.0;
        let (x1, v1) = (v, accel(x, v));
        let (x2, v2) = (v + v1 * half, accel(x + x1 * half, v + v1 * half));
        let (x3, v3) = (v + v2 * half, accel(x + x2 * half, v + v2 * half));
        let (x4, v4) = (v + v3 * dt, accel(x + x3 * dt, v + v3 * dt));
        (
            x + dt / 6.0 * (x1 + 2.0 * x2 + 2.0 * x3 + x4),
            v + dt / 6.0 * (v1 + 2.0 * v2 + 2.0 * v3 + v4),
        )
    }
}

/// The number of whole steps in `t` seconds, for `t` in `0..=LIMIT`.
fn steps(t: f32) -> u32 {
    if t > 0.0 {
        // Truncation is intended; `t` is at most `LIMIT`, so this fits.
        (t / STEP) as u32
    } else {
        0
    }
}

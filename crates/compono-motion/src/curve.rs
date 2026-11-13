//! Easing curves.

use crate::math::clamp01;

/// A curve from linear time to eased progress, both `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Linear,
    /// A CSS cubic Bézier through (0, 0), (x1, y1), (x2, y2) and (1, 1).
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    /// Material 3 standard easing. Source: `EasingStandardCubicBezier` in
    /// `MotionTokens.kt`, Compose Material 3 1.10.0-alpha05.
    pub const STANDARD: Self = Self::CubicBezier(0.2, 0.0, 0.0, 1.0);
    /// Material 3 emphasized decelerate easing. Source:
    /// `EasingEmphasizedDecelerateCubicBezier` in `MotionTokens.kt`.
    pub const EMPHASIZED_DECELERATE: Self = Self::CubicBezier(0.05, 0.7, 0.1, 1.0);
    /// Material 3 standard decelerate easing, which the Android predictive Back
    /// guide applies to gesture progress. Source:
    /// `EasingStandardDecelerateCubicBezier` in `MotionTokens.kt`.
    pub const STANDARD_DECELERATE: Self = Self::CubicBezier(0.0, 0.0, 0.0, 1.0);
    /// The easing of Compose's `tween()`. Source: `FastOutSlowInEasing`,
    /// `Easing.kt:50`, `androidx.compose.animation:animation-core` 1.12.0.
    pub const FAST_OUT_SLOW_IN: Self = Self::CubicBezier(0.4, 0.0, 0.2, 1.0);

    /// Returns eased progress at linear time `t`, which is clamped to `0..=1`.
    #[must_use]
    pub fn ease(self, t: f32) -> f32 {
        let t = clamp01(t);
        match self {
            Self::Linear => t,
            Self::CubicBezier(x1, y1, x2, y2) => bezier(y1, y2, solve(x1, x2, t)),
        }
    }
}

/// One coordinate of the curve at parameter `s`, with end points 0 and 1.
fn bezier(p1: f32, p2: f32, s: f32) -> f32 {
    let u = 1.0 - s;
    3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s
}

fn bezier_slope(p1: f32, p2: f32, s: f32) -> f32 {
    let u = 1.0 - s;
    3.0 * u * u * p1 + 6.0 * u * s * (p2 - p1) + 3.0 * s * s * (1.0 - p2)
}

/// Finds the parameter whose x is `x`: Newton's method, then bisection when Newton
/// stalls on a flat slope. Both are bounded.
fn solve(x1: f32, x2: f32, x: f32) -> f32 {
    const TOLERANCE: f32 = 1e-6;
    let mut s = x;
    for _ in 0..8 {
        let error = bezier(x1, x2, s) - x;
        if error.abs() < TOLERANCE {
            return s;
        }
        let slope = bezier_slope(x1, x2, s);
        if slope.abs() < TOLERANCE {
            break;
        }
        s = clamp01(s - error / slope);
    }
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    s = x;
    for _ in 0..32 {
        let value = bezier(x1, x2, s);
        if (value - x).abs() < TOLERANCE {
            break;
        }
        if value < x {
            low = s;
        } else {
            high = s;
        }
        s = 0.5 * (low + high);
    }
    s
}

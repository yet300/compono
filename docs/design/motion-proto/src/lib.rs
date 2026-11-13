//! Prototype of a shared motion model: data and pure functions, no std, no deps.
#![no_std]

/// A curve from linear time to eased progress, both 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Linear,
    /// CSS-style cubic Bézier with end points (0,0) and (1,1).
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    pub const STANDARD: Self = Self::CubicBezier(0.2, 0.0, 0.0, 1.0); // M3 EasingStandard
    pub const EMPHASIZED_DECELERATE: Self = Self::CubicBezier(0.05, 0.7, 0.1, 1.0);
    pub const STANDARD_DECELERATE: Self = Self::CubicBezier(0.0, 0.0, 0.0, 1.0);
    pub const FAST_OUT_SLOW_IN: Self = Self::CubicBezier(0.4, 0.0, 0.2, 1.0); // Compose tween() default

    pub fn ease(self, t: f32) -> f32 {
        let t = clamp01(t);
        match self {
            Self::Linear => t,
            Self::CubicBezier(x1, y1, x2, y2) => {
                let s = solve_x(x1, x2, t);
                bezier(y1, y2, s)
            }
        }
    }
}

fn bezier(p1: f32, p2: f32, s: f32) -> f32 {
    let u = 1.0 - s;
    3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s
}

fn bezier_dx(p1: f32, p2: f32, s: f32) -> f32 {
    let u = 1.0 - s;
    3.0 * u * u * p1 + 6.0 * u * s * (p2 - p1) + 3.0 * s * s * (1.0 - p2)
}

/// Finds s with x(s) = x: Newton steps, then bisection if Newton stalls.
fn solve_x(x1: f32, x2: f32, x: f32) -> f32 {
    let mut s = x;
    for _ in 0..8 {
        let err = bezier(x1, x2, s) - x;
        if err.abs() < 1e-6 {
            return s;
        }
        let d = bezier_dx(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        s -= err / d;
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    s = x;
    for _ in 0..32 {
        let v = bezier(x1, x2, s);
        if (v - x).abs() < 1e-6 {
            break;
        }
        if v < x { lo = s } else { hi = s }
        s = 0.5 * (lo + hi);
    }
    s
}

/// A spring from 0 to 1, as Compose and Material specify it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub damping_ratio: f32,
    pub stiffness: f32,
}

impl Spring {
    pub const M3_STANDARD_DEFAULT_SPATIAL: Self = Self { damping_ratio: 0.9, stiffness: 700.0 };
    pub const M3_STANDARD_FAST_SPATIAL: Self = Self { damping_ratio: 0.9, stiffness: 1400.0 };
    /// SwiftUI `Animation.default`: response 0.55, dampingFraction 1.0.
    /// stiffness = (2π / response)², mass 1.
    pub const SWIFTUI_DEFAULT: Self = Self { damping_ratio: 1.0, stiffness: 130.5 };

    /// Position at `t` seconds, starting at 0 with `velocity` (units of the 0..1
    /// distance per second). Fixed-step semi-implicit Euler: core has no exp or sin.
    pub fn position(self, t: f32, velocity: f32) -> (f32, f32) {
        const DT: f32 = 1.0 / 1000.0;
        let w0 = sqrt(self.stiffness);
        let c = 2.0 * self.damping_ratio * w0;
        let (mut x, mut v) = (0.0f32, velocity);
        let mut left = t.max(0.0);
        while left > 0.0 {
            let dt = if left < DT { left } else { DT };
            let a = -self.stiffness * (x - 1.0) - c * v;
            v += a * dt;
            x += v * dt;
            left -= dt;
        }
        (x, v)
    }

    /// The time after which the spring stays within 0.1% of its target.
    pub fn settle_time(self) -> f32 {
        const DT: f32 = 1.0 / 1000.0;
        let w0 = sqrt(self.stiffness);
        let c = 2.0 * self.damping_ratio * w0;
        let (mut x, mut v, mut t, mut last_out) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        while t < 10.0 {
            let a = -self.stiffness * (x - 1.0) - c * v;
            v += a * DT;
            x += v * DT;
            t += DT;
            if (x - 1.0).abs() > 0.001 { last_out = t }
        }
        last_out
    }
}

fn sqrt(x: f32) -> f32 {
    // Newton's method; enough for stiffness values.
    if x <= 0.0 { return 0.0 }
    let mut r = if x > 1.0 { x * 0.5 } else { 1.0 };
    for _ in 0..30 { r = 0.5 * (r + x / r) }
    r
}

fn clamp01(t: f32) -> f32 { if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t } }
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// How long and with which curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Timing {
    Tween { millis: u16, easing: Easing },
    Spring(Spring),
}

impl Timing {
    /// Eased progress after `elapsed` seconds, and whether it is finished.
    pub fn progress(self, elapsed: f32) -> (f32, bool) {
        match self {
            Self::Tween { millis, easing } => {
                let d = f32::from(millis) / 1000.0;
                if d <= 0.0 { return (1.0, true) }
                (easing.ease(elapsed / d), elapsed >= d)
            }
            Self::Spring(s) => {
                let (x, _) = s.position(elapsed, 0.0);
                (x, elapsed >= s.settle_time())
            }
        }
    }
}

/// What a UI applies to one drawn entry. Offsets are shares of the router's size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerStyle {
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale: f32,
    pub opacity: f32,
    /// Logical pixels.
    pub corner_radius: f32,
    /// Black scrim drawn over the layer, 0..=1.
    pub dim: f32,
}

impl LayerStyle {
    pub const REST: Self = Self { offset_x: 0.0, offset_y: 0.0, scale: 1.0, opacity: 1.0, corner_radius: 0.0, dim: 0.0 };
}

/// The two entries a stack draws during a transition, bottom first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StackFrame {
    pub under: LayerStyle,
    pub over: LayerStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { Forward, Backward }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StackTransition {
    /// The new entry slides over; the covered one moves `parallax` of the width.
    Slide { parallax: f32, dim: f32 },
    Fade,
    /// Decompose's `scale()`: front from `front` to 1, back from 1 to `back`.
    Scale { front: f32, back: f32 },
    /// Material shared axis X: 30 dp shift approximated as a share, fade through.
    SharedAxisX { shift: f32 },
    None,
}

impl StackTransition {
    /// `t` is eased progress 0..=1. Forward: `over` enters. Backward: `over` leaves.
    pub fn frame(self, direction: Direction, t: f32) -> StackFrame {
        // Express forward motion with p = how far the upper entry is "in".
        let p = match direction { Direction::Forward => t, Direction::Backward => 1.0 - t };
        let mut under = LayerStyle::REST;
        let mut over = LayerStyle::REST;
        match self {
            Self::Slide { parallax, dim } => {
                over.offset_x = 1.0 - p;
                under.offset_x = -parallax * p;
                under.dim = dim * p;
            }
            Self::Fade => {
                over.opacity = p;
                under.opacity = 1.0 - p;
            }
            Self::Scale { front, back } => {
                over.scale = lerp(front, 1.0, p);
                over.opacity = p;
                under.scale = lerp(1.0, back, p);
            }
            Self::SharedAxisX { shift } => {
                over.offset_x = shift * (1.0 - p);
                under.offset_x = -shift * p;
                // Fade through: outgoing fades in the first 35%, incoming after.
                over.opacity = clamp01((p - 0.35) / 0.65);
                under.opacity = clamp01(1.0 - p / 0.35);
            }
            Self::None => {
                over.opacity = if p > 0.0 { 1.0 } else { 0.0 };
            }
        }
        StackFrame { under, over }
    }
}

/// Where a predictive Back gesture started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge { Left, Right }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackGestureStyle {
    /// iOS interactive pop: the top entry follows the finger, the one under it
    /// moves from -parallax to 0.
    Swipe { parallax: f32, dim: f32 },
    /// Android in-app predictive Back, from the Android design guide:
    /// exit scale 1 → 0.9, 8 dp from the edge, fade-through at 35%.
    Predictive { corner_radius: f32 },
}

impl BackGestureStyle {
    /// `progress` is the gesture's 0..=1; `width_dp` lets dp margins become shares.
    pub fn frame(self, progress: f32, edge: Edge, width_dp: f32) -> StackFrame {
        let p = clamp01(progress);
        let mut under = LayerStyle::REST;
        let mut over = LayerStyle::REST;
        match self {
            Self::Swipe { parallax, dim } => {
                over.offset_x = p;
                under.offset_x = -parallax * (1.0 - p);
                under.dim = dim * (1.0 - p);
            }
            Self::Predictive { corner_radius } => {
                let scale = lerp(1.0, 0.9, p);
                over.scale = scale;
                over.corner_radius = corner_radius * p;
                // Scaled about the centre, then shifted toward the far edge so
                // 8 dp stays between the surface and the swipe edge.
                let gap = if width_dp > 0.0 { 8.0 / width_dp } else { 0.0 };
                let toward = (1.0 - scale) / 2.0 - gap * p;
                over.offset_x = match edge { Edge::Left => toward, Edge::Right => -toward };
                under.scale = lerp(1.1, 1.0, p);
                over.opacity = clamp01(1.0 - p / 0.35);
                under.opacity = clamp01((p - 0.35) / 0.65);
            }
        }
        StackFrame { under, over }
    }
}

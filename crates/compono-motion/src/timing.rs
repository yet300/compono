//! How long a transition takes and along which curve.

use crate::curve::Easing;
use crate::spring::Spring;

/// How long a transition takes and along which curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Timing {
    /// A fixed duration along an easing curve.
    Tween { millis: u16, easing: Easing },
    /// A spring from 0 to 1, finished when it settles within 0.1 %.
    Spring(Spring),
}

impl Timing {
    /// Returns eased progress `elapsed` seconds after the start, and whether the
    /// transition has finished. A spring is integrated from the start, so an
    /// adapter that calls this every frame should use `TransitionState` instead.
    #[must_use]
    pub fn progress(self, elapsed: f32) -> (f32, bool) {
        match self {
            Self::Tween { millis, easing } => {
                let duration = f32::from(millis) / 1000.0;
                if duration <= 0.0 {
                    return (1.0, true);
                }
                if elapsed.is_nan() || elapsed <= 0.0 {
                    return (0.0, false);
                }
                (easing.ease(elapsed / duration), elapsed >= duration)
            }
            Self::Spring(spring) => {
                if elapsed.is_nan() || elapsed <= 0.0 {
                    return (0.0, false);
                }
                let (x, _) = spring.position(elapsed, 0.0);
                if elapsed >= spring.settle_time() {
                    (1.0, true)
                } else {
                    (x, false)
                }
            }
        }
    }
}

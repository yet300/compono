//! The presets of each platform.

use crate::back::BackAnimation;
use crate::curve::Easing;
use crate::spring::Spring;
use crate::timing::Timing;
use crate::transition::Transition;

/// The duration of the fade that reduced motion uses, in milliseconds.
const REDUCED_MILLIS: u16 = 150;

/// What the user asked of motion, which the adapter reads from the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MotionPreference {
    #[default]
    Full,
    /// Keep orientation, drop travel: spatial transitions become a 150 ms fade, and
    /// a Back gesture only fades the top entry.
    Reduced,
    /// No motion at all.
    Off,
}

/// The motion a router uses: its transition, timing and Back gesture.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub transition: Transition,
    pub timing: Timing,
    pub back: BackAnimation,
}

impl Motion {
    /// Android's Material 3 look: shared axis X over 30 dp on the standard spatial
    /// spring, and the Material predictive Back with 16 dp corners.
    ///
    /// Sources: the shared axis shift of 30 dp is Material's `MaterialSharedAxis`;
    /// the spring is `Spring::MATERIAL_SPATIAL`; the 16 dp corner radius is
    /// Decompose's `LayoutCorners.kt:20`, because the Android guide does not state one.
    #[must_use]
    pub const fn material() -> Self {
        Self {
            transition: Transition::SharedAxisX { shift_dp: 30.0 },
            timing: Timing::Spring(Spring::MATERIAL_SPATIAL),
            back: BackAnimation::Predictive {
                corner_radius: 16.0,
            },
        }
    }

    /// The iOS look: the new screen slides over, the covered one moves 30 % to the
    /// left under a 12 % shade, on UIKit's push spring, with the swipe-back.
    ///
    /// Sources: all values are observed, not documented by Apple: measured frame
    /// by frame on `UINavigationController` pushes in the iOS 26 simulator. The
    /// covered screen moved 0.300 of the width per unit of progress and darkened by
    /// 0.12, and the timing is `Spring::CUPERTINO`.
    #[must_use]
    pub const fn cupertino() -> Self {
        // Observed on UIKit, not documented: parallax 0.3 and shade 0.12.
        Self {
            transition: Transition::Slide {
                parallax: 0.3,
                dim: 0.12,
            },
            timing: Timing::Spring(Spring::CUPERTINO),
            back: BackAnimation::Swipe {
                parallax: 0.3,
                dim: 0.12,
            },
        }
    }

    /// The desktop look for egui, iced, GPUI and macOS: a 150 ms fade on the
    /// Material standard easing, with no Back gesture.
    ///
    /// Source: the design's recommendation; desktop systems state no navigation
    /// motion.
    #[must_use]
    pub const fn desktop() -> Self {
        Self {
            transition: Transition::Fade,
            timing: Timing::Tween {
                millis: 150,
                easing: Easing::STANDARD,
            },
            back: BackAnimation::None,
        }
    }

    /// The web look: a 250 ms fade on CSS `ease`, with no Back gesture.
    ///
    /// Source: the user-agent stylesheet of CSS View Transitions, 0.25 s.
    #[must_use]
    pub const fn web() -> Self {
        Self {
            transition: Transition::Fade,
            // CSS `ease`, the default timing function of the stylesheet.
            timing: Timing::Tween {
                millis: 250,
                easing: Easing::CubicBezier(0.25, 0.1, 0.25, 1.0),
            },
            back: BackAnimation::None,
        }
    }

    /// No motion.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            transition: Transition::None,
            timing: Timing::Tween {
                millis: 0,
                easing: Easing::Linear,
            },
            back: BackAnimation::None,
        }
    }

    /// Returns this preset with another timing.
    #[must_use]
    pub const fn with_timing(mut self, timing: Timing) -> Self {
        self.timing = timing;
        self
    }

    /// Returns this preset with another transition.
    #[must_use]
    pub const fn with_transition(mut self, transition: Transition) -> Self {
        self.transition = transition;
        self
    }

    /// Returns this preset changed for `preference`.
    ///
    /// `Full` keeps it. `Reduced` turns a transition that moves or scales entries
    /// into a 150 ms fade on the Material standard easing, keeps a fade and its
    /// timing, and makes a Back gesture fade the top entry only. `Off` is
    /// `Motion::none()`.
    #[must_use]
    pub const fn for_preference(self, preference: MotionPreference) -> Self {
        match preference {
            MotionPreference::Full => self,
            MotionPreference::Reduced => {
                let mut reduced = self;
                if self.transition.is_spatial() {
                    reduced.transition = Transition::Fade;
                    reduced.timing = Timing::Tween {
                        millis: REDUCED_MILLIS,
                        easing: Easing::STANDARD,
                    };
                }
                if !matches!(self.back, BackAnimation::None) {
                    reduced.back = BackAnimation::Fade;
                }
                reduced
            }
            MotionPreference::Off => Self::none(),
        }
    }
}

//! How the entries of a router move during a transition.

use crate::math::{clamp01, lerp};
use crate::style::{Direction, LayerStyle, Size};

/// The share of a fade-through in which the leaving entry fades out and after which
/// the entering one fades in. Source: Material's fade through, `FadeThroughProvider`
/// in Material Components for Android, threshold 0.35.
const FADE_THROUGH: f32 = 0.35;

/// How the entries of a router move during a transition.
#[derive(Clone, Copy, Debug)]
pub enum Transition {
    /// The front entry slides in from the right edge and out to it. The back entry
    /// moves `parallax` of the width to the left and is dimmed by up to `dim`.
    Slide { parallax: f32, dim: f32 },
    /// Entering entries fade in and leaving ones fade out.
    Fade,
    /// The front entry scales from `front` to 1 and fades in; the back entry scales
    /// from 1 to `back`. Decompose's `fade() + scale()` for the front entry.
    Scale { front: f32, back: f32 },
    /// Material's shared axis X: both entries shift by `shift_dp` logical pixels
    /// and fade through, the leaving one out by 35 % and the entering one in after.
    SharedAxisX { shift_dp: f32 },
    /// The front entry slides up from the bottom edge and back down: a sheet.
    SlideUp,
    /// Nothing moves: entering entries are at rest and leaving ones hidden.
    None,
    /// An application's own formula, given the role and the eased progress.
    Custom(fn(Direction, f32) -> LayerStyle),
}

impl Transition {
    /// Returns the style of an entry in `direction` at eased `progress`, where 0 is
    /// the start of the transition and 1 its end. An entering entry is at rest at
    /// 1, a leaving one at 0. A spring may take `progress` slightly past 1.
    #[must_use]
    pub fn layer(self, direction: Direction, progress: f32, size: Size) -> LayerStyle {
        // How far the entry is from rest: 0 at rest, 1 fully away.
        let away = if direction.is_enter() {
            1.0 - progress
        } else {
            progress
        };
        let mut style = LayerStyle::REST;
        match self {
            Self::Slide { parallax, dim } => {
                if direction.is_front() {
                    style.offset_x = away;
                } else {
                    style.offset_x = -parallax * away;
                    style.dim = clamp01(dim * away);
                }
            }
            Self::Fade => style.opacity = clamp01(1.0 - away),
            Self::Scale { front, back } => {
                if direction.is_front() {
                    style.scale = lerp(1.0, front, away);
                    style.opacity = clamp01(1.0 - away);
                } else {
                    style.scale = lerp(1.0, back, away);
                }
            }
            Self::SharedAxisX { shift_dp } => {
                let shift = if size.width > 0.0 {
                    shift_dp / size.width
                } else {
                    0.0
                };
                // The front entry sits to the right of rest, the back one to the left.
                style.offset_x = if direction.is_front() { shift } else { -shift } * away;
                style.opacity = if direction.is_enter() {
                    clamp01((progress - FADE_THROUGH) / (1.0 - FADE_THROUGH))
                } else {
                    clamp01(1.0 - progress / FADE_THROUGH)
                };
            }
            Self::SlideUp => {
                if direction.is_front() {
                    style.offset_y = away;
                }
            }
            Self::None => {
                if !direction.is_enter() {
                    style.opacity = 0.0;
                }
            }
            Self::Custom(layer) => style = layer(direction, progress),
        }
        style
    }

    /// Whether entries change place or size, which reduced motion removes.
    #[must_use]
    pub const fn is_spatial(self) -> bool {
        !matches!(self, Self::Fade | Self::None)
    }
}

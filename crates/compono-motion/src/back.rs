//! How a stack follows a Back gesture.

use crate::math::{clamp01, lerp};
use crate::style::{LayerStyle, Size};

/// The smallest scale of the top entry. Source: Android predictive Back design
/// guide, "Exit scale 100 % → 90 %".
const PREDICTIVE_SCALE: f32 = 0.9;
/// The gap between the shrunk entry and the screen edge, in dp. Source: Android
/// predictive Back design guide, 8 dp.
const PREDICTIVE_GAP: f32 = 8.0;
/// The scrim over the revealed entry. Source: Decompose,
/// `MaterialPredictiveBackAnimatable.kt:77`, black at 25 %.
const PREDICTIVE_SCRIM: f32 = 0.25;

/// Where a Back gesture started, as AndroidX `BackEvent` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwipeEdge {
    Left,
    Right,
    /// Not a swipe, such as the three-button Back.
    None,
}

/// One moment of a Back gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackEvent {
    /// How far the gesture has gone, `0..=1`.
    pub progress: f32,
    /// The edge the gesture started from.
    pub edge: SwipeEdge,
    /// How far the touch has moved down since the gesture started, in logical
    /// pixels. Negative is up.
    pub touch_y: f32,
}

/// The two entries a stack draws during a Back gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StackFrame {
    /// The entry Back reveals, drawn first.
    pub under: LayerStyle,
    /// The entry Back removes, drawn above.
    pub over: LayerStyle,
}

/// How a stack follows a Back gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackAnimation {
    /// iOS interactive pop: the top entry follows the finger to the right, and the
    /// one under it moves from `-parallax` of the width to 0, dimmed by up to `dim`.
    Swipe { parallax: f32, dim: f32 },
    /// Android's in-app predictive Back in the Material style: the top entry shrinks
    /// to 90 %, moves toward the far edge, follows the touch vertically and rounds
    /// its corners up to `corner_radius`; the entry under it is dimmed by 25 %.
    Predictive { corner_radius: f32 },
    /// The top entry fades out with the gesture and nothing moves: reduced motion.
    Fade,
    /// Nothing follows the gesture.
    None,
}

impl BackAnimation {
    /// Returns both layers during a gesture at `event`. `finish` runs from 0 to 1
    /// after the gesture commits, while the UI animates the top entry away.
    ///
    /// `event.progress` is used as it is: Android already eases it, and an iOS
    /// swipe follows the finger.
    #[must_use]
    pub fn frame(self, event: BackEvent, finish: f32, size: Size) -> StackFrame {
        let progress = clamp01(event.progress);
        let finish = clamp01(finish);
        // Swipe and fade carry on from where the finger let go.
        let travel = progress + (1.0 - progress) * finish;
        let mut under = LayerStyle::REST;
        let mut over = LayerStyle::REST;
        match self {
            Self::Swipe { parallax, dim } => {
                over.offset_x = travel;
                under.offset_x = -parallax * (1.0 - travel);
                under.dim = clamp01(dim * (1.0 - travel));
            }
            Self::Predictive { corner_radius } => {
                over.scale = lerp(1.0, PREDICTIVE_SCALE, progress);
                over.corner_radius = corner_radius * progress;
                over.opacity = 1.0 - finish;
                // Scaled about the centre, then moved toward the far edge until the
                // gap there is 8 dp.
                let gap = if size.width > 0.0 {
                    PREDICTIVE_GAP / size.width
                } else {
                    0.0
                };
                let toward = (1.0 - over.scale) / 2.0 - gap * progress;
                over.offset_x = match event.edge {
                    SwipeEdge::Left => toward,
                    SwipeEdge::Right => -toward,
                    SwipeEdge::None => 0.0,
                };
                if size.height > 0.0 {
                    let limit = (size.height / 20.0 - PREDICTIVE_GAP).max(0.0);
                    let touch = event.touch_y.clamp(-limit, limit);
                    let grow = (3.0 * progress).min(1.0);
                    over.offset_y = touch * grow / size.height;
                }
                under.dim = PREDICTIVE_SCRIM * (1.0 - finish);
            }
            Self::Fade => over.opacity = 1.0 - travel,
            Self::None => {}
        }
        StackFrame { under, over }
    }
}

//! The running transition of one router.

use crate::change::StackChange;
use crate::preset::Motion;
use crate::style::{Direction, LayerStyle, Size};
use crate::timing::Timing;
use crate::transition::Transition;

/// One entry to draw: its key, its role and its style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub key: u64,
    pub direction: Direction,
    pub style: LayerStyle,
}

/// The entries to draw at one moment, bottom first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The back entry, while a transition runs.
    pub under: Option<Layer>,
    /// The front entry.
    pub over: Layer,
}

impl Frame {
    /// Returns the layers bottom first.
    pub fn layers(&self) -> impl Iterator<Item = Layer> {
        self.under.into_iter().chain(core::iter::once(self.over))
    }
}

/// A transition in progress.
#[derive(Clone, Copy, Debug)]
struct Running {
    from: u64,
    backward: bool,
    motion: Motion,
    start: f64,
    /// When a spring settles, in seconds after the start.
    settle: f32,
    /// A spring's position and velocity at `integrated` seconds after the start.
    x: f32,
    v: f32,
    integrated: f32,
}

/// The running transition of one router. An adapter keeps one per router.
///
/// It holds no clock: every call takes the time as seconds from any fixed origin,
/// such as egui's `input.time`. A spring is integrated from the previous frame, so
/// a frame costs a few steps of one millisecond. Nothing here allocates.
#[derive(Clone, Copy, Debug)]
pub struct TransitionState {
    active: u64,
    size: Size,
    running: Option<Running>,
}

impl Default for TransitionState {
    /// Rests on key 0, the first key of a `Stack`.
    fn default() -> Self {
        Self::new(0)
    }
}

impl TransitionState {
    /// Creates a state resting on the entry `active`.
    #[must_use]
    pub const fn new(active: u64) -> Self {
        Self {
            active,
            size: Size::new(0.0, 0.0),
            running: None,
        }
    }

    /// Starts the transition `change` at `now_secs`.
    ///
    /// A change while another runs starts again from the new pair.
    /// `StackChange::None` changes nothing, and a motion whose transition is
    /// `Transition::None` or whose tween lasts 0 ms switches at once.
    pub fn start(&mut self, change: StackChange, motion: Motion, now_secs: f64) {
        let (from, to, backward) = match change {
            StackChange::None => return,
            StackChange::Forward { from, to } => (from, to, false),
            StackChange::Backward { from, to } => (from, to, true),
        };
        self.active = to;
        let instant = matches!(motion.transition, Transition::None)
            || matches!(motion.timing, Timing::Tween { millis: 0, .. });
        if instant {
            self.running = None;
            return;
        }
        let settle = match motion.timing {
            Timing::Spring(spring) => spring.settle_time(),
            Timing::Tween { .. } => 0.0,
        };
        self.running = Some(Running {
            from,
            backward,
            motion,
            start: now_secs,
            settle,
            x: 0.0,
            v: 0.0,
            integrated: 0.0,
        });
    }

    /// Sets the size of the router, which `Transition::SharedAxisX` needs.
    pub const fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    /// Returns the layers to draw at `now_secs`, and ends the transition once its
    /// timing has finished.
    pub fn frame(&mut self, now_secs: f64) -> Frame {
        let rest = Frame {
            under: None,
            over: Layer {
                key: self.active,
                direction: Direction::EnterFront,
                style: LayerStyle::REST,
            },
        };
        let Some(running) = self.running.as_mut() else {
            return rest;
        };
        // Seconds since the start as `f32`: transitions last seconds, not years.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a transition lasts seconds"
        )]
        let elapsed = (now_secs - running.start) as f32;
        let elapsed = if elapsed > 0.0 { elapsed } else { 0.0 };
        let (progress, finished) = match running.motion.timing {
            Timing::Tween { .. } => running.motion.timing.progress(elapsed),
            Timing::Spring(spring) => {
                if elapsed > running.integrated {
                    (running.x, running.v) =
                        spring.advance(running.x, running.v, elapsed - running.integrated);
                    running.integrated = elapsed;
                }
                (running.x, elapsed >= running.settle)
            }
        };
        if finished {
            self.running = None;
            return rest;
        }
        let transition = running.motion.transition;
        let (under, over) = if running.backward {
            (
                (self.active, Direction::EnterBack),
                (running.from, Direction::ExitFront),
            )
        } else {
            (
                (running.from, Direction::ExitBack),
                (self.active, Direction::EnterFront),
            )
        };
        let layer = |(key, direction): (u64, Direction)| Layer {
            key,
            direction,
            style: transition.layer(direction, progress, self.size),
        };
        Frame {
            under: Some(layer(under)),
            over: layer(over),
        }
    }

    /// Whether a transition is running, so the UI must draw another frame.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// The key of the entry that is, or is becoming, active.
    #[must_use]
    pub const fn active(&self) -> u64 {
        self.active
    }
}

//! Router transitions for Compono UIs, as plain values and pure functions.
//!
//! Every UI of a Compono application moves between router entries with the same
//! language: which layers move, by how much, and along which curve. This crate holds
//! that language once. A Rust UI adapter keeps one [`TransitionState`] per router,
//! starts it when [`StackChange::between`] reports a change, and applies the
//! [`LayerStyle`] of each layer that [`TransitionState::frame`] returns. The Swift
//! package and the Compose library use their own animation engines, and their tests
//! compare their values with the golden table in `golden/motion.csv`, which the
//! `golden` example writes.
//!
//! ```
//! use compono_motion::{Direction, Motion, StackChange, TransitionState};
//!
//! // Keys of a stack's entries, oldest first, before and after a push.
//! let before = [0];
//! let change = StackChange::between(&before, 0, [0, 1].into_iter(), 1);
//! assert_eq!(change, StackChange::Forward { from: 0, to: 1 });
//!
//! let mut state = TransitionState::new(0);
//! state.start(change, Motion::desktop(), 10.0);
//! let frame = state.frame(10.075);
//! assert_eq!(frame.over.key, 1);
//! assert_eq!(frame.over.direction, Direction::EnterFront);
//! assert!(frame.over.style.opacity > 0.0 && frame.over.style.opacity < 1.0);
//! assert!(state.is_running());
//! assert!(!{ state.frame(11.0); state.is_running() });
//! ```
//!
//! The crate is `no_std`, has no dependencies and never allocates. `core` has no
//! `exp` or `sin`, so springs are integrated with fourth-order Runge-Kutta in fixed
//! steps of one millisecond, within 1e-6 of the closed form.
#![no_std]
#![forbid(unsafe_code)]

mod back;
mod change;
mod curve;
mod math;
mod preset;
mod spring;
mod state;
mod style;
mod timing;
mod transition;

pub use back::{BackAnimation, BackEvent, StackFrame, SwipeEdge};
pub use change::StackChange;
pub use curve::Easing;
pub use preset::{Motion, MotionPreference};
pub use spring::Spring;
pub use state::{Frame, Layer, TransitionState};
pub use style::{Direction, LayerStyle, Size};
pub use timing::Timing;
pub use transition::Transition;

mod context;
mod handle;
pub(crate) mod node;
mod phase;

pub use context::Cx;
pub use handle::{Handle, NodeId, NotOwnChild, StaleHandle};
pub use phase::Phase;

/// A unit of UI-independent logic held by a [`Tree`](crate::Tree).
pub trait Component: Sized + 'static {
    /// Inputs from the UI, the parent and finished tasks.
    type Msg: 'static;
    /// Events for the parent, which replace callbacks passed to a constructor.
    type Output: 'static;

    /// Handles one message to completion.
    fn update(&mut self, msg: Self::Msg, cx: &mut Cx<'_, Self>);

    /// Called when the effective phase changes, once per change.
    fn phase_changed(&mut self, _phase: Phase, cx: &mut Cx<'_, Self>) {
        cx.unchanged();
    }

    /// Returns the message to run on Back, or `None` to let an ancestor handle it.
    ///
    /// This is declarative because platforms must know whether Back is handled
    /// before the gesture starts.
    fn back(&self) -> Option<Self::Msg> {
        None
    }
}

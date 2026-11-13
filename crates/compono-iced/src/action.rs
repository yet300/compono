use compono::{Component, Handle, Tree, WakeSignal};
use iced::Subscription;
use iced::futures::stream;
use std::fmt;
use std::sync::Arc;

/// Something a view or subscription wants done to the tree.
#[derive(Clone)]
#[non_exhaustive]
pub enum Action {
    /// Delivers messages queued from other threads.
    Pump,
    /// Presses Back.
    Back,
    /// Sends one message to one component.
    Send(Arc<dyn Fn(&mut Tree) + Send + Sync>),
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pump => "Pump",
            Self::Back => "Back",
            Self::Send(_) => "Send",
        })
    }
}

impl Action {
    /// Carries out the action.
    pub fn apply(&self, tree: &mut Tree) {
        match self {
            // A poisoned queue only happens after a panic on another thread.
            Self::Pump => {
                let _closed = tree.pump();
            }
            Self::Back => {
                let _handled = tree.back();
            }
            Self::Send(send) => send(tree),
        }
    }
}

/// Creates an action that sends `msg` to `target`.
pub fn send<C>(target: Handle<C>, msg: C::Msg) -> Action
where
    C: Component,
    C::Msg: Clone + Send + Sync,
{
    Action::Send(Arc::new(move |tree| {
        // A view can outlive the component it was built from by one message.
        let _stale_is_fine = tree.send(target, msg.clone());
    }))
}

/// Produces [`Action::Pump`] whenever other threads have queued work.
pub fn wakes(signal: &WakeSignal) -> Subscription<Action> {
    fn listen(signal: &WakeSignal) -> impl stream::Stream<Item = Action> + use<> {
        let signal = signal.clone();
        iced::futures::stream::poll_fn(move |context| {
            signal.poll_wake(context).map(|()| Some(Action::Pump))
        })
    }
    Subscription::run_with(signal.clone(), listen)
}

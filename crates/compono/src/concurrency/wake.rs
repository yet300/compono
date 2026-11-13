use super::sync::{Mutex, MutexGuard};
use alloc::sync::Arc;
use alloc::task::Wake;
use core::future::{Future, poll_fn};
use core::hash::{Hash, Hasher};
use core::task::{Context, Poll, Waker};
use std::sync::PoisonError;

/// A wake-up and its listener change together under one lock, so no wake-up can
/// fall between checking for one and registering for the next.
#[derive(Debug, Default)]
struct State {
    pending: bool,
    listener: Option<Waker>,
}

#[derive(Debug, Default)]
struct Shared(Mutex<State>);

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Wake for Shared {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let listener = {
            let mut state = self.state();
            state.pending = true;
            state.listener.take()
        };
        // Woken outside the lock, so the listener may poll again at once.
        if let Some(listener) = listener {
            listener.wake();
        }
    }
}

/// Tells an event loop that can await that a tree has queued work.
///
/// A [`Host`](crate::Host) returns [`WakeSignal::waker`], and the event loop
/// awaits [`WakeSignal::wait`] and then calls [`Tree::pump`](crate::Tree::pump).
/// Any number of wake-ups before the next wait become one.
#[derive(Clone, Debug, Default)]
pub struct WakeSignal(Arc<Shared>);

impl WakeSignal {
    /// Creates a signal with nothing pending.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the waker that raises this signal.
    #[must_use]
    pub fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.0))
    }

    /// Returns whether a wake-up is waiting to be taken.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.0.state().pending
    }

    /// Takes a waiting wake-up, or registers `context` for the next one.
    pub fn poll_wake(&self, context: &Context<'_>) -> Poll<()> {
        let mut state = self.0.state();
        if core::mem::take(&mut state.pending) {
            return Poll::Ready(());
        }
        match &mut state.listener {
            Some(listener) => listener.clone_from(context.waker()),
            None => state.listener = Some(context.waker().clone()),
        }
        Poll::Pending
    }

    /// Completes at the next wake-up, or at once if one is waiting.
    pub fn wait(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(|context| self.poll_wake(context))
    }
}

impl Hash for WakeSignal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

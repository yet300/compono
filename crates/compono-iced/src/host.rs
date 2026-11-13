use compono::{Commit, Host, WakeSignal};
use std::task::Waker;

/// A [`Host`] for trees driven by an iced application.
#[derive(Clone, Debug, Default)]
pub struct IcedHost {
    signal: WakeSignal,
}

impl IcedHost {
    /// Creates a host with a fresh signal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the signal to pass to [`wakes`](crate::wakes).
    #[must_use]
    pub fn signal(&self) -> WakeSignal {
        self.signal.clone()
    }
}

impl Host for IcedHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}

    fn waker(&self) -> Waker {
        self.signal.waker()
    }
}

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// A future that completes after a delay.
pub type SleepFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// What components need from outside the tree, passed down by constructors.
#[derive(Clone)]
pub struct Services {
    sleep: Arc<dyn Fn(Duration) -> SleepFuture + Send + Sync>,
}

impl std::fmt::Debug for Services {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Services").finish_non_exhaustive()
    }
}

impl Services {
    /// Creates services around a clock.
    pub fn new(sleep: impl Fn(Duration) -> SleepFuture + Send + Sync + 'static) -> Self {
        Self {
            sleep: Arc::new(sleep),
        }
    }

    /// Returns a future that completes after `delay`.
    pub fn sleep(&self, delay: Duration) -> SleepFuture {
        (self.sleep)(delay)
    }
}

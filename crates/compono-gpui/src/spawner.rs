use compono::{Spawner, TaskGuard};
use gpui::BackgroundExecutor;
use std::future::Future;
use std::pin::Pin;

/// A [`Spawner`] on GPUI's background threads.
#[derive(Clone)]
pub struct GpuiSpawner {
    executor: BackgroundExecutor,
}

impl std::fmt::Debug for GpuiSpawner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuiSpawner").finish_non_exhaustive()
    }
}

impl GpuiSpawner {
    /// Spawns onto `executor`.
    #[must_use]
    pub const fn new(executor: BackgroundExecutor) -> Self {
        Self { executor }
    }
}

impl Spawner for GpuiSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard {
        // A GPUI task is cancelled when dropped.
        TaskGuard::new(self.executor.spawn(task))
    }
}

//! Runs Compono component tasks on a Tokio runtime.
//!
//! ```
//! use compono::{Component, Cx, Tree};
//! use compono_tokio::TokioSpawner;
//! # use compono::{Commit, Host};
//! # struct Quiet;
//! # impl Host for Quiet {
//! #     fn commit(&mut self, _commit: &Commit<'_>) {}
//! # }
//!
//! struct Loader {
//!     loaded: Option<u32>,
//! }
//!
//! enum Msg {
//!     Load,
//!     Loaded(u32),
//! }
//!
//! impl Component for Loader {
//!     type Msg = Msg;
//!     type Output = ();
//!
//!     fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
//!         match msg {
//!             // The task belongs to this component and is cancelled with it.
//!             Msg::Load => {
//!                 let _no_spawner = cx.perform(async { 40 + 2 }, Msg::Loaded);
//!             }
//!             Msg::Loaded(value) => self.loaded = Some(value),
//!         }
//!     }
//! }
//!
//! let runtime = tokio::runtime::Builder::new_current_thread().build()?;
//! let mut tree = Tree::new(Quiet);
//! tree.set_spawner(TokioSpawner::new(runtime.handle().clone()));
//! let loader = tree.mount(|_cx| Loader { loaded: None });
//! tree.send(loader, Msg::Load)?;
//!
//! // A real host pumps when its waker fires. Here the runtime is run by hand.
//! runtime.block_on(tokio::task::yield_now());
//! tree.pump()?;
//! assert_eq!(tree.get(loader).and_then(|loader| loader.loaded), Some(42));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use compono::{Spawner, TaskGuard};
use core::future::Future;
use core::pin::Pin;
use tokio::runtime::Handle;
use tokio::task::AbortHandle;

/// A [`Spawner`] backed by a Tokio runtime.
#[derive(Clone, Debug)]
pub struct TokioSpawner {
    handle: Handle,
}

impl TokioSpawner {
    /// Spawns onto the runtime behind `handle`.
    #[must_use]
    pub const fn new(handle: Handle) -> Self {
        Self { handle }
    }
}

struct AbortOnDrop(AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Spawner for TokioSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard {
        TaskGuard::new(AbortOnDrop(self.handle.spawn(task).abort_handle()))
    }
}

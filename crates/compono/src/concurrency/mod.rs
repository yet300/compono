//! How work on other threads reaches a tree, which itself stays on one thread.

pub(crate) mod local;
pub(crate) mod sender;
mod sync;
mod task;
mod wake;

pub use sender::{QueueClosed, SendError, Sender};
pub use task::{NoSpawner, Spawner, TaskGuard, TaskId};
pub use wake::WakeSignal;

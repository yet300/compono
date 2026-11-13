use crate::tree::Tree;
use crate::{Component, Cx};
use alloc::boxed::Box;
use core::any::Any;
use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Names a task started with [`Cx::perform`], so it can be cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Stops its task when dropped.
pub struct TaskGuard {
    _cancel_on_drop: Box<dyn Any>,
}

impl TaskGuard {
    /// Wraps whatever value cancels the task on drop.
    pub fn new(cancel_on_drop: impl Any) -> Self {
        Self {
            _cancel_on_drop: Box::new(cancel_on_drop),
        }
    }
}

impl fmt::Debug for TaskGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TaskGuard")
    }
}

/// Runs background work, so the core stays free of any executor.
pub trait Spawner {
    /// Starts `task`. Dropping the returned guard must cancel it.
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard;
}

/// No [`Spawner`] was installed on the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoSpawner;

impl fmt::Display for NoSpawner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("tree has no spawner")
    }
}
impl core::error::Error for NoSpawner {}

impl Tree {
    /// Installs the executor bridge used by [`Cx::perform`].
    pub fn set_spawner(&mut self, spawner: impl Spawner + 'static) {
        self.spawner = Some(Box::new(spawner));
    }
}

impl<C: Component> Cx<'_, C> {
    /// Stops a task of this component and discards any result it already produced.
    ///
    /// Returns whether the task was still outstanding. This is the counterpart
    /// of a React effect cleanup: cancel the old request before starting a new one.
    pub fn cancel(&mut self, task: TaskId) -> bool {
        let Some(slot) = self.tree.slot_mut(self.me) else {
            return false;
        };
        if let Some(position) = slot.tasks.iter().position(|(id, _)| *id == task.0) {
            drop(slot.tasks.swap_remove(position));
            return true;
        }
        if let Some(position) = slot.local.iter().position(|local| local.id() == task.0) {
            drop(slot.local.swap_remove(position));
            return true;
        }
        false
    }
}

/// Runs a future and gives `None` if a poll panics, instead of unwinding further.
///
/// Built only on `std::panic::catch_unwind`, so it adds no dependency. Under
/// `panic = "abort"` it is a plain forwarding future.
struct CatchUnwind<F>(F);

impl<F: Future + Unpin> Future for CatchUnwind<F> {
    type Output = Option<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = Pin::new(&mut self.0);
        match catch_unwind(AssertUnwindSafe(|| future.poll(cx))) {
            Ok(Poll::Ready(value)) => Poll::Ready(Some(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(_panic) => Poll::Ready(None),
        }
    }
}

impl<C: Component> Cx<'_, C>
where
    C::Msg: Send,
{
    /// Runs `future` in the background and delivers its result as a message.
    ///
    /// The task is cancelled when this component is destroyed, and a late result
    /// is discarded, so a finished request can never reach a removed component.
    ///
    /// If `future` or `to_msg` panics, the panic stops at the task: its guard is
    /// released at the next [`Tree::pump`], and the component receives nothing
    /// from it. Builds with `panic = "abort"` stop the process instead.
    ///
    /// # Errors
    ///
    /// Returns [`NoSpawner`] if the tree has no executor bridge.
    pub fn perform<T>(
        &mut self,
        future: impl Future<Output = T> + Send + 'static,
        to_msg: impl FnOnce(T) -> C::Msg + Send + 'static,
    ) -> Result<TaskId, NoSpawner> {
        let id = self.tree.next_task;
        self.tree.next_task = id.wrapping_add(1);
        let sender = self.sender();
        let task = Box::pin(async move {
            let work = core::pin::pin!(async move { to_msg(future.await) });
            // A task that panicked still reports, so its guard is released.
            let _ignored = match CatchUnwind(work).await {
                Some(msg) => sender.push(Some(Box::new(msg)), Some(id)),
                None => sender.fail(id),
            };
        });
        let guard = self.tree.spawner.as_ref().ok_or(NoSpawner)?.spawn(task);
        if let Some(slot) = self.tree.slot_mut(self.me) {
            slot.tasks.push((id, guard));
        }
        Ok(TaskId(id))
    }
}

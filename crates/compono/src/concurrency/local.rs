//! Tasks that are not `Send`, owned and polled by the tree on its own thread.
//!
//! The tree keeps each such future in its component's slot and polls it during
//! [`Tree::pump`]. Its waker only records which task to poll and wakes the host,
//! so no value that is not `Send` is shared with anything outside the tree.
//!
//! A task is listed once or not at all. Its waker's `listed` flag is set by the
//! wake-up that lists it and cleared by the pump just before the poll, so a
//! wake-up during the poll lists the task again and none is lost.

use super::sync::{AtomicBool, Mutex, Ordering};
use crate::tree::{Tree, Work};
use crate::{Component, Cx, NodeId, TaskId};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::task::Wake;
use alloc::vec::Vec;
use core::any::Any;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::sync::PoisonError;

type LocalFuture = Pin<Box<dyn Future<Output = Box<dyn Any>>>>;

/// How many local tasks one pump polls at most.
///
/// A poll runs user code of any length, unlike delivering a message, so this is
/// far below the work limit.
pub(crate) const DEFAULT_POLL_LIMIT: usize = 1024;

/// A future that is not `Send`, and the waker that schedules its next poll.
pub(crate) struct LocalTask {
    id: u64,
    state: Arc<TaskWaker>,
    waker: Waker,
    future: LocalFuture,
}

impl Drop for LocalTask {
    /// A task that has finished or been cancelled is never listed again.
    fn drop(&mut self) {
        self.state.listed.store(true, Ordering::Release);
    }
}

impl LocalTask {
    pub(crate) const fn id(&self) -> u64 {
        self.id
    }
}

/// Tasks woken since the last pump, each once, with the host to wake about them.
pub(crate) struct Woken {
    tasks: Mutex<Vec<(NodeId, u64)>>,
    host: Waker,
}

impl Woken {
    pub(crate) fn new(host: Waker) -> Arc<Self> {
        Arc::new(Self {
            tasks: Mutex::new(Vec::new()),
            host,
        })
    }

    fn mark(&self, node: NodeId, id: u64) {
        self.tasks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((node, id));
        self.host.wake_by_ref();
    }
}

struct TaskWaker {
    node: NodeId,
    id: u64,
    /// Whether the task is in `woken`, or has ended and must not enter it.
    listed: AtomicBool,
    woken: Arc<Woken>,
}

impl TaskWaker {
    /// Lists the task and wakes the host, unless the task is already listed.
    fn list(&self) {
        // Release publishes what the waking thread wrote before it to the pump,
        // whose clearing swap acquires it, even when this wake-up adds nothing.
        if !self.listed.swap(true, Ordering::AcqRel) {
            self.woken.mark(self.node, self.id);
        }
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.list();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.list();
    }
}

impl<C: Component> Cx<'_, C> {
    /// Runs `future` on the tree's own thread and delivers its result as a message.
    ///
    /// Unlike [`Cx::perform`], neither the future nor the message needs to be
    /// `Send`, so a browser `fetch` or a toolkit's main-thread future can be
    /// awaited. No [`Spawner`](crate::Spawner) is needed: the tree polls the
    /// future during [`Tree::pump`], and the future's wake-ups reach the host's
    /// waker, which asks for that pump.
    ///
    /// The task is cancelled when this component is destroyed or by
    /// [`Cx::cancel`], and its result is then never delivered.
    pub fn perform_local<T: 'static>(
        &mut self,
        future: impl Future<Output = T> + 'static,
        to_msg: impl FnOnce(T) -> C::Msg + 'static,
    ) -> TaskId {
        let id = self.tree.next_task;
        self.tree.next_task = id.wrapping_add(1);
        let state = Arc::new(TaskWaker {
            node: self.me,
            id,
            listed: AtomicBool::new(false),
            woken: Arc::clone(&self.tree.woken),
        });
        let waker = Waker::from(Arc::clone(&state));
        let future: LocalFuture = Box::pin(async move {
            let msg: Box<dyn Any> = Box::new(to_msg(future.await));
            msg
        });
        if let Some(slot) = self.tree.slot_mut(self.me) {
            // The first poll happens at the next pump, which this asks for.
            state.list();
            slot.local.push(LocalTask {
                id,
                state,
                waker,
                future,
            });
        }
        TaskId::new(id)
    }
}

impl Tree {
    /// Sets how many tasks started with [`Cx::perform_local`] one
    /// [`Tree::pump`] polls at most.
    ///
    /// A poll runs the task's own code, which can take long, so a burst of
    /// woken tasks is spread over several pumps. The rest wait for the next
    /// pump, ahead of tasks woken later, and the host is asked for it. Messages
    /// are bounded separately, by [`Tree::set_work_limit`].
    ///
    /// The default is 1 024. A limit of 0 is treated as 1.
    pub fn set_poll_limit(&mut self, limit: usize) {
        self.poll_limit = limit.max(1);
    }

    /// Polls the tasks that are not `Send` and were woken since the last pump, up
    /// to the poll limit. The rest wait, ahead of later wake-ups, for the next
    /// pump, which the host is asked for.
    pub(crate) fn poll_local(&mut self) {
        // Swapping keeps both buffers' capacity, so pumping does not allocate.
        let mut batch = core::mem::take(&mut self.woken_batch);
        core::mem::swap(
            &mut *self
                .woken
                .tasks
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
            &mut batch,
        );
        let polled = batch.len().min(self.poll_limit);
        for (node, id) in batch.drain(..polled) {
            let Some(slot) = self.slot_mut(node) else {
                continue;
            };
            let Some(position) = slot.local.iter().position(|task| task.id == id) else {
                continue;
            };
            let Some(task) = slot.local.get_mut(position) else {
                continue;
            };
            // Cleared before the poll, so a wake-up during it lists the task
            // again. Acquire sees what the waking thread wrote before waking.
            task.state.listed.swap(false, Ordering::AcqRel);
            let mut context = Context::from_waker(&task.waker);
            if let Poll::Ready(payload) = task.future.as_mut().poll(&mut context) {
                drop(slot.local.swap_remove(position));
                self.enqueue(Work::Msg { to: node, payload });
            }
        }
        if !batch.is_empty() {
            let mut tasks = self
                .woken
                .tasks
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            tasks.splice(0..0, batch.drain(..));
            drop(tasks);
            self.woken.host.wake_by_ref();
        }
        self.woken_batch = batch;
    }
}

#[cfg(test)]
mod tests {
    use crate::{Commit, Component, Cx, Host, Tree};
    use alloc::rc::Rc;
    use core::cell::RefCell;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll, Waker};
    use std::sync::PoisonError;

    struct NullHost;
    impl Host for NullHost {
        fn commit(&mut self, _commit: &Commit<'_>) {}
    }

    struct Unit;
    impl Component for Unit {
        type Msg = ();
        type Output = ();
        fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    }

    /// Never finishes and hands out its waker.
    struct Parked(Rc<RefCell<Option<Waker>>>);

    impl Future for Parked {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            *self.0.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    fn waiting(tree: &Tree) -> usize {
        tree.woken
            .tasks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    #[test]
    fn a_task_woken_many_times_is_listed_once() {
        let mut tree = Tree::new(NullHost);
        let waker = Rc::new(RefCell::new(None));
        let parked = Parked(Rc::clone(&waker));
        tree.mount(|cx| {
            cx.perform_local(parked, |()| ());
            Unit
        });
        assert_eq!(waiting(&tree), 1, "a new task waits for its first poll");
        assert!(tree.pump().is_ok());
        assert_eq!(waiting(&tree), 0);
        let waker = waker.borrow_mut().take();
        for _ in 0..10_000 {
            waker.iter().for_each(Waker::wake_by_ref);
        }
        assert_eq!(waiting(&tree), 1, "repeated wake-ups add nothing");
    }
}

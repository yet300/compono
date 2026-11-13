use super::sync::{AtomicUsize, Mutex, Ordering};
use crate::tree::{Tree, Work};
use crate::{Component, Cx, NodeId};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::any::Any;
use core::fmt;
use core::marker::PhantomData;
use core::task::Waker;

/// How many messages can wait for [`Tree::pump`] before senders are refused.
const DEFAULT_QUEUE_LIMIT: usize = 1 << 16;

pub(crate) struct Envelope {
    to: NodeId,
    /// `None` for a task that panicked, which delivers nothing.
    payload: Option<Box<dyn Any + Send>>,
    /// The finished task that produced this message, whose guard can now go.
    task: Option<u64>,
}

/// The waiting messages and whether their tree still exists, under one lock.
#[derive(Default)]
struct Queue {
    items: VecDeque<Envelope>,
    closed: bool,
}

pub(crate) struct Inbox {
    queue: Mutex<Queue>,
    limit: AtomicUsize,
    waker: Waker,
}

impl Inbox {
    /// Refuses every later message and drops the waiting ones.
    ///
    /// Called when the tree is dropped, so a sender that outlives it cannot
    /// fill a queue nobody will read.
    pub(crate) fn close(&self) {
        let waiting = match self.queue.lock() {
            Ok(mut queue) => {
                queue.closed = true;
                core::mem::take(&mut queue.items)
            }
            Err(_poisoned) => return,
        };
        // Dropped outside the lock, because a message's `Drop` is user code.
        drop(waiting);
    }

    pub(crate) fn new(waker: Waker) -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(Queue::default()),
            limit: AtomicUsize::new(DEFAULT_QUEUE_LIMIT),
            waker,
        })
    }
}

/// The queue was poisoned by a panic on another thread, or the tree by a panic in
/// one of its operations ([`Tree::is_poisoned`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueClosed;

impl fmt::Display for QueueClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("component queue is closed")
    }
}
impl core::error::Error for QueueClosed {}

/// A message could not be queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SendError {
    /// The tree was dropped, or another thread panicked while using the queue.
    Closed,
    /// The host is not calling [`Tree::pump`] fast enough, so the message was refused.
    Full,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "component queue is closed",
            Self::Full => "component queue is full",
        })
    }
}
impl core::error::Error for SendError {}

/// Sends messages to one component from any thread.
pub struct Sender<C> {
    inbox: Arc<Inbox>,
    id: NodeId,
    marker: PhantomData<fn() -> C>,
}

impl<C> Clone for Sender<C> {
    fn clone(&self) -> Self {
        Self {
            inbox: Arc::clone(&self.inbox),
            id: self.id,
            marker: PhantomData,
        }
    }
}

impl<C> fmt::Debug for Sender<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<C: Component> Sender<C>
where
    C::Msg: Send,
{
    /// Queues a message and wakes the host. A message for a removed component is dropped.
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Full`] if too many messages are waiting, and
    /// [`SendError::Closed`] if the tree was dropped or another thread panicked
    /// while using the queue.
    pub fn send(&self, msg: C::Msg) -> Result<(), SendError> {
        self.push(Some(Box::new(msg)), None)
    }

    /// Reports a task that panicked, so the pump releases its guard.
    pub(super) fn fail(&self, task: u64) -> Result<(), SendError> {
        self.push(None, Some(task))
    }

    /// Queues `payload` for the component. A result of `task` is never
    /// refused, because running tasks already bound their number.
    pub(super) fn push(
        &self,
        payload: Option<Box<dyn Any + Send>>,
        task: Option<u64>,
    ) -> Result<(), SendError> {
        let mut queue = self
            .inbox
            .queue
            .lock()
            .map_err(|_error| SendError::Closed)?;
        if queue.closed {
            return Err(SendError::Closed);
        }
        if task.is_none() && queue.items.len() >= self.inbox.limit.load(Ordering::Relaxed) {
            return Err(SendError::Full);
        }
        queue.items.push_back(Envelope {
            to: self.id,
            payload,
            task,
        });
        drop(queue);
        self.inbox.waker.wake_by_ref();
        Ok(())
    }
}

impl Tree {
    /// Sets how many messages from [`Sender::send`] can wait for [`Tree::pump`].
    pub fn set_queue_limit(&self, limit: usize) {
        self.inbox.limit.store(limit, Ordering::Relaxed);
    }

    /// Delivers everything queued from other threads, as one commit.
    ///
    /// # Errors
    ///
    /// Returns [`QueueClosed`] if another thread panicked while using the queue,
    /// or if the tree is poisoned.
    pub fn pump(&mut self) -> Result<(), QueueClosed> {
        self.operate(Err(QueueClosed), Self::pump_now)
    }

    fn pump_now(&mut self) -> Result<(), QueueClosed> {
        // Swapping keeps both buffers' capacity, so pumping does not allocate.
        let mut batch = core::mem::take(&mut self.inbox_batch);
        {
            let mut queue = self.inbox.queue.lock().map_err(|_error| QueueClosed)?;
            core::mem::swap(&mut queue.items, &mut batch);
        }
        while let Some(envelope) = batch.pop_front() {
            let Envelope { to, payload, task } = envelope;
            if let Some(task) = task {
                // A result with no guard belongs to a task that was cancelled.
                let Some(slot) = self.slot_mut(to) else {
                    continue;
                };
                let Some(position) = slot.tasks.iter().position(|(id, _)| *id == task) else {
                    continue;
                };
                drop(slot.tasks.swap_remove(position));
            }
            let Some(payload) = payload else {
                continue;
            };
            let payload: Box<dyn Any> = payload;
            self.enqueue(Work::Msg { to, payload });
        }
        self.inbox_batch = batch;
        self.poll_local();
        self.finish();
        Ok(())
    }
}

impl<C: Component> Cx<'_, C> {
    /// Returns a thread-safe way to message this component later.
    #[must_use]
    pub fn sender(&self) -> Sender<C> {
        Sender {
            inbox: Arc::clone(&self.tree.inbox),
            id: self.me,
            marker: PhantomData,
        }
    }
}

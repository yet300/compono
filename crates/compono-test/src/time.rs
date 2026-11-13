use compono::{QueueClosed, Spawner, TaskGuard, Tree};
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

type BoxedTask = Pin<Box<dyn Future<Output = ()> + Send>>;
type TimerKey = (Duration, u64);

#[derive(Default)]
struct State {
    now: Duration,
    next_id: u64,
    timers: BTreeMap<TimerKey, Waker>,
    tasks: BTreeMap<u64, BoxedTask>,
    ready: VecDeque<u64>,
}

fn lock(shared: &Mutex<State>) -> MutexGuard<'_, State> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A clock and executor that move only when told, so timing tests are exact.
///
/// Clones share one clock. Install a clone with
/// [`Tree::set_spawner`](compono::Tree::set_spawner).
#[derive(Clone, Default)]
pub struct VirtualTime {
    shared: Arc<Mutex<State>>,
}

impl std::fmt::Debug for VirtualTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualTime")
            .field("now", &self.now())
            .field("tasks", &self.live_tasks())
            .finish()
    }
}

struct TaskWaker {
    id: u64,
    shared: Weak<Mutex<State>>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if let Some(shared) = self.shared.upgrade() {
            lock(&shared).ready.push_back(self.id);
        }
    }
}

struct CancelTask {
    id: u64,
    shared: Weak<Mutex<State>>,
}

impl Drop for CancelTask {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            // The future is dropped after unlocking, because its timers lock too.
            let task = lock(&shared).tasks.remove(&self.id);
            drop(task);
        }
    }
}

impl Spawner for VirtualTime {
    fn spawn(&self, task: BoxedTask) -> TaskGuard {
        let mut state = lock(&self.shared);
        let id = state.next_id;
        state.next_id += 1;
        state.tasks.insert(id, task);
        state.ready.push_back(id);
        drop(state);
        TaskGuard::new(CancelTask {
            id,
            shared: Arc::downgrade(&self.shared),
        })
    }
}

impl VirtualTime {
    /// Creates a clock at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current virtual time.
    #[must_use]
    pub fn now(&self) -> Duration {
        lock(&self.shared).now
    }

    /// Returns the number of tasks neither finished nor cancelled.
    #[must_use]
    pub fn live_tasks(&self) -> usize {
        lock(&self.shared).tasks.len()
    }

    /// Returns a future that completes `delay` from now.
    #[must_use]
    pub fn sleep(&self, delay: Duration) -> Sleep {
        Sleep {
            shared: Arc::clone(&self.shared),
            deadline: self.now().saturating_add(delay),
            timer: None,
        }
    }

    /// Runs tasks and delivers their messages until nothing more can happen now.
    ///
    /// # Errors
    ///
    /// Returns [`QueueClosed`] if the tree's queue is poisoned.
    pub fn settle(&self, tree: &mut Tree) -> Result<(), QueueClosed> {
        loop {
            self.run_ready();
            tree.pump()?;
            if lock(&self.shared).ready.is_empty() {
                return Ok(());
            }
        }
    }

    /// Moves the clock forward, firing timers in order and settling after each.
    ///
    /// # Errors
    ///
    /// Returns [`QueueClosed`] if the tree's queue is poisoned.
    pub fn advance(&self, tree: &mut Tree, by: Duration) -> Result<(), QueueClosed> {
        let target = self.now().saturating_add(by);
        self.settle(tree)?;
        while let Some(waker) = self.fire_next(target) {
            waker.wake();
            self.settle(tree)?;
        }
        lock(&self.shared).now = target;
        self.settle(tree)
    }

    fn fire_next(&self, target: Duration) -> Option<Waker> {
        let mut state = lock(&self.shared);
        let key = *state.timers.keys().next().filter(|key| key.0 <= target)?;
        state.now = key.0;
        state.timers.remove(&key)
    }

    fn run_ready(&self) {
        loop {
            let mut state = lock(&self.shared);
            let Some(id) = state.ready.pop_front() else {
                return;
            };
            let Some(mut task) = state.tasks.remove(&id) else {
                continue;
            };
            drop(state);
            let waker = Waker::from(Arc::new(TaskWaker {
                id,
                shared: Arc::downgrade(&self.shared),
            }));
            let mut context = Context::from_waker(&waker);
            if task.as_mut().poll(&mut context).is_pending() {
                lock(&self.shared).tasks.insert(id, task);
            }
        }
    }
}

/// The future returned by [`VirtualTime::sleep`].
pub struct Sleep {
    shared: Arc<Mutex<State>>,
    deadline: Duration,
    timer: Option<TimerKey>,
}

impl std::fmt::Debug for Sleep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sleep")
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let shared = Arc::clone(&self.shared);
        let mut state = lock(&shared);
        if state.now >= self.deadline {
            return Poll::Ready(());
        }
        let key = match self.timer {
            Some(key) => key,
            None => {
                let key = (self.deadline, state.next_id);
                state.next_id += 1;
                key
            }
        };
        state.timers.insert(key, context.waker().clone());
        drop(state);
        self.timer = Some(key);
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let Some(key) = self.timer {
            lock(&self.shared).timers.remove(&key);
        }
    }
}

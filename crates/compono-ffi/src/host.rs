use compono::{Commit, Host};
use core::cell::RefCell;
use core::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Wake, Waker};

/// What the native side provides.
pub trait Platform: Send + Sync + 'static {
    /// Reports one finished call on the tree's thread.
    ///
    /// `dirty` holds [`NodeId::to_bits`](compono::NodeId::to_bits) values, each
    /// once. The tree is free again, so this may call back into it.
    fn commit(&self, dirty: &[u64], can_back: bool);

    /// Asks for [`NativeTree::pump`](crate::NativeTree::pump) on the tree's thread.
    ///
    /// Called from any thread, and must not pump before returning. It is also
    /// called when the tree's [`NativeTree`](crate::NativeTree) is dropped on
    /// another thread, so that the owner frees the tree. A pump of any tree on
    /// the owner thread does that, and a binding left with no tree to pump calls
    /// [`collect_retired`](crate::collect_retired).
    fn schedule_pump(&self);
}

#[derive(Default)]
struct Mail {
    dirty: Vec<u64>,
    can_back: bool,
    commits: u32,
}

/// Commits waiting for the tree to be released.
#[derive(Default)]
pub(crate) struct Inbox(RefCell<Mail>);

impl Inbox {
    fn record(&self, commit: &Commit<'_>) {
        let mut mail = self.0.borrow_mut();
        mail.dirty
            .extend(commit.dirty.iter().map(|id| id.to_bits()));
        mail.can_back = commit.can_back;
        mail.commits = mail.commits.saturating_add(1);
    }

    /// Takes what is waiting, keeping each address once.
    pub(crate) fn take(&self) -> Option<(Vec<u64>, bool)> {
        let mut mail = self.0.borrow_mut();
        if mail.commits == 0 {
            return None;
        }
        if mail.commits > 1 {
            mail.dirty.sort_unstable();
            mail.dirty.dedup();
        }
        mail.commits = 0;
        Some((core::mem::take(&mut mail.dirty), mail.can_back))
    }

    /// Returns a delivered buffer so that the next commit does not allocate.
    pub(crate) fn recycle(&self, mut dirty: Vec<u64>) {
        let mut mail = self.0.borrow_mut();
        if mail.dirty.capacity() == 0 {
            dirty.clear();
            mail.dirty = dirty;
        }
    }
}

struct PumpRequest(Arc<dyn Platform>);

impl Wake for PumpRequest {
    fn wake(self: Arc<Self>) {
        self.0.schedule_pump();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.schedule_pump();
    }
}

/// A [`Host`] that holds each commit until the tree is released.
pub struct NativeHost {
    inbox: Rc<Inbox>,
    request: Arc<PumpRequest>,
}

impl fmt::Debug for NativeHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeHost").finish_non_exhaustive()
    }
}

impl NativeHost {
    /// Creates a host that wakes `platform`.
    #[must_use]
    pub fn new(platform: Arc<dyn Platform>) -> Self {
        Self {
            inbox: Rc::default(),
            request: Arc::new(PumpRequest(platform)),
        }
    }

    pub(crate) fn inbox(&self) -> Rc<Inbox> {
        Rc::clone(&self.inbox)
    }
}

impl Host for NativeHost {
    fn commit(&mut self, commit: &Commit<'_>) {
        self.inbox.record(commit);
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.request))
    }
}

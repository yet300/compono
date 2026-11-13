use compono::{Commit, Diagnostic, Host, NodeId};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Wake, Waker};

/// An owned copy of one [`Commit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRecord {
    /// The components that changed.
    pub dirty: Vec<NodeId>,
    /// Whether Back would be handled.
    pub can_back: bool,
}

/// An owned copy of one [`Diagnostic`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticRecord {
    /// [`Diagnostic::RouterLimit`].
    RouterLimit {
        /// The component that made the router.
        owner: NodeId,
    },
    /// [`Diagnostic::RepeatedRouterName`].
    RepeatedRouterName {
        /// The component that named the router.
        owner: NodeId,
    },
    /// [`Diagnostic::RepeatedChildName`].
    RepeatedChildName {
        /// The parent of both children.
        owner: NodeId,
        /// The repeated name.
        name: String,
    },
    /// A diagnostic this version does not know, as its `Debug` text.
    Other(String),
}

impl From<&Diagnostic<'_>> for DiagnosticRecord {
    fn from(diagnostic: &Diagnostic<'_>) -> Self {
        match *diagnostic {
            Diagnostic::RouterLimit { owner } => Self::RouterLimit { owner },
            Diagnostic::RepeatedRouterName { owner } => Self::RepeatedRouterName { owner },
            Diagnostic::RepeatedChildName { owner, name } => Self::RepeatedChildName {
                owner,
                name: name.to_owned(),
            },
            ref other => Self::Other(format!("{other:?}")),
        }
    }
}

/// What a [`Recording`] host has seen. Clones share one log.
#[derive(Clone, Debug, Default)]
pub struct CommitLog {
    commits: Rc<RefCell<Vec<CommitRecord>>>,
    diagnostics: Rc<RefCell<Vec<DiagnosticRecord>>>,
    wakes: Arc<AtomicUsize>,
}

impl CommitLog {
    /// Returns the number of commits since the last [`CommitLog::clear`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.commits.borrow().len()
    }

    /// Returns whether no commit has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commits.borrow().is_empty()
    }

    /// Returns a copy of the commits.
    #[must_use]
    pub fn commits(&self) -> Vec<CommitRecord> {
        self.commits.borrow().clone()
    }

    /// Returns a copy of the newest commit.
    #[must_use]
    pub fn last(&self) -> Option<CommitRecord> {
        self.commits.borrow().last().cloned()
    }

    /// Returns a copy of the diagnostics, oldest first.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<DiagnosticRecord> {
        self.diagnostics.borrow().clone()
    }

    /// Forgets recorded commits and diagnostics. Wake counts are kept.
    pub fn clear(&self) {
        self.commits.borrow_mut().clear();
        self.diagnostics.borrow_mut().clear();
    }

    /// Returns how many times the host's waker has fired.
    #[must_use]
    pub fn wakes(&self) -> usize {
        self.wakes.load(Ordering::SeqCst)
    }
}

/// A host that does nothing, for tests that only read the log.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHost;

impl Host for NoHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

/// Records everything passing through to an inner host.
#[derive(Debug)]
pub struct Recording<H> {
    inner: H,
    log: CommitLog,
}

/// Wraps `inner` so its traffic can be inspected.
pub fn record<H: Host>(inner: H) -> (Recording<H>, CommitLog) {
    let log = CommitLog::default();
    let host = Recording {
        inner,
        log: log.clone(),
    };
    (host, log)
}

/// Creates a recording host with no UI behind it.
#[must_use]
pub fn headless() -> (Recording<NoHost>, CommitLog) {
    record(NoHost)
}

struct CountingWaker {
    wakes: Arc<AtomicUsize>,
    inner: Waker,
}

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
        self.inner.wake_by_ref();
    }
}

impl<H: Host> Host for Recording<H> {
    fn commit(&mut self, commit: &Commit<'_>) {
        self.log.commits.borrow_mut().push(CommitRecord {
            dirty: commit.dirty.to_vec(),
            can_back: commit.can_back,
        });
        self.inner.commit(commit);
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic<'_>) {
        self.log
            .diagnostics
            .borrow_mut()
            .push(DiagnosticRecord::from(diagnostic));
        self.inner.diagnostic(diagnostic);
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::new(CountingWaker {
            wakes: Arc::clone(&self.log.wakes),
            inner: self.inner.waker(),
        }))
    }
}

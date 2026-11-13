use compono::{Commit, Host, NodeId, WakeSignal};
use std::cell::Cell;
use std::rc::Rc;
use std::task::Waker;

/// Records which components changed since the last check.
#[derive(Clone, Default)]
pub struct Changes(Rc<Cell<Vec<NodeId>>>);

impl std::fmt::Debug for Changes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Changes")
    }
}

impl Changes {
    /// Returns the changed components, and resets the record.
    #[must_use]
    pub fn take(&self) -> Vec<NodeId> {
        self.0.take()
    }

    fn add(&self, ids: &[NodeId]) {
        let mut all = self.0.take();
        all.extend_from_slice(ids);
        self.0.set(all);
    }
}

/// A [`Host`] for a tree owned by a GPUI view.
#[derive(Clone, Debug, Default)]
pub struct GpuiHost {
    signal: WakeSignal,
    changes: Changes,
}

impl GpuiHost {
    /// Creates a host.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the signal to pass to [`listen`](crate::listen).
    #[must_use]
    pub fn signal(&self) -> WakeSignal {
        self.signal.clone()
    }

    /// Returns the record the view keeps for [`TreeView::changes`](crate::TreeView::changes).
    #[must_use]
    pub fn changes(&self) -> Changes {
        self.changes.clone()
    }
}

impl Host for GpuiHost {
    fn commit(&mut self, commit: &Commit<'_>) {
        self.changes.add(commit.dirty);
    }

    fn waker(&self) -> Waker {
        self.signal.waker()
    }
}

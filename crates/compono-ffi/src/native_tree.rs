use crate::host::Inbox;
use crate::{NativeError, NativeHost, Platform};
use compono::{Component, NodeId, Phase, RestoreOutcome, Tree};
use core::cell::RefCell;
use core::fmt;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

struct Owned {
    tree: RefCell<Tree>,
    inbox: Rc<Inbox>,
    platform: Arc<dyn Platform>,
}

impl Owned {
    fn deliver(&self) {
        if let Some((mut dirty, can_back)) = self.inbox.take() {
            // A call can change a component and then remove it, and the removal
            // alone reports nothing.
            if let Ok(tree) = self.tree.try_borrow() {
                dirty.retain(|bits| tree.contains(NodeId::from_bits(*bits)));
            }
            self.platform.commit(&dirty, can_back);
            self.inbox.recycle(dirty);
        }
    }
}

/// Trees whose [`NativeTree`] was dropped on another thread, waiting for their
/// owner to free them.
///
/// Each owning thread has one, shared with the handles of its trees. It lives
/// only as long as that thread or one of those handles, so a thread that has
/// ended leaves nothing behind.
#[derive(Default)]
struct Retired {
    /// A hint that `ids` is not empty, so the usual call takes no lock. It only
    /// changes while `ids` is locked, and that lock is what orders it.
    any: AtomicBool,
    ids: Mutex<Vec<u64>>,
}

impl fmt::Debug for Retired {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Retired").finish_non_exhaustive()
    }
}

impl Retired {
    fn push(&self, id: u64) {
        let mut ids = self.ids.lock().unwrap_or_else(PoisonError::into_inner);
        ids.push(id);
        self.any.store(true, Ordering::Release);
    }

    fn take(&self) -> Vec<u64> {
        if !self.any.load(Ordering::Acquire) {
            return Vec::new();
        }
        let mut ids = self.ids.lock().unwrap_or_else(PoisonError::into_inner);
        self.any.store(false, Ordering::Release);
        core::mem::take(&mut *ids)
    }
}

#[derive(Default)]
struct Local {
    trees: RefCell<BTreeMap<u64, Rc<Owned>>>,
    retired: Arc<Retired>,
}

thread_local! {
    static LOCAL: Local = Local::default();
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn remove(id: u64) {
    // The tree is dropped after the map is released, so its components may
    // drop another `NativeTree`.
    let owned = LOCAL
        .try_with(|local| local.trees.borrow_mut().remove(&id))
        .ok()
        .flatten();
    drop(owned);
}

/// Frees trees owned by this thread whose [`NativeTree`] was dropped elsewhere.
///
/// Every call on a tree does this too. A binding calls it directly when the last
/// tree may be released from a garbage collector's thread, or when
/// [`Platform::schedule_pump`] arrives and it has no tree left to pump.
pub fn collect_retired() {
    let mine = LOCAL
        .try_with(|local| local.retired.take())
        .unwrap_or_default();
    for id in mine {
        remove(id);
    }
}

/// A [`Tree`] that language runtimes can hold from any thread.
///
/// It is only an identifier. The tree stays in thread-local storage on the thread
/// that created it. Dropping this value on another thread calls
/// [`Platform::schedule_pump`], and the owner's next call on any tree, or
/// [`collect_retired`], frees the tree.
pub struct NativeTree {
    id: u64,
    owner: ThreadId,
    retired: Arc<Retired>,
    platform: Arc<dyn Platform>,
}

impl fmt::Debug for NativeTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeTree")
            .field("id", &self.id)
            .field("owner", &self.owner)
            .finish_non_exhaustive()
    }
}

impl NativeTree {
    /// Creates an empty tree owned by the current thread.
    #[must_use]
    pub fn new(platform: Arc<dyn Platform>) -> Self {
        let host = NativeHost::new(Arc::clone(&platform));
        let inbox = host.inbox();
        Self::adopt(Tree::new(host), inbox, platform)
    }

    /// Creates a tree that restores from `bytes` when their schema matches.
    #[must_use]
    pub fn restore(
        platform: Arc<dyn Platform>,
        bytes: &[u8],
        schema: u32,
    ) -> (Self, RestoreOutcome) {
        let host = NativeHost::new(Arc::clone(&platform));
        let inbox = host.inbox();
        let (tree, outcome) = Tree::with_saved(host, bytes, schema);
        (Self::adopt(tree, inbox, platform), outcome)
    }

    fn adopt(tree: Tree, inbox: Rc<Inbox>, platform: Arc<dyn Platform>) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let owned = Rc::new(Owned {
            tree: RefCell::new(tree),
            inbox,
            platform: Arc::clone(&platform),
        });
        // Fails only while the thread is exiting, when no call can reach the tree.
        let retired = LOCAL
            .try_with(|local| {
                local.trees.borrow_mut().insert(id, owned);
                Arc::clone(&local.retired)
            })
            .unwrap_or_default();
        Self {
            id,
            owner: thread::current().id(),
            retired,
            platform,
        }
    }

    fn owned(&self) -> Result<Rc<Owned>, NativeError> {
        if thread::current().id() != self.owner {
            return Err(NativeError::WrongThread);
        }
        collect_retired();
        LOCAL
            .try_with(|local| local.trees.borrow().get(&self.id).cloned())
            .ok()
            .flatten()
            .ok_or(NativeError::Gone)
    }

    /// Changes the tree, then reports the result to the platform once.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] off the owning thread,
    /// [`NativeError::Busy`] from inside another call on this tree, and
    /// [`NativeError::Poisoned`] once a panic in an earlier call was caught.
    pub fn with<R>(&self, change: impl FnOnce(&mut Tree) -> R) -> Result<R, NativeError> {
        let owned = self.owned()?;
        let value = {
            let mut tree = owned
                .tree
                .try_borrow_mut()
                .map_err(|_error| NativeError::Busy)?;
            if tree.is_poisoned() {
                return Err(NativeError::Poisoned);
            }
            change(&mut tree)
        };
        owned.deliver();
        Ok(value)
    }

    /// Reads the tree.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`].
    pub fn read<R>(&self, read: impl FnOnce(&Tree) -> R) -> Result<R, NativeError> {
        let owned = self.owned()?;
        let tree = owned
            .tree
            .try_borrow()
            .map_err(|_error| NativeError::Busy)?;
        Ok(read(&tree))
    }

    /// Sends a message to a component known by its address.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`], and [`NativeError::Gone`] if
    /// the component no longer exists or is not a `C`.
    pub fn send<C: Component>(&self, node: u64, msg: C::Msg) -> Result<(), NativeError> {
        self.with(|tree| {
            let handle = tree
                .handle_of::<C>(NodeId::from_bits(node))
                .ok_or(NativeError::Gone)?;
            tree.send(handle, msg).map_err(|_stale| NativeError::Gone)
        })?
    }

    /// Sets how present the application is.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`].
    pub fn set_phase(&self, phase: Phase) -> Result<(), NativeError> {
        self.with(|tree| tree.set_phase(phase))
    }

    /// Delivers a Back press and returns whether it was handled.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`].
    pub fn back(&self) -> Result<bool, NativeError> {
        self.with(Tree::back)
    }

    /// Starts a predictive Back gesture and returns its target's address.
    ///
    /// Returns `None` if nothing would handle Back. Progress stays in the UI, so a
    /// gesture costs one commit at each end and none per frame.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`].
    pub fn back_start(&self) -> Result<Option<u64>, NativeError> {
        self.with(|tree| {
            tree.back_start();
            tree.gesture_target().map(NodeId::to_bits)
        })
    }

    /// Abandons a gesture without navigating.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`].
    pub fn back_cancel(&self) -> Result<(), NativeError> {
        self.with(Tree::back_cancel)
    }

    /// Completes a gesture and returns whether Back was handled.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`].
    pub fn back_commit(&self) -> Result<bool, NativeError> {
        self.with(Tree::back_commit)
    }

    /// Delivers work that arrived from other threads.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::with`], and
    /// [`NativeError::QueueClosed`] if the queue was poisoned.
    pub fn pump(&self) -> Result<(), NativeError> {
        self.with(Tree::pump)?
            .map_err(|_closed| NativeError::QueueClosed)
    }

    /// Encodes every persistent component.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`NativeTree::read`], and [`NativeError::Save`] if a
    /// component cannot be encoded.
    pub fn save(&self, schema: u32) -> Result<Vec<u8>, NativeError> {
        self.read(|tree| tree.save(schema).map_err(|_error| NativeError::Save))?
    }
}

impl Drop for NativeTree {
    fn drop(&mut self) {
        if thread::current().id() == self.owner {
            remove(self.id);
            return;
        }
        self.retired.push(self.id);
        // The owner may make no other call, so it is asked for one.
        self.platform.schedule_pump();
    }
}

#[cfg(test)]
mod tests {
    use super::NativeTree;
    use crate::Platform;
    use std::sync::Arc;

    struct Silent;
    impl Platform for Silent {
        fn commit(&self, _dirty: &[u64], _can_back: bool) {}
        fn schedule_pump(&self) {}
    }

    #[test]
    fn a_tree_whose_thread_has_ended_leaves_no_record_behind() {
        let tree = std::thread::spawn(|| NativeTree::new(Arc::new(Silent)))
            .join()
            .unwrap_or_else(|_panicked| std::process::abort());
        let record = Arc::downgrade(&tree.retired);
        drop(tree);
        assert!(
            record.upgrade().is_none(),
            "nothing could ever collect a record of this tree"
        );
    }
}

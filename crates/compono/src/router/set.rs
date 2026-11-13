//! A keyed set of children reconciled against a desired list.
//!
//! [`ChildSet`] is the building block for navigation models that are easiest
//! to describe as "these keys should exist, in this order, with these phases".
//! Models with cheap point operations, such as a stack, use
//! [`Cx::scoped`](crate::Cx::scoped) directly.

use crate::router::{Router, RouterId, Scope};
use crate::{Component, Cx, Phase};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use core::hash::Hash;
use hashbrown::{HashMap, HashSet};

type Build<K, H, P> = Box<dyn Fn(&K, &mut Cx<'_, P>) -> H>;

struct Entry<K, H> {
    #[cfg_attr(
        not(feature = "serde"),
        expect(dead_code, reason = "only persistence reads this")
    )]
    id: u64,
    key: K,
    child: H,
    scope: Scope,
    phase: Phase,
    /// The phase requested by the reconcile in progress.
    wanted: Phase,
}

/// The desired list named the same key twice. Nothing was changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuplicateKey;

impl fmt::Display for DuplicateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("desired children contain a duplicate key")
    }
}
impl core::error::Error for DuplicateKey {}

/// Checks that no key repeats, before a keyed router changes or reserves anything.
pub(crate) fn check_unique<K: Eq + Hash>(keys: &[K]) -> Result<(), DuplicateKey> {
    let mut seen = HashSet::with_capacity(keys.len());
    if keys.iter().all(|key| seen.insert(key)) {
        Ok(())
    } else {
        Err(DuplicateKey)
    }
}

/// Children identified by key, held as a field of their parent `P`.
///
/// ```
/// use compono::router::set::ChildSet;
/// use compono::{Component, Cx, Handle, Phase, Tree};
/// # use compono::{Commit, Host};
/// # struct Quiet;
/// # impl Host for Quiet {
/// #     fn commit(&mut self, _commit: &Commit<'_>) {}
/// # }
/// # struct Screen;
/// # impl Component for Screen {
/// #     type Msg = ();
/// #     type Output = ();
/// #     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
/// # }
/// # fn screen<P: Component>(cx: &mut Cx<'_, P>) -> Handle<Screen> {
/// #     cx.child(|_cx| Screen, |_parent, (), _cx| {})
/// # }
///
/// /// Cards on a table, where the last one named is on top.
/// struct Table {
///     cards: ChildSet<&'static str, Handle<Screen>, Table>,
/// }
///
/// impl Component for Table {
///     type Msg = Vec<&'static str>;
///     type Output = ();
///
///     fn update(&mut self, names: Vec<&'static str>, cx: &mut Cx<'_, Self>) {
///         let top = names.len().saturating_sub(1);
///         let desired = names.into_iter().enumerate().map(|(index, name)| {
///             let phase = if index == top { Phase::Focused } else { Phase::Visible };
///             (name, phase)
///         });
///         // `Err(DuplicateKey)` changes nothing.
///         if self.cards.reconcile(desired, cx).is_err() {
///             cx.unchanged();
///         }
///     }
/// }
///
/// let mut tree = Tree::new(Quiet);
/// let table = tree.mount(|cx| Table {
///     cards: ChildSet::new(cx, |_name, cx| screen(cx)),
/// });
/// tree.send(table, vec!["ace", "king"])?;
/// let ace = tree.get(table).and_then(|table| table.cards.child_at(0).copied());
///
/// // A key that stays keeps its component, wherever it moves.
/// tree.send(table, vec!["king", "queen", "ace"])?;
/// let cards = tree.get(table).map(|table| &table.cards);
/// assert_eq!(cards.map(ChildSet::len), Some(3));
/// assert_eq!(cards.and_then(|cards| cards.child_at(2).copied()), ace);
/// # Ok::<(), compono::StaleHandle>(())
/// ```
pub struct ChildSet<K, H, P> {
    entries: Vec<Entry<K, H>>,
    /// Ids from a restored snapshot, consumed when a key is first built.
    restored: HashMap<K, u64>,
    next_id: u64,
    router: RouterId,
    build: Build<K, H, P>,
}

impl<K, H, P> fmt::Debug for ChildSet<K, H, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildSet")
            .field("len", &self.entries.len())
            .finish_non_exhaustive()
    }
}

impl<K: Eq + Hash + 'static, H: 'static, P: Component> ChildSet<K, H, P> {
    /// Creates an empty set. `build` runs once for each key that appears.
    pub fn new(cx: &mut Cx<'_, P>, build: impl Fn(&K, &mut Cx<'_, P>) -> H + 'static) -> Self {
        let router = cx.router();
        cx.forget_router(router);
        Self {
            entries: Vec::new(),
            restored: HashMap::new(),
            next_id: 0,
            router,
            build: Box::new(build),
        }
    }

    /// Returns the number of live children.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether no child is live.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the value at a position in the desired order.
    #[must_use]
    pub fn child_at(&self, position: usize) -> Option<&H> {
        self.entries.get(position).map(|entry| &entry.child)
    }

    /// Returns key, value and granted phase of each child, in desired order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &H, Phase)> {
        self.entries
            .iter()
            .map(|entry| (&entry.key, &entry.child, entry.phase))
    }

    /// Makes the live children match `desired`, in `O(n)` expected time.
    ///
    /// Removed keys are destroyed first, starting from the end of the current
    /// order. Then lowered phases are applied, new children are created, and
    /// raised phases are applied, so an old child always loses focus before a
    /// new one gains it. Children whose key survives are kept, even if their
    /// position changes.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateKey`] without changing anything if a key repeats.
    pub fn reconcile(
        &mut self,
        desired: impl IntoIterator<Item = (K, Phase)>,
        cx: &mut Cx<'_, P>,
    ) -> Result<(), DuplicateKey> {
        let desired: Vec<(K, Phase)> = desired.into_iter().collect();
        if self.same_keys(&desired) {
            self.apply_phases(&desired, cx);
            return Ok(());
        }
        let mut wanted: HashMap<&K, usize> = HashMap::with_capacity(desired.len());
        for (position, (key, _)) in desired.iter().enumerate() {
            if wanted.insert(key, position).is_some() {
                return Err(DuplicateKey);
            }
        }

        let mut kept: Vec<Option<Entry<K, H>>> = Vec::new();
        kept.resize_with(desired.len(), || None);
        for entry in self.entries.drain(..).rev() {
            let place = wanted
                .get(&entry.key)
                .copied()
                .and_then(|position| kept.get_mut(position));
            match place {
                Some(place) => *place = Some(entry),
                None => cx.destroy_scope(&entry.scope),
            }
        }
        drop(wanted);

        for (entry, (_, phase)) in kept.iter_mut().zip(&desired).rev() {
            if let Some(entry) = entry
                && *phase < entry.phase
            {
                cx.assign_scope(&entry.scope, *phase);
                entry.phase = *phase;
            }
        }

        self.entries.reserve(desired.len());
        for (entry, (key, phase)) in kept.into_iter().zip(desired) {
            let mut entry = match entry {
                Some(entry) => entry,
                None => self.create(key, cx),
            };
            entry.wanted = phase;
            self.entries.push(entry);
        }
        for entry in &mut self.entries {
            if entry.wanted > entry.phase {
                cx.assign_scope(&entry.scope, entry.wanted);
                entry.phase = entry.wanted;
            }
        }
        Ok(())
    }

    fn same_keys(&self, desired: &[(K, Phase)]) -> bool {
        self.entries.len() == desired.len()
            && self
                .entries
                .iter()
                .zip(desired)
                .all(|(entry, (key, _))| entry.key == *key)
    }

    fn apply_phases(&mut self, desired: &[(K, Phase)], cx: &mut Cx<'_, P>) {
        for (entry, (_, phase)) in self.entries.iter_mut().zip(desired).rev() {
            if *phase < entry.phase {
                cx.assign_scope(&entry.scope, *phase);
                entry.phase = *phase;
            }
        }
        for (entry, (_, phase)) in self.entries.iter_mut().zip(desired) {
            if *phase > entry.phase {
                cx.assign_scope(&entry.scope, *phase);
                entry.phase = *phase;
            }
        }
    }

    fn create(&mut self, key: K, cx: &mut Cx<'_, P>) -> Entry<K, H> {
        let id = self.restored.remove(&key).unwrap_or_else(|| {
            let id = self.next_id;
            self.next_id = id.wrapping_add(1);
            id
        });
        let (child, scope) = cx.scoped(self.router, id, |cx| (self.build)(&key, cx));
        Entry {
            id,
            key,
            child,
            scope,
            phase: Phase::Hidden,
            wanted: Phase::Hidden,
        }
    }

    /// Discards every restored key that was never rebuilt, with its saved state.
    ///
    /// A model whose keys lose their state when they leave, such as a window
    /// over a list, calls this after its first [`ChildSet::reconcile`].
    pub fn forget_waiting(&mut self, cx: &mut Cx<'_, P>) {
        for (_key, id) in self.restored.drain() {
            cx.forget_entry(self.router, id);
        }
    }

    /// Discards a restored key that was never rebuilt, along with its saved state.
    ///
    /// Restored keys wait for their next [`ChildSet::reconcile`] indefinitely,
    /// so a model that knows a key is gone for good says so here. Returns
    /// whether the key was waiting.
    pub fn forget(&mut self, key: &K, cx: &mut Cx<'_, P>) -> bool {
        match self.restored.remove(key) {
            Some(id) => {
                cx.forget_entry(self.router, id);
                true
            }
            None => false,
        }
    }
}

impl<K: Eq + Hash + 'static, H: 'static, P: Component> Router<P> for ChildSet<K, H, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        [self.router]
    }

    fn destroy(mut self, cx: &mut Cx<'_, P>) {
        for entry in self.entries.drain(..).rev() {
            cx.destroy_scope(&entry.scope);
        }
        self.forget_waiting(cx);
    }
}

#[cfg(feature = "serde")]
mod saved {
    use super::{Build, ChildSet};
    use crate::router::scope::repair_keys;
    use crate::{Component, Cx};
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::hash::Hash;
    use hashbrown::HashMap;

    /// The persistable form of a [`ChildSet`].
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct SavedChildSet<K> {
        entries: Vec<(u64, K)>,
        next_id: u64,
    }

    impl<K: Clone + Eq + Hash + 'static, H: 'static, P: Component> ChildSet<K, H, P> {
        /// Captures keys and ids, including restored keys not yet rebuilt.
        pub fn snapshot(&self) -> SavedChildSet<K> {
            let live = self
                .entries
                .iter()
                .map(|entry| (entry.id, entry.key.clone()));
            let waiting = self.restored.iter().map(|(key, id)| (*id, key.clone()));
            SavedChildSet {
                entries: live.chain(waiting).collect(),
                next_id: self.next_id,
            }
        }

        /// Prepares a set that gives each saved key its old state when it is
        /// next desired. No component is created until [`ChildSet::reconcile`].
        pub fn restore(
            saved: Option<SavedChildSet<K>>,
            cx: &mut Cx<'_, P>,
            build: impl Fn(&K, &mut Cx<'_, P>) -> H + 'static,
        ) -> Self {
            let Some(saved) = saved else {
                return Self::new(cx, build);
            };
            let (mut entries, next_id) = (saved.entries, saved.next_id);
            let next_id = repair_keys(entries.iter_mut().map(|(id, _key)| id), next_id);
            let build: Build<K, H, P> = Box::new(build);
            let restored: HashMap<K, u64> =
                entries.into_iter().map(|(id, key)| (key, id)).collect();
            let router = cx.router();
            // A key saved twice keeps one id, so only the ids still named survive.
            let named: Vec<u64> = restored.values().copied().collect();
            cx.keep_router_entries(router, &named);
            Self {
                entries: Vec::new(),
                restored,
                next_id,
                router,
                build,
            }
        }
    }
}

#[cfg(feature = "serde")]
pub use saved::SavedChildSet;

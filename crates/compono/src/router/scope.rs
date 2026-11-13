use crate::NodeId;
use crate::saved_key;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt;
#[cfg(feature = "serde")]
use hashbrown::HashSet;

/// Keeps the entries of two routers in one parent apart in saved state.
///
/// A router gets its id from [`Cx::router`](crate::Cx::router) when it is
/// constructed. Inside [`Cx::named_routers`](crate::Cx::named_routers) the id
/// is the router's name. Otherwise it is a number in construction order, so
/// unnamed routers must be constructed in the same order on every launch, and
/// changing that order is a schema change.
///
/// A component has 32 767 numbered ids that save state. Every unnamed router
/// it makes after those works as usual but saves nothing.
///
/// An id belongs to the component that made it and to one router at a time.
/// [`Cx::replace_router`](crate::Cx::replace_router) passes it from a router
/// to the router that replaces it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RouterId {
    /// The component that made the id.
    owner: NodeId,
    /// The bits above the entry key in every saved key of the router.
    base: u64,
}

impl fmt::Debug for RouterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple = f.debug_tuple("RouterId");
        if self.is_named() {
            let name = self.base >> saved_key::NAME_SHIFT;
            tuple.field(&format_args!("named {name:#x}"));
        } else {
            tuple.field(&(self.base >> saved_key::ROUTER_SHIFT));
        }
        tuple.finish()
    }
}

impl RouterId {
    /// Returns the id for the `number`th router of `owner`, from zero.
    pub(crate) const fn numbered(owner: NodeId, number: u16) -> Self {
        Self {
            owner,
            base: saved_key::numbered_router(number),
        }
    }

    /// Returns the id of a router of `owner` named by the FNV-1a `hash` of its name.
    pub(crate) const fn named(owner: NodeId, hash: u64) -> Self {
        Self {
            owner,
            base: saved_key::named_router(hash),
        }
    }

    const fn is_named(self) -> bool {
        saved_key::is_named_router(self.base)
    }

    /// Returns an id of `owner` whose router saves nothing.
    pub(crate) const fn unsaved(owner: NodeId) -> Self {
        Self::numbered(owner, saved_key::UNSAVED_ROUTER)
    }

    pub(crate) const fn owner(self) -> NodeId {
        self.owner
    }

    pub(crate) const fn base(self) -> u64 {
        self.base
    }

    /// Returns whether this router's entries are saved.
    #[must_use]
    pub const fn is_saved(self) -> bool {
        self.base != saved_key::UNSAVED
    }

    /// Returns the lowest and highest saved key an entry of this router can have.
    #[cfg(feature = "serde")]
    pub(crate) const fn saved_keys(self) -> (u64, u64) {
        saved_key::router_entries(self.base)
    }

    /// Returns the saved key of the entry `entry` of this router.
    ///
    /// An entry of a named router whose key does not fit in 32 bits saves nothing.
    pub(crate) const fn saved_key(self, entry: u64) -> u64 {
        saved_key::router_entry(self.base, entry)
    }
}

/// No honest counter gets this far, and below it a counter cannot wrap.
#[cfg(feature = "serde")]
const KEY_LIMIT: u64 = 1 << 47;

/// Makes entry keys read from saved state safe to use.
///
/// Saved bytes can be damaged, so keys may repeat and the counter may be lower
/// than a key already in use. A repeated key is replaced by a new one, and the
/// returned counter is above every key. Keys too large to be real are all
/// replaced, which only costs those entries their saved state.
#[cfg(feature = "serde")]
pub(crate) fn repair_keys<'a>(keys: impl Iterator<Item = &'a mut u64>, counter: u64) -> u64 {
    let mut keys: Vec<&mut u64> = keys.collect();
    let highest = keys.iter().map(|key| **key).max();
    if counter >= KEY_LIMIT || highest.is_some_and(|key| key >= KEY_LIMIT) {
        let mut next = 0;
        for key in &mut keys {
            **key = next;
            next += 1;
        }
        return next;
    }
    let mut next = highest.map_or(counter, |key| counter.max(key + 1));
    let mut seen = HashSet::with_capacity(keys.len());
    for key in &mut keys {
        if !seen.insert(**key) {
            **key = next;
            next += 1;
        }
    }
    next
}

/// Entry components whose scope was dropped, each with the component that made
/// it. The tree removes them when the operation ends.
pub(crate) type Graveyard = Rc<RefCell<Vec<(NodeId, NodeId)>>>;

/// The components one router entry created.
///
/// Holding a scope is how a router addresses an entry without knowing what it
/// contains. The usual single component needs no allocation.
///
/// Dropping a scope that [`Cx::scoped`](crate::Cx::scoped) made removes its
/// components when the running operation ends, as
/// [`Cx::destroy_scope`](crate::Cx::destroy_scope) would at once. A router that
/// is dropped therefore takes its entries with it.
#[derive(Debug, Default)]
pub struct Scope {
    first: Option<NodeId>,
    rest: Vec<NodeId>,
    /// The component that made the entry, and where its components go when the
    /// scope is dropped. Absent on a scope made with `default`.
    grave: Option<(NodeId, Graveyard)>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let Some((owner, graveyard)) = &self.grave else {
            return;
        };
        // The tree borrows the graveyard only to swap it out, while no user code
        // runs, so this borrow always succeeds.
        if let Ok(mut buried) = graveyard.try_borrow_mut() {
            let ids = self.first.iter().chain(self.rest.iter());
            buried.extend(ids.map(|id| (*owner, *id)));
        }
    }
}

impl Scope {
    pub(crate) fn new(owner: NodeId, graveyard: &Graveyard) -> Self {
        Self {
            first: None,
            rest: Vec::new(),
            grave: Some((owner, Rc::clone(graveyard))),
        }
    }

    pub(crate) fn add(&mut self, id: NodeId) {
        if self.first.is_none() {
            self.first = Some(id);
        } else {
            self.rest.push(id);
        }
    }

    /// Returns the components in creation order.
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.first.iter().copied().chain(self.rest.iter().copied())
    }
}

//! What the tree records for each component.

use super::Route;
use crate::NodeId;
use crate::Phase;
use crate::component::node::Node;
use alloc::boxed::Box;
#[cfg(feature = "std")]
use alloc::vec::Vec;

/// The router entry whose children are being created.
#[derive(Clone, Copy)]
pub(crate) struct OpenScope {
    pub(super) owner: NodeId,
    pub(super) base: u64,
    pub(super) next_child: u32,
}

pub(crate) struct NodeSlot {
    pub(crate) generation: u32,
    pub(crate) node: Option<Box<dyn Node>>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) route: Option<Route>,
    /// Identifies the component among its siblings in saved state.
    pub(crate) key: u64,
    /// The creation order within a router entry, for a component made by one.
    #[cfg_attr(
        not(feature = "serde"),
        expect(dead_code, reason = "only persistence reads keys")
    )]
    pub(crate) child: Option<u32>,
    pub(super) next_auto_key: u64,
    pub(crate) next_router: u16,
    pub(super) assigned: Phase,
    pub(super) effective: Phase,
    pub(crate) first_child: Option<NodeId>,
    pub(super) last_child: Option<NodeId>,
    pub(super) prev_sibling: Option<NodeId>,
    pub(crate) next_sibling: Option<NodeId>,
    /// The child granted `Visible` or above the longest ago, where a rising
    /// phase starts.
    pub(super) first_visible: Option<NodeId>,
    /// The child granted `Visible` or above most recently, where Back and a
    /// falling phase start.
    pub(super) last_visible: Option<NodeId>,
    pub(super) prev_visible: Option<NodeId>,
    pub(super) next_visible: Option<NodeId>,
    /// Whether this component's own `back` answered, as of its last run.
    pub(super) own_back: bool,
    /// How many visible children have a Back handler somewhere below them.
    pub(super) live_back: u32,
    pub(super) dirty: bool,
    #[cfg(feature = "serde")]
    pub(crate) saver: Option<crate::persist::Saver>,
    #[cfg(feature = "std")]
    pub(crate) tasks: Vec<(u64, crate::TaskGuard)>,
    /// Tasks that are not `Send`, which the tree polls itself.
    #[cfg(feature = "std")]
    pub(crate) local: Vec<crate::concurrency::local::LocalTask>,
}

impl NodeSlot {
    /// Returns the slot of a new component, with no value and no children yet.
    ///
    /// `child` is the component's place in a router entry, if it belongs to one.
    pub(super) fn new(
        generation: u32,
        parent: Option<NodeId>,
        route: Option<Route>,
        key: u64,
        child: Option<u32>,
    ) -> Self {
        Self {
            generation,
            node: None,
            parent,
            route,
            key,
            child,
            next_auto_key: 0,
            next_router: 0,
            // A router grants phases to its entries. Any other child follows its parent.
            assigned: if child.is_some() {
                Phase::Hidden
            } else {
                Phase::Focused
            },
            effective: Phase::Hidden,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            first_visible: None,
            last_visible: None,
            prev_visible: None,
            next_visible: None,
            own_back: false,
            live_back: 0,
            dirty: false,
            #[cfg(feature = "serde")]
            saver: None,
            #[cfg(feature = "std")]
            tasks: Vec::new(),
            #[cfg(feature = "std")]
            local: Vec::new(),
        }
    }
}

// Nine links are `Option<NodeId>`, 8 bytes each because `NodeId` has a niche.
// The size on 64-bit targets for each set of features. Growing it must be a
// deliberate change.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(
    core::mem::size_of::<NodeSlot>()
        == match (cfg!(feature = "std"), cfg!(feature = "serde")) {
            (true, true) => 200,
            (true, false) => 192,
            (false, true) => 152,
            (false, false) => 144,
        }
);

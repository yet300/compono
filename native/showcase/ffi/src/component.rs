//! What every exported component shares: a weak link to the tree and its address.

use boltffi::data;
use compono::{Component, NodeId, Tree};
use compono_ffi::{NativeError, NativeTree, phase_code};
use std::sync::{Arc, Weak};

/// One child of a router.
#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChildRef {
    /// Identifies this entry for as long as it exists.
    pub key: i64,
    /// The component's address.
    pub node: i64,
    /// 0 for hidden, 1 for visible and 2 for focused.
    pub phase: i32,
}

/// Returns the value of a tree call, or `None` when the tree or component is gone.
///
/// A native UI meets a missing component in normal use, for example when a tap
/// reaches a component that was just removed. Any other error is a bug in the
/// native caller, such as a call from another thread, so a debug build stops at it.
#[track_caller]
pub(crate) fn checked<T>(result: Result<T, NativeError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            debug_assert!(!error.is_misuse(), "native call failed: {error}");
            None
        }
    }
}

/// The tree, held weakly so that a component object never keeps it alive.
///
/// A component object that outlives its application answers with nothing.
#[derive(Clone, Debug)]
pub(crate) struct Ctx(Weak<NativeTree>);

impl Ctx {
    pub(crate) fn new(tree: &Arc<NativeTree>) -> Self {
        Self(Arc::downgrade(tree))
    }

    pub(crate) fn tree<R>(&self, read: impl FnOnce(&Tree) -> Option<R>) -> Option<R> {
        checked(self.0.upgrade()?.read(read)).flatten()
    }

    pub(crate) fn read<C: Component, R>(&self, node: u64, read: impl FnOnce(&C) -> R) -> Option<R> {
        self.tree(|tree| {
            let handle = tree.handle_of::<C>(NodeId::from_bits(node))?;
            tree.get(handle).map(read)
        })
    }

    pub(crate) fn send<C: Component>(&self, node: u64, msg: C::Msg) {
        if let Some(tree) = self.0.upgrade() {
            checked(tree.send::<C>(node, msg));
        }
    }

    pub(crate) fn child(&self, key: u64, node: NodeId) -> Option<ChildRef> {
        self.tree(|tree| {
            let phase = tree.node_phase(node)?;
            Some(ChildRef {
                key: key.cast_signed(),
                node: node.to_bits().cast_signed(),
                phase: i32::from(phase_code(phase)),
            })
        })
    }
}

pub(crate) const fn address(child: ChildRef) -> u64 {
    child.node.cast_unsigned()
}

pub(crate) fn to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub(crate) fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

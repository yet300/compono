use crate::NodeId;
use core::task::Waker;

/// What one finished operation changed.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Commit<'a> {
    /// Components whose state may have changed, each listed once.
    pub dirty: &'a [NodeId],
    /// Whether a Back press would currently be handled.
    pub can_back: bool,
}

/// Something a component asked for that works but saves nothing, reported to
/// [`Host::diagnostic`].
///
/// None of these is an error: the component goes on as usual, and only its
/// saved state is lost on the next launch. Each names the component that asked,
/// as `owner`. Applications log them in debug builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Diagnostic<'a> {
    /// The component made more numbered routers than
    /// [`RouterId`](crate::router::RouterId) has saved ids for, and this one saves
    /// nothing.
    RouterLimit {
        /// The component that made the router.
        owner: NodeId,
    },
    /// The component gave a router name it had already given out, and this
    /// router saves nothing.
    RepeatedRouterName {
        /// The component that named the router.
        owner: NodeId,
    },
    /// A child was named with a name a live sibling already uses, and it saves
    /// nothing.
    RepeatedChildName {
        /// The parent of both children.
        owner: NodeId,
        /// The repeated name.
        name: &'a str,
    },
}

/// The platform side of a [`Tree`](crate::Tree).
///
/// A host receives no reference to the tree, so it cannot reenter it. It records
/// what happened and reads the tree on its next turn.
pub trait Host {
    /// Called once after each operation that changed something.
    fn commit(&mut self, commit: &Commit<'_>);

    /// Returns the waker used when work arrives from another thread, or when an
    /// operation stops at [`Tree::set_work_limit`](crate::Tree::set_work_limit)
    /// with work left.
    ///
    /// The host answers it by calling [`Tree::pump`](crate::Tree::pump) on the
    /// tree's own thread.
    fn waker(&self) -> Waker {
        Waker::noop().clone()
    }

    /// Receives a [`Diagnostic`] when a component asks for something that works
    /// but saves nothing. Called during the operation, before its commit.
    ///
    /// Does nothing by default. Applications log diagnostics in debug builds.
    fn diagnostic(&mut self, diagnostic: &Diagnostic<'_>) {
        let _ignored = diagnostic;
    }
}

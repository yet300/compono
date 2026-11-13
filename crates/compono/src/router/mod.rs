//! Navigation models: which children exist and which phase each is granted.

pub mod items;
pub mod link;
pub mod pages;
pub mod panels;
pub(crate) mod scope;
pub mod set;
pub mod slot;
pub mod stack;

pub use scope::{RouterId, Scope};

use crate::{Component, Cx};

/// What every router offers its owner `P`, built-in or custom.
///
/// An owner replaces a router with [`Cx::replace_router`], which needs both
/// methods: it destroys the old router and gives its ids to the new one.
pub trait Router<P: Component> {
    /// Returns the ids this router took from [`Cx::router`], in the order it
    /// took them.
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId>;

    /// Removes the router with every component it holds, at once.
    ///
    /// A router is normally a field that lives as long as its owner, and its
    /// components go when the owner does. Dropping the router earlier removes
    /// them when the running operation ends, and this removes them at once.
    fn destroy(self, cx: &mut Cx<'_, P>);
}

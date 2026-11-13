//! The router names each component has given out.

use super::Tree;
use crate::NodeId;
use crate::router::RouterId;

impl Tree {
    /// Gives the named id `id` to a new router of `owner`, unless `owner` has
    /// given it out before. A name given out again gives an id that saves
    /// nothing, so it cannot take or overwrite the first router's state.
    pub(crate) fn name_router(&mut self, owner: NodeId, id: RouterId) -> RouterId {
        let names = self.router_names.entry(owner).or_default();
        if names.contains(&id.base()) {
            self.diagnose(&crate::Diagnostic::RepeatedRouterName { owner });
            return RouterId::unsaved(owner);
        }
        names.push(id.base());
        id
    }
}

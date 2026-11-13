use crate::TreeView;
use compono::{Component, Handle, NodeId, Tree};
use gpui::prelude::*;
use gpui::{AnyElement, AnyView, Context, EntityId, WeakEntity, Window, div};
use std::collections::HashMap;

/// Builds the element for one component. It runs with the owner of the tree.
pub type Draw<V, C> = fn(&mut V, Handle<C>, &mut Context<'_, V>) -> AnyElement;

struct ComponentView<V, C> {
    owner: WeakEntity<V>,
    handle: Handle<C>,
    draw: Draw<V, C>,
}

impl<V: TreeView, C: Component> Render for ComponentView<V, C> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let (handle, draw) = (self.handle, self.draw);
        // GPUI renders a child only after its parent's `render` has returned,
        // so the owner is free here. A released owner has nothing to draw.
        self.owner
            .update(cx, |owner, cx| draw(owner, handle, cx))
            .unwrap_or_else(|_released| div().into_any_element())
    }
}

/// The GPUI views that each draw one component.
///
/// This is what makes `Commit::dirty` matter: a component with a view redraws
/// alone, and its cached siblings are reused.
#[derive(Default)]
pub struct ComponentViews {
    views: HashMap<NodeId, AnyView>,
}

impl std::fmt::Debug for ComponentViews {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentViews")
            .field("views", &self.views.len())
            .finish()
    }
}

impl ComponentViews {
    /// Returns the view of a component, creating it on first use.
    ///
    /// Use `.cached(style)` on the result where the style fully sizes the view.
    pub fn view<V: TreeView, C: Component>(
        &mut self,
        handle: Handle<C>,
        draw: Draw<V, C>,
        cx: &mut Context<'_, V>,
    ) -> AnyView {
        let owner = cx.weak_entity();
        self.views
            .entry(handle.id())
            .or_insert_with(|| {
                AnyView::from(cx.new(|_cx| ComponentView {
                    owner,
                    handle,
                    draw,
                }))
            })
            .clone()
    }

    /// Forgets views whose component no longer exists.
    pub(crate) fn retain_live(&mut self, tree: &Tree) {
        self.views.retain(|id, _view| tree.contains(*id));
    }

    pub(crate) fn entity_of(&self, id: NodeId) -> Option<EntityId> {
        self.views.get(&id).map(AnyView::entity_id)
    }
}

use crate::{Changes, ComponentViews};
use compono::{Component, Handle, NodeId, Tree, WakeSignal};
use gpui::{App, Context, EntityId, Task, Window};

/// A GPUI view that owns a component tree.
pub trait TreeView: Sized + 'static {
    /// Returns the tree.
    fn tree_mut(&mut self) -> &mut Tree;

    /// Returns the record created by [`GpuiHost::changes`](crate::GpuiHost::changes).
    fn changes(&self) -> &Changes;

    /// Returns the views drawing individual components.
    fn views_mut(&mut self) -> &mut ComponentViews;
}

/// Runs `operation` on the tree and redraws what changed.
///
/// A component with its own view redraws alone. Any other change redraws `view`.
pub fn update<V: TreeView, R>(
    view: &mut V,
    cx: &mut Context<'_, V>,
    operation: impl FnOnce(&mut Tree) -> R,
) -> R {
    let value = operation(view.tree_mut());
    let dirty = view.changes().take();
    if dirty.is_empty() {
        return value;
    }
    let mut redraw_owner = false;
    for id in dirty {
        match nearest_view(view, id) {
            Some(entity) => App::notify(cx, entity),
            None => redraw_owner = true,
        }
    }
    if redraw_owner {
        cx.notify();
    }
    let mut views = core::mem::take(view.views_mut());
    views.retain_live(view.tree_mut());
    *view.views_mut() = views;
    value
}

/// Finds the view that draws a component: its own, or its nearest ancestor's.
fn nearest_view<V: TreeView>(view: &mut V, id: NodeId) -> Option<EntityId> {
    let mut cursor = Some(id);
    while let Some(id) = cursor {
        if let Some(entity) = view.views_mut().entity_of(id) {
            return Some(entity);
        }
        cursor = view.tree_mut().parent(id);
    }
    None
}

/// Delivers work from other threads until the view is released.
///
/// Keep the returned task in the view. Dropping it stops delivery.
pub fn listen<V: TreeView>(signal: WakeSignal, cx: &mut Context<'_, V>) -> Task<()> {
    cx.spawn(async move |view, cx| {
        loop {
            signal.wait().await;
            let delivered = view.update(cx, |view, cx| {
                update(view, cx, |tree| {
                    // A poisoned queue only happens after a panic on another thread.
                    let _closed = tree.pump();
                });
            });
            if delivered.is_err() {
                return;
            }
        }
    })
}

/// Creates an event listener that sends `msg` to `target`.
pub fn send<V, C, E>(
    target: Handle<C>,
    msg: C::Msg,
    cx: &Context<'_, V>,
) -> impl Fn(&E, &mut Window, &mut App) + 'static
where
    V: TreeView,
    C: Component,
    C::Msg: Clone,
    E: 'static,
{
    cx.listener(move |view, _event: &E, _window, cx| {
        update(view, cx, |tree| {
            // A frame can outlive the component it was built from by one event.
            let _stale_is_fine = tree.send(target, msg.clone());
        });
    })
}

use crate::{TreeView, update};
use compono::Phase;
use gpui::{Context, Subscription, Window};

/// Returns how present the window is, for [`Tree::set_phase`](compono::Tree::set_phase).
///
/// GPUI 0.2 does not report minimizing, so this is `Visible` or `Focused`.
#[must_use]
pub fn window_phase(window: &Window) -> Phase {
    if window.is_window_active() {
        Phase::Focused
    } else {
        Phase::Visible
    }
}

/// Keeps the tree's phase equal to the window's. Keep the returned subscription.
pub fn follow_window<V: TreeView>(window: &mut Window, cx: &mut Context<'_, V>) -> Subscription {
    cx.observe_window_activation(window, |view, window, cx| {
        let phase = window_phase(window);
        update(view, cx, |tree| tree.set_phase(phase));
    })
}

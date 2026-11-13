use crate::ui;
use compono::{Handle, Tree};
use compono_gpui::{
    Changes, ComponentViews, GpuiHost, GpuiSpawner, TreeView, follow_window, is_back_button,
    is_back_key, listen, update, window_phase,
};
use gpui::prelude::*;
use gpui::{Context, FocusHandle, KeyDownEvent, MouseDownEvent, Subscription, Task, Window, div};
use showcase::Services;
use showcase::feature::root::Root;

/// The application window's view, which owns the component tree.
pub struct ShowcaseView {
    tree: Tree,
    root: Handle<Root>,
    changes: Changes,
    views: ComponentViews,
    focus: FocusHandle,
    _wakes: Task<()>,
    _window_focus: Subscription,
}

impl std::fmt::Debug for ShowcaseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShowcaseView").finish_non_exhaustive()
    }
}

impl TreeView for ShowcaseView {
    fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }

    fn changes(&self) -> &Changes {
        &self.changes
    }

    fn views_mut(&mut self) -> &mut ComponentViews {
        &mut self.views
    }
}

impl ShowcaseView {
    /// Builds the tree, resuming `saved` from [`ShowcaseView::save`] when it is given.
    pub fn new(
        window: &mut Window,
        cx: &mut Context<'_, Self>,
        services: Services,
        saved: Option<&[u8]>,
    ) -> Self {
        let host = GpuiHost::new();
        let signal = host.signal();
        let changes = host.changes();
        let mut tree = showcase::saved::tree(host, saved);
        tree.set_spawner(GpuiSpawner::new(cx.background_executor().clone()));
        tree.set_phase(window_phase(window));
        let root = tree.mount_persistent(move |cx| Root::new(cx, services));
        Self {
            tree,
            root,
            changes,
            views: ComponentViews::default(),
            focus: cx.focus_handle(),
            _wakes: listen(signal, cx),
            _window_focus: follow_window(window, cx),
        }
    }

    /// Encodes the tree for the next launch.
    #[must_use]
    pub fn save(&self) -> Option<Vec<u8>> {
        showcase::saved::save(&self.tree)
    }

    /// Returns the tree, for tests.
    #[must_use]
    pub const fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Returns the root component.
    #[must_use]
    pub const fn root(&self) -> Handle<Root> {
        self.root
    }

    /// Returns the tree to read and the views to create, as separate borrows.
    pub(crate) const fn parts(&mut self) -> (&Tree, &mut ComponentViews) {
        (&self.tree, &mut self.views)
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<'_, Self>) {
        if is_back_key(&event.keystroke) {
            self.back(cx);
        }
    }

    fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if is_back_button(event.button) {
            self.back(cx);
        }
    }

    fn back(&mut self, cx: &mut Context<'_, Self>) {
        update(self, cx, |tree| {
            let _handled = tree.back();
        });
    }
}

impl Render for ShowcaseView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // Clicking a plain element clears focus, and keys need it back.
        if !self.focus.is_focused(window) {
            self.focus.focus(window, cx);
        }
        let root = self.root;
        let screen = self.views.view(root, ui::root, cx).cached(ui::fill());
        div()
            .size_full()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .on_any_mouse_down(cx.listener(Self::mouse_down))
            .child(screen)
    }
}

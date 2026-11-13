# GPUI

`compono-gpui` builds on `gpui-pre`, the published GPUI snapshot that
`gpui-component` uses. Import it under the name `gpui`:

```toml
gpui = { package = "gpui-pre", version = "=0.3.12" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.12", features = ["font-kit", "wayland", "x11", "runtime_shaders"] }
gpui-component = "0.9.1"
```

`runtime_shaders` compiles shaders at startup, so the build needs no Xcode Metal
toolchain; leave it out of release builds. `gpui-pre-macos` needs Rust 1.99 or
later.

## The view that owns the tree

One GPUI view owns the `Tree` and implements `TreeView`. Every change goes
through `compono_gpui::update`, which redraws only the views of components that
changed.

```rust,ignore
use compono::{Handle, Tree};
use compono_gpui::{
    Changes, ComponentViews, GpuiHost, GpuiSpawner, TreeView, follow_window, is_back_button,
    is_back_key, listen, update, window_phase,
};

pub struct AppView {
    tree: Tree,
    root: Handle<Root>,
    changes: Changes,
    views: ComponentViews,
    focus: FocusHandle,
    _wakes: Task<()>,
    _window_focus: Subscription,
}

impl TreeView for AppView {
    fn tree_mut(&mut self) -> &mut Tree { &mut self.tree }
    fn changes(&self) -> &Changes { &self.changes }
    fn views_mut(&mut self) -> &mut ComponentViews { &mut self.views }
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<'_, Self>, saved: Option<&[u8]>) -> Self {
        let host = GpuiHost::new();
        let (signal, changes) = (host.signal(), host.changes());
        let mut tree = match saved {
            Some(bytes) => Tree::with_saved(host, bytes, SCHEMA).0,
            None => Tree::new(host),
        };
        tree.set_spawner(GpuiSpawner::new(cx.background_executor().clone()));
        tree.set_phase(window_phase(window));
        let root = tree.mount_persistent(|cx| Root::new(cx));
        Self {
            tree, root, changes,
            views: ComponentViews::default(),
            focus: cx.focus_handle(),
            _wakes: listen(signal, cx),             // work from other threads
            _window_focus: follow_window(window, cx), // phase follows the window
        }
    }

    /// Splits the borrows a view function needs.
    pub(crate) fn parts(&mut self) -> (&Tree, &mut ComponentViews) {
        (&self.tree, &mut self.views)
    }
}
```

In `render`, track `self.focus`, and on `on_key_down` / `on_any_mouse_down` call
`update(self, cx, |tree| { let _ = tree.back(); })` when `is_back_key(&event.keystroke)`
or `is_back_button(event.button)` is true. Open the window with
`gpui_platform::application()`, call `gpui_component::init(cx)` first, and wrap
the view in `gpui_component::Root::new(view, window, cx)`.

## Views

A view function has the type `Draw<AppView, C>`:
`fn(&mut AppView, Handle<C>, &mut Context<'_, AppView>) -> AnyElement`.
Give each component with state its own GPUI view with
`views.view(handle, draw_fn, cx)`; then a change redraws only that view.

```rust,ignore
pub fn view(owner: &mut AppView, handle: Handle<Counters>, cx: &mut Context<'_, AppView>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(counters) = tree.get(handle) else { return div().into_any_element() };
    let screen = counters.stack.view(|child| {
        views.view(*child, counter::view, cx).cached(fill()).into_any_element()
    });
    v_flex()
        .child(Button::new("first").label("Back to first")
            .on_click(send(handle, CountersMsg::PopToRoot, cx)))
        .child(screen)
        .into_any_element()
}
```

- `send(handle, msg, cx)` returns a handler for any event type, so it fits
  `on_click`, `TabBar` tabs and `Switch::on_click`.
- `.cached(style)` reuses a child view's last frame when the style fully sizes it.
- `ListView::new(id, items.len(), handle, Msg::Window).window(items.window()).view(cx, |owner, index, cx| ...)`
  returns a `uniform_list`. Pass `.window(...)`, or every draw reports a window.
  Rows have no views of their own.
- GPUI reports no minimise event, so the phase is `Visible` or `Focused`.
- Router views animate with `compono_motion::Motion::desktop()`, a 150 ms fade;
  `compono_gpui::set_motion(Motion::web())` changes it for the whole application. A `Stack` moves its
  new entry in, a `Slot` fades in, `Pages` slide by page order and `Panels` in
  `PanelsMode::Single` slide the deepest panel in. An entry that leaves disappears at
  once, because GPUI cannot redraw a removed component.
  GPUI applies offsets and opacity, not scale. GPUI's `cx.set_reduce_motion(true)`
  makes every router fade in place.

## Testing

GPUI's test platform drives real views: `cx.update(gpui_component::init)`, then
`cx.add_window_view(...)`, `cx.debug_bounds(selector)` and `cx.simulate_click`.
Give clickable widgets a `debug_selector` with their label.

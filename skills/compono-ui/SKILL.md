---
name: compono-ui
description: Draws Compono components with a Rust UI toolkit and runs the tree in a desktop or web app. Use when connecting a Compono tree to egui, eframe, iced or GPUI, drawing routers with RouterView or ListView, wiring Back, window focus and saved state on desktop, or running in a browser with compono-web.
---

# Compono in a Rust UI

The components know nothing about the UI. A view reads a component through
`&Tree` and sends it messages. Each adapter crate provides the same three things:

- a **host** that wakes the UI when the tree changes;
- `RouterView`, so `router.view(...)` draws whatever a router shows;
- `ListView`, a scrolling list that reports the rows on screen, plus spare rows
  on each side (`.overscan(n)`, 10 by default), to an `Items`.

| Toolkit | Crate | How a view sends | Reference |
|---|---|---|---|
| egui, eframe | `compono-egui` | `out.send(handle, msg)` into an `Outbox`, flushed after the frame | [references/egui.md](references/egui.md) |
| iced | `compono-iced` | `send(handle, msg)` returns an `Action` message | [references/iced.md](references/iced.md) |
| GPUI | `compono-gpui` | `send(handle, msg, cx)` returns a click handler | [references/gpui.md](references/gpui.md) |

Read the reference for the toolkit in use before writing code.

## Drawing routers

Every adapter gives the routers a `view` method with the same shape. egui also
passes `ui`:

```rust,ignore
root.stack.view(|child| match *child {        // Stack: always one child
    Child::List(list) => list_view(tree, list),
    Child::Details(details) => details_view(tree, details),
});
menu.dialog.view(|dialog| dialog_view(tree, *dialog)); // Slot, Pages: Option
articles.panels.view(|panel| match panel {      // Panels: once per visible panel
    Panel::Main(list) => list_view(tree, *list),
    Panel::Details(details) => details_view(tree, *details),
    Panel::Extra(author) => author_view(tree, *author),
});
ListView::new(&feed.items, handle, FeedMsg::Window)  // Items
    .row_height(32.0)
    .view(|index, row| row_view(tree, index, row));  // row is None until it exists
```

## Project layout

Mirror the component folders in the UI, one file per component:

```text
src/feature/articles/mod.rs      Articles (owns Panels)
src/feature/articles/list.rs     ArticleList
src/ui/articles/mod.rs           draws Articles
src/ui/articles/list.rs          draws ArticleList
src/ui/widgets.rs                buttons, cards, titles shared by every screen
```

A view takes the component's `Handle`, reads it with `tree.get(handle)`, and
returns nothing when that is `None`. It holds no state of its own beyond what the
toolkit needs for widgets (scroll offsets, text input). Use the toolkit's
standard widget library: egui's widgets, iced with `iced_aw`, GPUI with
`gpui-component`.

## Desktop

- **Phase.** Follow the window: `Focused` when it has focus, `Visible` when shown
  without focus, `Hidden` when minimised. Each adapter has a helper.
- **Back.** `compono-desktop` knows each OS's Back shortcuts and mouse buttons.
  The adapters wrap it; call `tree.back()` when one fires.
- **Saved state.** `compono_desktop::StateFile::in_app_dir("my-app")` gives a file
  in the OS's app-data folder. Load it before building the tree, pass the bytes to
  `Tree::with_saved(host, &bytes, SCHEMA)`, and store `tree.save(SCHEMA)` when
  the window closes.
- **Tasks.** `compono_tokio::TokioSpawner::new(runtime.handle().clone())` runs
  `cx.perform` on Tokio. GPUI uses `compono_gpui::GpuiSpawner` instead. Create a
  Tokio timer inside the future, or enter the runtime first, because the UI
  thread has no runtime context.

```rust,ignore
let state = StateFile::in_app_dir("my-app").ok();
let saved = state.as_ref().and_then(|state| state.load().ok().flatten());
let mut tree = match saved {
    Some(bytes) => Tree::with_saved(host, &bytes, SCHEMA).0, // other schemas start clean
    None => Tree::new(host),
};
tree.set_spawner(TokioSpawner::new(runtime.handle().clone()));
let root = tree.mount_persistent(|cx| Root::new(cx));
// On close:
if let (Some(state), Ok(bytes)) = (&state, tree.save(SCHEMA)) {
    let _ = state.store(&bytes);
}
```

## Web

`compono-web` runs the same tree in a browser on `wasm32`:

- `WebSpawner` for `cx.perform`, `sleep` for timers, and `cx.perform_local` for
  `fetch` and other futures that are not `Send`.
- `PageEvents` and `page_phase` follow the tab's visibility and focus.
- `SessionStore` keeps saved state across reloads.
- `history::History` mirrors one `Stack` in browser history, so Back, Forward,
  reload and typed addresses work. It uses the `Paths` from
  `compono::router::link`; see the `compono-navigation` skill.

eframe runs the egui UI in a browser unchanged.

## Critical warnings

- Read the tree only through `&Tree` while drawing. Change it only through the
  adapter's send path, after or between frames.
- A view that caches a component's fields shows stale data. Read from the tree
  each frame.
- `ListView` must report its window, or an `Items` creates no rows.
- Without a spawner, `cx.perform` does nothing. Set one before mounting.

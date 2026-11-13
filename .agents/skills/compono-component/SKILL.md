---
name: compono-component
description: Shapes Compono components, routers and showcase screens the way the project requires. Use when adding or changing a component, a router or custom navigation, saved state, or a screen in examples/showcase and its egui, iced and GPUI views.
---

# Writing Compono components

Read `ARCHITECTURE.md` for the model. This skill is the checklist for code.

## A component

- Implement `Component` with its own `Msg` and `Output`. Dependencies are
  constructor arguments, so the parent decides what to pass.
- Talk upward only through `cx.emit(output)`. A child never holds a reference or
  handle to its parent. When a child needs something from above, add an `Output`
  variant and let the parent act.
- Declare Back with `fn back(&self) -> Option<Msg>`, computed from the
  component's own fields only, because the tree caches the answer.
- Keep what every UI shows but does not draw in the component: button lists,
  labels, dialog text. The egui, iced and GPUI views then share it.
- Work off the UI thread goes through `cx.perform(future, to_msg)` and is
  cancelled with the component. Timers come from an injected service such as
  `showcase::Services`, so tests can run them on `compono_test::VirtualTime`.

## Routers and navigation

- A router is a field of its parent: `Stack`, `Slot`, `Pages`, `Panels`, `Items`
  or `ChildSet`. The value the UI matches on is usually an enum of handles.
- A new router or custom navigation takes `cx.router()` once and passes it to
  `cx.scoped`, so saved keys stay apart.
- Name every router with saved state: `cx.named_routers("tabs", |cx| Stack::restore_or(…))`.
  A named router finds its state by name. An unnamed one saves under its place in
  construction order, which a router added before it in a later version shifts.
- To replace a router after construction, hold it in an `Option` and call
  `cx.replace_router(old, |cx| …)`. The new router takes the old one's ids, and with
  them its name and its entries' saved state.
- Use only public `Cx` methods. `examples/showcase/src/feature/gallery/filmstrip.rs`
  is a complete custom router built that way.

## Saved state

- Implement `Persist` with a `KIND` such as `"showcase.counter"`, and create the
  child with `cx.persistent_child`. Read it in the constructor with
  `cx.restored()`.
- Outside router entries, name persistent children with
  `cx.persistent_child_keyed`. Unnamed children are identified by creation order,
  counting every child of the parent.
- When a saved type changes shape, raise its `VERSION` and read the old shape in
  `migrate`, or raise the schema to start every saved state clean. In the
  showcase the schema is `showcase::saved::SCHEMA`, with `SCHEMA` in
  `native/showcase/ffi/src/lib.rs` kept equal.

## Showcase screens

- A feature is a folder, `examples/showcase/src/feature/<feature>/`, with no UI
  dependency. `mod.rs` holds the parent component and re-exports the rest; each
  child component has a file of its own, such as `counters/counter.rs`.
- Each UI mirrors that folder as `src/ui/<feature>/` in `showcase-egui`,
  `showcase-iced` and `showcase-gpui`: `mod.rs` draws the parent and one file draws
  each child. Add all three, and keep logic out of them.
- Use the toolkit's usual widgets: egui's own, iced's with `iced_aw` (tab bar, card,
  badge), GPUI's with `gpui-component` (button, tab bar, switch, progress, group
  box, tag). Shared helpers go in `src/ui/widgets.rs`.
- The native showcase follows the same split. Typed component objects live in
  `native/showcase/ffi/src/feature/<feature>.rs`, and screens in
  `ui/<feature>/` of the Android and iOS apps. A screen takes its component
  object, such as `CountersComponent`, never a bare node address.
- Each showcase has `tests/ui.rs` driving real frames. Extend it for a new screen,
  and keep the texts it searches for.

## Code rules

Library code has no `unwrap`, `expect`, `panic!`, indexing, `unreachable!`,
`todo!` or `unsafe`; `[workspace.lints]` enforces this. Stale handles give
`None` or `Err(StaleHandle)`. One public path per item, no glob re-exports, and
files under about 400 lines.

## Tests

- Core behaviour belongs in `crates/compono/tests`, with a `Host` that ignores
  commits or with `compono_test::record` to inspect them.
- A new `Host` is checked with `compono_test::check_host`.
- Allocation budgets are enforced by `crates/compono/tests/allocations.rs`.
- Write the failing test first, as the `test-driven-development` skill describes.

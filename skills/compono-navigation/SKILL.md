---
name: compono-navigation
description: Builds navigation with Compono routers. Use when choosing or using Stack, Slot, Pages, Panels, Items or ChildSet, saving navigation across restarts, putting several routers in one component, handling deep links with Paths and Navigation, or writing a custom navigation model.
---

# Compono navigation

A **router** is a field of the component that owns it. It creates a child
component for each route it shows, grants each child a `Phase`, and removes
children it no longer needs. Every operation takes the owner's `&mut Cx` and runs
synchronously: when it returns, the new state is readable. Background components
stay alive with their state.

The `compono-components` skill covers components, `Cx` and `Persist`.

## Choosing a router

| Need | Router | Decompose |
|---|---|---|
| Screens pushed and popped, Back pops | `Stack` | `ChildStack` |
| Tabs that keep their state | `Stack` with `bring_to_front` | `ChildStack` with `bringToFront` |
| Zero or one child: dialog, sheet | `Slot` | `ChildSlot` |
| Pager, only neighbours alive | `Pages` | `ChildPages` |
| List, details and extra side by side | `Panels` | `ChildPanels` |
| Long list, components only for rows on screen | `Items` | `ChildItems` |
| Keyed children in any order you choose | `ChildSet` | none |
| Anything else | Custom router, see below | generic navigation |

## Routes

A route is the data that names a screen, such as an enum. It plays the part of a
Decompose configuration.

- Keep routes to plain data: ids, enums, small values. Pass services to the
  component through the `build` closure, not through the route.
- To save navigation, the route must implement `Serialize` and
  `DeserializeOwned`, and `Clone`.
- `Pages`, `Items` and `ChildSet` identify children by route or key, so these
  need `Clone + Eq + Hash`. A repeated key is refused with `DuplicateKey` and
  nothing changes. `Items` checks only the keys inside its window.
- `Stack` entries are visits: pushing the same route twice gives two entries
  with separate state. `push_new`, `push_to_front` and `bring_to_front` compare
  routes and need `PartialEq`.

## Stack

```rust
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    List,
    Details(u32),
}

/// What the UI matches on: one handle type per screen.
#[derive(Clone, Copy, Debug)]
pub enum Child {
    List(Handle<List>),
    Details(Handle<Details>),
}

pub struct Root {
    pub stack: Stack<Screen, Child, Self>,
}

impl Root {
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        // `None` on a first launch: the stack starts at `Screen::List`.
        let saved = cx.restored();
        // The name finds the saved stack even if routers are added before it.
        let stack = cx.named_routers("screens", |cx| {
            Stack::restore_or(saved, Screen::List, cx, |screen, cx| match *screen {
                Screen::List => Child::List(cx.persistent_child(|_cx| List, Self::on_list)),
                Screen::Details(id) => {
                    Child::Details(cx.child(move |_cx| Details { id }, Self::on_details))
                }
            })
        });
        Self { stack }
    }

    fn on_list(&mut self, id: u32, cx: &mut Cx<'_, Self>) {
        self.stack.push(Screen::Details(id), cx);
    }

    fn on_details(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        let _at_root = self.stack.pop(cx);
    }
}

impl Component for Root {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        let _at_root = self.stack.pop(cx);
    }

    fn back(&self) -> Option<()> {
        self.stack.can_pop().then_some(())
    }
}

impl Persist for Root {
    type Saved = SavedStack<Screen>;
    const KIND: &'static str = "app.root";
    fn save(&self) -> SavedStack<Screen> {
        self.stack.snapshot()
    }
}

pub struct List;
impl Component for List {
    type Msg = u32;
    type Output = u32;
    fn update(&mut self, id: u32, cx: &mut Cx<'_, Self>) {
        cx.emit(id);
    }
}
impl Persist for List {
    type Saved = ();
    const KIND: &'static str = "app.list";
    fn save(&self) {}
}

pub struct Details {
    pub id: u32,
}
impl Component for Details {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.emit(());
    }
}
```

- The active entry is `Focused`; the rest are `Hidden` and keep their state.
- `push`, `pop` (`Err(AtRoot)` at the root), `replace`, `pop_to(depth)`,
  `pop_while`, `replace_all`, `reset`, `push_new`, `push_to_front`,
  `bring_to_front`.
- Read with `active()`, `route()`, `depth()`, `can_pop()`, `previous()` (what Back
  reveals) and `entries()`.
- A stack is never empty, so `stack.active()` always returns a child.
- Tabs: one `Stack<Tab, TabChild, Self>` and `bring_to_front(tab, cx)` on
  selection. Each tab's subtree stays alive.

## Slot

```rust
use compono::router::slot::Slot;
use compono::{Component, Cx, Handle};

pub struct Confirm {
    pub text: String,
}
impl Component for Confirm {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.emit(());
    }
}

pub struct Settings {
    pub dialog: Slot<String, Handle<Confirm>, Self>,
}

impl Settings {
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        let dialog = Slot::new(cx, |text: &String, cx| {
            let text = text.clone();
            cx.child(move |_cx| Confirm { text }, Self::on_confirm)
        });
        Self { dialog }
    }

    fn on_confirm(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.dialog.close(cx);
    }
}

impl Component for Settings {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.dialog.show("Delete everything?".to_owned(), cx);
    }

    fn back(&self) -> Option<()> {
        // Back closes the dialog first.
        self.dialog.is_open().then_some(())
    }
}
```

`show` replaces whatever is open. `close` removes it. `active()` is `Option`.
`active_key()` names the open visit, new on every `show`, for UI keys and transitions.
Save with `snapshot()` and `Slot::restore(saved, cx, build)`.

## Pages

```rust,ignore
let pages = Pages::new(routes, selected, retain, cx, build)?; // Err on a repeated route
pages.select(index, cx); // returns whether the selection moved
```

The selected page is `Focused`. Pages within `retain` of it exist and are
`Hidden`. Others have no component. Read `selected()`, `active()`, `routes()`
and `live()`, and `child(index)` for any page that exists. `set_routes` changes the list. Save with `snapshot()` and
`Pages::restore_or(saved, initial, retain, cx, build)`.

## Panels

```rust,ignore
pub struct ArticlePanels;
impl PanelTypes for ArticlePanels {
    type Main = ();
    type MainChild = Handle<ArticleList>;
    type Details = u32;
    type DetailsChild = Handle<ArticleDetails>;
    type Extra = u32;
    type ExtraChild = Handle<Author>;
}

let panels = Panels::new((), PanelsMode::Dual, cx, build_main, build_details, build_extra);
panels.set_details(Some(id), cx); // open or replace details; None closes it
panels.set_extra(Some(author), cx);
panels.set_mode(PanelsMode::Single, cx); // from the window width
```

The deepest open panel is `Focused`. Others that fit the mode are `Visible`, the
rest `Hidden`. `pop` closes the deepest panel and `can_pop` feeds `fn back`.
`visible()` yields `Panel::Main`, `Panel::Details` and `Panel::Extra` for the UI,
and `keys()` the visit key of each open panel.
The UI decides the mode from the window size and sends it in.

## Items

```rust,ignore
let mut items = Items::new(ids, cx, build); // every key known, no component yet
items.set_window(first..last, cx)?; // the UI reports the rows on screen
```

Only rows inside the window have components, and they are `Visible`. Rows that
leave lose their component and state. UI adapters provide a `ListView` that
reports the window as `FeedMsg::Window(range)`; the owner calls `set_window`. The
window is the rows on screen plus `items::OVERSCAN` spare rows on each side, so a
fast scroll finds rows ready; `items::next_window` computes it for a custom list.
`set_keys` replaces the list. Save with `snapshot()` and `Items::restore_or`.

## ChildSet

```rust,ignore
let cards = ChildSet::new(cx, build);
cards.reconcile([(1, Phase::Visible), (2, Phase::Focused)], cx)?;
```

You give the full desired list of `(key, phase)` each time. Keys that stay keep
their component wherever they move, new keys get one, missing keys lose theirs.
It is the base for list-shaped models.

## Several routers in one component

Each router takes a `RouterId` when it is constructed, so routers in one
component keep their saved state apart. Name every router that has saved state,
so it finds that state by its name:

```rust,ignore
let stack = cx.named_routers("stack", |cx| Stack::restore_or(saved.stack, Screen::Home, cx, build));
let sheet = cx.named_routers("sheet", |cx| Slot::restore(saved.sheet, cx, build_sheet));
```

A later version can then construct another router before them, or construct one
only behind a feature flag, and each keeps its own state. The name changes only
where the router saves. Names are unique within one component: a second router
given a name the component already gave out saves nothing.

The host's `diagnostic` method hears of each router that saves nothing:
`Diagnostic::RepeatedRouterName` and `Diagnostic::RouterLimit` past the numbered
ids below. Log them in debug builds, so a lost saved state shows up during development.

A router without a name is numbered in construction order instead. Construct
unnamed routers in the same order on every launch; changing that order is a
saved-state schema change. Save the routers together:

```rust,ignore
#[derive(Serialize, Deserialize)]
pub struct Saved {
    stack: SavedStack<Screen>,
    sheet: SavedSlot<Sheet>,
}
```

A component has a fixed number of numbered router ids that save state, given in
the documentation of `RouterId`. Routers made after those work but save nothing
(`RouterId::is_saved()` is false). A router built inside `update` after every
reset uses them up; build routers in the constructor.

To replace a router after construction, hold it in an `Option` and pass it to
`cx.replace_router`, which destroys it and builds the new one in its ids:

```rust,ignore
let old = self.sheet.take()?;
self.sheet = Some(cx.replace_router(old, |cx| Slot::new(cx, build_sheet)));
```

The new router takes the old one's id or name, and with it the state of entries
the old one restored but never showed.

## Nesting

A child of a router can own routers itself: a `Stack` per tab inside a tab
`Stack`, a `Slot` inside a stack screen. A child's phase never exceeds its
parent's, so a hidden tab hides everything under it. Back goes to the deepest
visible component whose `fn back` answers, then upward.

## Removing a router

A router lives as long as its owner. To discard one earlier, call
`router.destroy(cx)`, a method of the `compono::router::Router` trait, which
removes its components at once. A router that is only dropped takes its
components with it when the running operation ends.

## Deep links

`compono::router::link::Paths` maps routes to paths and back.
`Navigation::toward(&paths, path, &stack)` returns the change that shows a path,
keeping entries the stack already shows. Apply it inside the owner:

```rust,ignore
impl Paths for Addresses {
    type Route = Screen;
    fn path(&self, route: &Screen) -> String { /* "/" or "/articles/3" */ }
    fn routes(&self, path: &str) -> Vec<Screen> { /* oldest first; empty if unknown */ }
}

// From an Android intent, an iOS universal link or a browser address:
if let Some(change) = tree.get(root).and_then(|root| Navigation::toward(&Addresses, path, &root.stack)) {
    tree.send(root, RootMsg::Navigate(change))?;
}
// In Root::update:
RootMsg::Navigate(change) => change.apply(&mut self.stack, cx),
```

`compono-web`'s `History` uses the same `Paths` to mirror a `Stack` in browser
history.

## Custom navigation

Read [references/custom-router.md](references/custom-router.md) when no built-in
router fits.

## Critical warnings

- A route holding a handle, an `Rc` or a service cannot be saved and couples the
  route to runtime state. Keep routes to data.
- Routers constructed in a different order on the next launch receive each
  other's saved state.
- `Pages::new`, `Pages::restore_or` and `Items::restore_or` return
  `Err(DuplicateKey)` for repeated keys. Build a fallback router in that case,
  so the routers after it keep their ids.

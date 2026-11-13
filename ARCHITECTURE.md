# Compono architecture

Compono is the layer between business logic and any UI: a component tree, its
lifecycle, navigation, Back and saved state. The core knows nothing about UI
toolkits, platforms or executors.

## Principles

1. **One owner.** `Tree` owns every component, and no component is shared. The
   core's only `Rc` and `RefCell` hold the addresses that dropped scopes hand
   back to the tree.
2. **A child does not know its parent.** Only a typed `Output` travels upward.
3. **No reentrancy.** A running component is outside the tree, and `Cx` offers no
   path to its parent. This follows from the API and needs no runtime check.
4. **No panics.** Errors and stale addresses are `Result` and `Option`.
5. **Pay for what is on screen.** Cost follows visible components, not stack depth
   or list size.
6. **A nearly empty language boundary.** Numbers and changed models cross it.

## Crate map

```text
crates/
  compono           Tree, Component, Cx, Phase, Handle, Host, Sender, Spawner, Persist, routers
  compono-test      recording Host, virtual time, conformance checks for any Host
  compono-egui      UI adapter: EguiHost, Outbox, BeforeDraw, window phase
  compono-iced      UI adapter: IcedHost, Action, wake and presence subscriptions
  compono-gpui      UI adapter: GpuiHost, TreeView, GpuiSpawner, event listeners
  compono-motion    router transitions: curves, springs, presets, layer values; no_std, no deps
  compono-ffi       native UI boundary: NativeTree, Platform, phase codes
  compono-tokio     Spawner on Tokio
  compono-desktop   saved-state file and the Back shortcuts of each desktop system
  compono-web       browser history for a stack, page lifecycle, session storage, tasks
examples/
  showcase          every navigation model and a 100 000 row load test, with no UI dependency
  showcase-egui     the same screens drawn with egui (also -iced and -gpui)
  showcase-web      showcase-egui in a browser, with history and session storage
native/
  apple/Compono     Swift package: ComponoTree, ChildStack and other router views, lifecycle
  android/compono   Compose library: ComponoTree, ChildStack and other router views, ComponoHost
  showcase          the showcase on SwiftUI and Compose: ffi, ios and android
benchmarks/
  ffi-boundary      BoltFFI against UniFFI on the same scenario
```

Inside `compono`, `component` holds the vocabulary a component uses, `tree` the owner
and its bookkeeping, `router` the navigation models, and `concurrency` everything that
crosses threads. Routers are imported as `compono::router::stack::Stack`.

Planned crates: `compono-dioxus`, `compono-slint`.

All crates share one version. `VERSIONING.md` says what a version number promises and
`CHANGELOG.md` lists what each release changed.

`compono` builds as `no_std + alloc`. The `std` feature adds `Sender` and tasks,
the `serde` feature adds persistence. Both are on by default.

## Component

```rust
pub trait Component: Sized + 'static {
    type Msg: 'static;     // from the UI, the parent and finished tasks
    type Output: 'static;  // to the parent, instead of constructor lambdas

    fn update(&mut self, msg: Self::Msg, cx: &mut Cx<'_, Self>);
    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {}
    fn back(&self) -> Option<Self::Msg> { None }
}
```

Dependencies are ordinary constructor arguments. No DI container is needed: the
parent knows what to pass, and the child declares what it requires.

| Need | Compono |
|---|---|
| Create a child | `cx.child(init, on_output)` returns `Handle<K>`. It follows the parent's phase |
| Tell the parent | `cx.emit(output)` |
| Message a child | `cx.send(handle, msg)`. Anything but itself and its own children is refused with `NotOwnChild` |
| Remove a child | `cx.destroy(handle)`. Anything but its own live children is refused with `NotOwnChild` |
| Say the UI need not change | `cx.unchanged()` |
| Read its own phase | `cx.phase()` |
| Lower a child's phase | `cx.set_child_phase(handle, phase)`, refused with `NotOwnChild` as `destroy` is |
| Work on another thread | `cx.sender()` or `cx.perform(future, to_msg)` |
| Cancel one task | `cx.cancel(task_id)` |

`Handle<C>` is 8 bytes, asserted at compile time, and `Copy + Send + Sync`. So is
`Option<NodeId>`: the generation is kept plus one in a `NonZeroU32`, so the tree's
links to parents, children and siblings cost no extra bytes. After its component is
removed it is stale: reading gives `None`, sending gives `Err(StaleHandle)`. A
reused slot does not revive old handles, because each slot has a generation.

## Execution

Every external operation (`send`, `set_phase`, `back`, `pump`) ends in one
`Commit`:

1. The component is taken out of its slot and runs.
2. Its `emit` and `send` calls are queued in call order.
3. When it returns it goes back in its slot, and the queue is processed.
4. The host receives a `Commit` listing each changed component once. Components
   removed during the operation are not listed.

One operation does at most `Tree::set_work_limit` units of work, 1 048 576 by
default, so components that keep answering each other, or that drop large routers,
cannot hold the thread. Each queued message or output delivered is one unit, and so
is each component removed because its router or `Scope` was dropped. Past the limit
the operation commits what it has done, keeps the rest waiting in order and wakes
the host. An entry the limit stops halfway stays hidden with what is left of it. The
next operation, usually the `pump` that the wake-up asks for, continues after doing
its own first step. A poll runs user code of any length, so polls of `perform_local`
tasks have their own limit, `Tree::set_poll_limit`, 1 024 by default: a `pump` polls
at most that many. The rest wait for the next pump, ahead of tasks woken later, so
a burst of finished tasks spreads over several frames instead of stalling one.

The UI reads through a direct reference: `tree.get(handle) -> Option<&C>`.

`Tree` stays on the thread that created it and is deliberately `!Send`. Other
threads can only enqueue through `Sender`, which refuses messages with
`SendError::Full` past a limit set by `Tree::set_queue_limit`. Task results are never
refused, because running tasks already bound their number. The host is woken through a standard
`core::task::Waker` and calls `tree.pump()` on its own thread. Once the tree is dropped
its queue is closed: waiting messages are dropped and `Sender::send` returns
`SendError::Closed`.

Dropping a `Tree` removes components the way navigation does: children first, each
told it is hidden.

Panics in user code are not caught, except in a task started with `Cx::perform`,
whose panic stops at the task. Applications should use `panic = "abort"`. If
something catches the unwind anyway, the operation it interrupted never finished,
and the tree is poisoned (`Tree::is_poisoned`): it runs no more user code and
commits nothing, while reads and dropping still work. `NativeTree` then answers
`NativeError::Poisoned`. The library cannot rule out out-of-memory. Its own walks over the tree, for phases,
removal, Back and saving, follow the tree's links and do not recurse, so the depth of
a tree is limited only by the constructors that build it.

## Lifecycle

```rust
pub enum Phase { Hidden, Visible, Focused }
```

Creation is the constructor and destruction is `Drop`. There are no subscribers,
only `phase_changed`, called once per change even when going straight from `Hidden`
to `Focused`. A component that does not override it is not marked changed, so a
focus change costs the UI nothing unless a component reacts.

A child's effective phase is the lower of its parent's effective phase and the
phase granted to it. A plain child is granted `Focused`, so it follows its parent.
A router entry starts hidden until its router grants a phase. Going up, parents change before children. Going down,
children change before parents. During navigation the old entry is lowered before
the new one is raised.

Among siblings, the one shown most recently falls first and rises last. A phase
change visits only the children granted `Visible` or above, so covering a component
costs the same however many hidden entries its routers hold.

| Platform | `Hidden` | `Visible` | `Focused` |
|---|---|---|---|
| Android | `onStop` | `onStart`, `onPause` | `onResume` |
| iOS | background | inactive | active |
| Web | `visibilitychange` hidden | visible without focus | visible with focus |
| Desktop | minimized | open without focus | focus |

There is no `InstanceKeeper`. On Android, `retainedTree` keeps the tree in the
Activity's `ViewModelStore`, as Decompose's `retainedComponent` does: it outlives a
configuration change, which does not change the phase, and is closed when the
Activity finishes for good. Its state goes into the Activity's saved state, so it also
survives the process being killed. On Apple systems `ComponoRoot` lives as long as
the process and writes its state to a file when the scene enters the background.
`ComponoTree` holds the bindings weakly, because the bindings hold it as their
listener. `ComponoRoot` keeps both, so releasing a root frees its tree.

## Navigation

A router is a field of its parent. `H` is the value the UI matches on, usually an
enum of handles. A child can also be a plain value with no component, such as a
simple dialog.

```rust
struct Root { stack: Stack<Route, Child, Root> }

self.stack.push(Route::Game { seed }, cx);
let route = self.stack.pop(cx)?;   // Err(AtRoot) instead of a panic at the root

match root.stack.active() {
    Child::Home(home) => home_view(tree.get(*home)),
    Child::Game(game) => game_view(tree.get(*game)),
}
```

`build` is given when the router is created and captures what it needs. It is
allocated once per router. A route `R` stays in Rust and needs `Clone` only when
persistence is used.

A route is data, and runtime dependencies belong in `build`. The compiler checks
this where it matters: a router made with `new` accepts any `'static` route, but a
snapshot can be part of `Persist::Saved` only if the route implements `Serialize`
and `DeserializeOwned`.

| Router | Model | Phases and rules |
|---|---|---|
| `Stack` | Always has an active entry, so an empty stack is unrepresentable. `push`, `push_new`, `push_to_front`, `bring_to_front`, `pop`, `pop_to`, `pop_while`, `replace`, `replace_all`, `reset` | Active is `Focused`, the rest `Hidden` |
| `Slot` | `Option` of one entry | The entry is `Focused` |
| `Pages` | A list and a selected index | Selected is `Focused`, neighbours within `retain` are `Hidden`, others have no component |
| `Panels` | Typed main, details, extra and a `PanelsMode` | Deepest open panel is `Focused`, others that fit are `Visible`, the rest `Hidden` |
| `Items` | Every key known, components only inside the window | Rows on screen are `Visible` |
| `ChildSet` | Keyed children reconciled against a desired list | The caller grants each phase |

A router lives as long as the component that holds it, and its components go when
that component does. A parent that discards a router earlier calls the router's
`destroy(cx)`, which removes its components at once. A router that is only dropped
takes them with it when the running operation ends: each `Scope` it held hands its
components back to the tree as it is dropped, and the tree removes them, hidden
first, before the operation commits.

A router operation changes the router in place. It is not a function from an old
state to a new one. While it runs, the tree creates and removes the components it
needs and tells them their phases, so constructors, `phase_changed` and `Drop` run
inside the operation. What those components ask for does not: their `emit` and `send`
calls wait in the queue until the component that called the router returns, and the
host receives one `Commit` at the end. A `build` closure receives `Cx` and not its
parent, so it cannot reach the router that called it.

Tabs are a `Stack` selected with `bring_to_front`, so each tab keeps its state.
Browser history mirrors one `Stack`, as described under "Web".

### Links

`compono::router::link` maps a `Stack`'s routes to paths such as `/articles/3`, for
browser history and for deep links on Android and iOS. `Paths` gives the path of a
route, and the screens a path opens, oldest first, so a link to `/book` opens the
book above the tabs and Back from it reveals them. `Navigation::toward` turns a path
into the change that shows it, keeping the entries the stack already shows at the
bottom with their state. The component that owns the stack applies it with
`Navigation::apply`, so its own rules still decide.

### Custom navigation

A custom navigation model is ordinary code and has the same power as the built-in
ones, which use nothing else. Decompose's `children(...)` corresponds to:

| Decompose | Compono |
|---|---|
| `NavState` and `navTransformer` | A field of your router, changed by your own methods |
| `ChildNavState(configuration, status)` | `(key, Phase)` |
| `children(...)` | `ChildSet::reconcile(desired, cx)`, or the lower-level calls below |
| `childFactory` | The `build` closure |
| `stateMapper` | Accessors on your router |

```rust
let router = cx.router();                                    // identity in saved state
cx.forget_router(router);                                    // start with no saved entries
let (child, scope) = cx.scoped(router, key, |cx| build(cx)); // create one entry
cx.assign_scope(&scope, Phase::Focused);                     // grant a phase
cx.destroy_scope(&scope);                                    // remove it
```

`examples/showcase/src/feature/gallery/filmstrip.rs` is a complete custom router built this way,
and `examples/showcase/src/feature/cards/mod.rs` is one built on `ChildSet`.

Each router gets a `RouterId` when constructed, so two routers in one parent cannot
collide in saved state. A router built inside `cx.named_routers(name, |cx| …)` is
named: it saves under its name, so routers constructed around it in a later version,
or only behind a feature flag, leave its state alone. Every router that build
constructs takes the name, the first as it is and the next ones followed by their
place, so the three routers of a `Panels` share one name. A name that the component
has already given out gives a router that saves nothing. A router without a name is
numbered in construction order, so unnamed routers must be constructed in the same
order on every launch, and changing that order is a schema change. A named router
takes no number. Children of one entry are identified by their creation order within
it. A component has a fixed number of numbered router ids that save state, given on
`RouterId`. Every unnamed router it makes after those works as usual but saves
nothing (`RouterId::is_saved` is false), so it can never take or overwrite another
router's state.

Every router implements `Router`, which gives its `router_ids` and `destroy`. A
router that replaces another after construction takes the other's ids, because a
relaunch gives the restored router the id from construction order, or the name.
`Cx::replace_router` consumes the old router and builds the new one in its ids, so
two routers never hold one id and replacing a router uses up no ids. Its rustdoc says
which saved state the new router keeps. An id saves only for the component that
made it.

Every saved key is one `u64`. `crates/compono/src/saved_key.rs` lays the keys out,
in a table in its module documentation, and is the only code that builds them.

A constructor that returns an error, such as `Pages::new` or `Items::restore_or`,
takes no `RouterId`. The owner builds another router in its place, and the routers
after it keep their own saved state.

An entry's key belongs to that visit, not to the route. Pushing the same route
twice gives two entries with separate state.

`Pages`, `Items` and `ChildSet` identify a child by its route or key instead, so a
repeat is refused with `DuplicateKey` and nothing changes. `Pages` and `ChildSet`
check the whole list. `Items` checks only the keys inside its window.

`push`, `pop` and `replace` are `O(1)` and never compare routes. Operations that
look for an equal route need `R: PartialEq` and are linear in stack depth. Reconciling a
`ChildSet` is expected `O(n)` in the child count, and an unchanged key list
allocates nothing beyond collecting the input.

Animations live in the UI, driven by its frame clock.

### Motion

`compono-motion` holds what every UI shares about moving between entries, as plain
values and pure functions: easing curves, springs, the transitions and Back
gestures, and a preset per platform (`Motion::material`, `cupertino`, `desktop`,
`web`). It holds no clock, so time and gesture progress are arguments, and it
never allocates. Springs are integrated with fourth-order Runge-Kutta in steps of
one millisecond, because `core` has no `exp`.

`StackChange::between` compares a stack's entry keys before and after a change. It
is backward only when the old active entry is gone and the new one was below it,
so `bring_to_front` and `replace` move forward, as in Decompose. A Rust adapter
keeps one `TransitionState` per router, starts it on a change and draws the
`(key, LayerStyle)` layers that `frame(now)` returns, bottom first.

The Swift package and the Compose library keep their own animation engines and
never call Rust for motion. Their tests compare their values with
`crates/compono-motion/golden/motion.csv`, which the crate's `golden` example
writes and a crate test keeps current.

egui draws the active entry live and replays the other one: each frame the adapter
copies the shapes the active entry painted, and after a change it repaints that
copy with the leaving layer's style. A popped entry's component is gone, so this is
the only way to show it leave. A replayed entry takes no input. The copy covers
every egui layer the entry painted, so a dialog in its own `Modal` layer fades out
with its slot.

Each router has its transition: a `Stack` uses its motion's, a `Slot` fades, `Pages`
slide by page order, and `Panels` slide the deepest panel in when only one fits.
All take the motion's timing.

iced and GPUI cannot draw a component that is gone: an iced `Element` borrows the
tree, and GPUI re-renders views whose bounds move. There, only the entry that
becomes active moves: on a pop the revealed entry comes in and the popped one
disappears at once, and on a push the new entry comes in over the background. The
tree does not keep removed components for them. iced keeps the state in a widget
and takes time from `RedrawRequested`; it has no subtree opacity, so it covers a
fading entry with the theme's background. GPUI keeps it in element state, takes time
from its executor's clock and moves the entry with offsets and opacity; it has no
subtree transform, so scale is not applied.

`MotionPreference` carries the system's reduced-motion setting into a preset:
`Reduced` turns a transition that moves entries into a 150 ms fade and a Back
gesture into a fade of the top entry, and `Off` removes motion. Each UI reads the
setting where it can: `compono_web::prefers_reduced_motion` in a browser, GPUI's
`App::reduce_motion`, SwiftUI's `accessibilityReduceMotion` and Android's animator
duration scale. egui and iced take it from the application.

## Back

A component declares in advance whether it handles Back: `fn back(&self) ->
Option<Msg>`. The platform must know this before a gesture starts. The deepest
visible component handles it, and among siblings the most recently shown one goes
first, and that order survives the whole application being hidden and shown again.
`can_back` arrives in `Commit`.

The tree records each component's answer after it runs and keeps handler counts on
ancestors, so `can_back` costs the same however many components are visible. `back`
must therefore depend only on the component's own fields.

A predictive gesture uses `back_start`, `back_cancel` and `back_commit`. Navigation
does not change before `back_commit`, and the UI reads the component the gesture
aims at with `tree.gesture_target()`. The gesture's progress stays in the UI, as
described under "Language boundary". If the target is destroyed the gesture ends,
and `back_commit` resolves the handler afresh. A gesture also ends when its target
stops being visible, and the end of a gesture always reaches the UI through a
`Commit`.

## Async and effects

The model is React's: logic on one thread, and a task owned by its component and
cancelled with it, like `useEffect` with cleanup and `AbortController`.

```rust
cx.perform(load_scores(repo), Msg::Loaded)?;
```

The result arrives as an ordinary message. A finished task's guard is released when
its message is delivered, so a timer loop does not accumulate state. A task whose
future or `to_msg` panics, in a build that unwinds, reports that it ended without a
message: the next pump releases its guard and delivers nothing. When a
component is removed its guards are dropped, and a late result is discarded because
its generation is stale.

`perform` returns a `TaskId`. `cx.cancel(task)` stops that task and discards its
result even if it has already finished, which covers the "new query replaces the old
request" case that `useEffect` cleanup handles in React.

The core has no executor. A `Spawner` supplies one, and cancellation is `Drop` of
the returned `TaskGuard`:

```rust
pub trait Spawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard;
}
```

A future that is not `Send`, such as a browser `fetch` or a task GPUI and Dioxus run on
their main thread, goes through `cx.perform_local(future, to_msg)`. It needs no
`Spawner`: the tree keeps the future in its component's slot and polls it during
`Tree::pump`. The future's waker records which task to poll and wakes the host, so
nothing that is not `Send` leaves the tree. A task waits for its poll once however
often it is woken, and the host is woken once for it. The pump clears that mark
before the poll, so a wake-up during the poll asks for another. Destroying the component or `cx.cancel`
drops the future, and its result is never delivered.

## Persistence

`serde` is on by default, with `postcard` as the format.

```rust
pub trait Persist: Component {
    type Saved: Serialize + DeserializeOwned;
    const KIND: &'static str = "";
    const VERSION: u32 = 0;
    fn save(&self) -> Self::Saved;
    fn migrate(version: u32, bytes: &[u8]) -> Option<Self::Saved> { None }
}
```

- Create with `cx.persistent_child(...)`, read in the constructor with `cx.restored()`.
- `tree.save(schema)` gives one flat buffer of key path to bytes, without nested frames.
  Each entry records the FNV-1a hash of its component's `KIND` and its `VERSION`,
  inline beside its bytes, so the tag allocates nothing. `cx.restored()` gives
  `None` for state of another kind and for bytes left after the value, and passes
  state of another version to `migrate`. An empty `KIND` checks no kind.
- `Tree::with_saved` starts clean on a different schema or corrupt bytes and returns
  a `RestoreOutcome`. The envelope begins with a format number, so bytes written
  before entries were tagged start clean too.
- Only the active component is created on restore. Lower stack entries are created
  when returned to, so startup depends on the screen, not on history.
- State of entries not yet rebuilt is carried into the next save, until the entry is
  removed. `ChildSet::forget` does this for a key the set is still waiting for.
- Every router has a saved form: `SavedStack`, `SavedSlot`, `SavedPages`,
  `SavedPanels`, `SavedItems` and `SavedChildSet`. The owner returns it from `save`
  and passes it back to `restore` or `restore_or`. `Items` saves its window and the
  rows inside it, not its keys. `Panels` saves its routes, not its mode.
- A router made with `new` starts with no saved state, so an owner that does not save
  a router gets a fresh one on every launch. This is the transient form.
- Saved bytes are not trusted. A repeated entry key or a counter below a key in use
  is repaired on restore, so no entry receives the state of another.
- Outside router entries, a child is identified by the order in which its parent
  created children, counting every child, saved or not. A child made with
  `Cx::child_keyed` or `Cx::persistent_child_keyed` is identified by its name
  instead, so children added around it in a later version leave its state alone.
  A name a live sibling already uses gives a child that saves nothing.
- A router is identified by its place in its owner's construction order, or by its
  name when the owner builds it inside `Cx::named_routers`, as described under
  "Custom navigation".

## Platform contract

```rust
pub trait Host {
    fn commit(&mut self, commit: &Commit<'_>);
    fn waker(&self) -> Waker { Waker::noop().clone() }
    fn diagnostic(&mut self, diagnostic: &Diagnostic<'_>) {}
}

pub struct Commit<'a> {
    pub dirty: &'a [NodeId],
    pub can_back: bool,
}
```

A platform calls `set_phase`, `back*`, `save` and `pump`. `Host` receives no reference
to the tree, so it cannot reenter it.

`Host::diagnostic` hears of a router or child that works but saves nothing: a
component's numbered router past the last saved one, and a router name or a live
sibling's child name given twice. Each `Diagnostic`
names the component that asked. Its default does nothing, and applications log it
in debug builds.

Extension rules:

- The core has no `cfg(target_os)` and no list of platforms.
- Our platforms live in separate crates and use only the public API.
- `compono_test::check_host` verifies any `Host`: commits, phase order, Back, saved
  state and waking from another thread.

### Desktop

`compono-desktop` knows no UI toolkit. `StateFile` keeps the bytes of `Tree::save` in
the system's directory for application state and replaces them through a temporary
file, so a crash during a save leaves the previous state. `Convention` names the
inputs that mean Back: Escape, a keyboard Back key and the mouse Back button
everywhere, Command-[ and Command-Left on macOS, and Alt-Left elsewhere. The adapters
translate their own events with `back_requested` (egui), `back_requests` (iced) and
`is_back_key` with `is_back_button` (GPUI). The showcases save when their window
closes and resume on the next launch.

### Web

`compono-web` has two halves. `history` is plain logic that runs and is tested
anywhere. `page`, built only for `wasm32`, binds it to the browser.

| Need | Web |
|---|---|
| Phase | `PageEvents` reports `page_phase()` on focus, blur and `visibilitychange`, and `Hidden` on `pagehide` |
| Saved state | `SessionStore` in `sessionStorage`, written when the page becomes hidden |
| Tasks | `WebSpawner` on the browser's event loop. `sleep` holds only numbers, so it is `Send` |
| Back and Forward | `History` mirrors one `Stack` in browser history |

Each stack entry is one history entry. Paths come from the core's
`compono::router::link`, described under "Links". `History::sync` runs after each change and
compares entry keys, so it allocates nothing when the stack is unchanged.

| The application | The browser |
|---|---|
| pushes | `pushState` |
| replaces the top | `replaceState` |
| pops one or more | `history.go(-n)`. Changes made before the browser arrives are written after |
| changes the root | goes back to the first entry and replaces it |

| The browser | The application |
|---|---|
| Back to one of its entries | `Navigation { keep }` pops to that depth |
| Forward | the passed entries are reopened from their paths, as new visits |
| an entry it does not know, such as a typed address | opens the screens its path names and keeps the matching bottom of the stack with its state |

Each entry stores the schema, an epoch and its index. The epoch names one run of
entries. A reload whose restored stack matches the entry keeps the epoch, so Back and
Forward keep working across it. Any other entry starts a new epoch at the current
position, which is also how a different schema is handled. A `Navigation` returns to
the component that owns the stack, so a guard in that component still applies, and
the next `sync` writes whatever the stack then shows.

History covers one stack. Dialogs, tabs and pages inside it are not entries. The
showcase uses addresses after `#`, which work from any static server. `Address::Path`
needs a server that answers every path with the page.

| Tier | Platforms | State today |
|---|---|---|
| Goal: supported and verified by running | Android, Apple, web/wasm, Linux, Windows, headless | Run on macOS desktop, in a browser and headless |
| Core build checked | wasm32, Android, Windows, iOS, `thumbv7em` | `cargo check` of the core |
| Through the contract | FreeBSD, QNX, embedded Linux, Fuchsia, consoles | Not attempted |

## UI adapters

An adapter does two things: it wakes the UI on `Commit`, and it reads `&T`. Each one
also draws routers the same way. A router gets a `view` method from the adapter's
`RouterView` trait, and `ListView` is a builder for long lists:

```rust
root.stack.view(|child| match *child {        // iced and GPUI; egui also takes `ui`
    Child::Tabs(tabs) => tabs_view(tree, tabs),
    Child::Book(book) => book_view(tree, book),
});
menu.dialog.view(|dialog| dialog_view(dialog)); // Option, because a slot can be empty
articles.panels.view(|panel| match panel {      // called once per visible panel
    Panel::Main(list) => list_view(tree, *list),
    Panel::Details(details) => details_view(tree, *details),
    Panel::Extra(author) => author_view(tree, *author),
});
ListView::new(&feed.items, handle, FeedMsg::Window)
    .row_height(32.0)
    .view(|index, row| row_view(tree, index, row));
```

A `ListView` reports the rows on screen plus `items::OVERSCAN` spare rows on each
side, and reports again only when the rows on screen leave that window
(`items::next_window`). A fast scroll then reaches rows whose components already
exist, and scrolling through spare rows sends no message. `ChildList` in SwiftUI
and Compose follows the same rule.

Decompose's `Children`, `ChildPages` and `ChildPanels` correspond to `view` on `Stack`,
`Pages` and `Panels`. A view sends with `out.send(handle, msg)` in egui,
`send(handle, msg)` in iced and `send(handle, msg, cx)` in GPUI.

The remaining differences come from the toolkits. egui draws into a `Ui` and queues
changes, iced returns an `Element` that carries an `Action`, and GPUI's list asks for
its rows by range. `showcase-egui`, `showcase-iced` and `showcase-gpui` draw the same
screens with these functions, file for file: `ui/<feature>/mod.rs` draws the feature's
parent component and one file next to it draws each child component, as
`examples/showcase/src/feature/<feature>` defines them. Each UI uses its toolkit's
usual widgets: egui's own, iced's with `iced_aw` for tabs, cards and badges, and GPUI's
with `gpui-component`.

| UI | Wake |
|---|---|
| egui | `request_repaint` |
| iced | A `Subscription` on a coalescing `WakeSignal`. `Commit` needs no action, because iced redraws after every message |
| Dioxus | signal update |
| Slint | `invoke_from_event_loop` |
| GPUI | A foreground task awaits `WakeSignal`. `cx.notify()` runs only if a `Commit` arrived |
| UIKit, Android Views | a subscription per `NodeId` |

Immediate-mode views hold `&Tree` while drawing, so changes go into an `Outbox` that
is flushed after the frame. Operations run before drawing use `BeforeDraw`, so they
do not request a frame the current one already covers.

In iced, views hold `&Tree` and buttons carry an `Action`, a cloneable description of
what to do to the tree. iced reports no minimize event, so `Presence::check_minimized`
asks after each loss of focus. Its answer can only lower the phase, so a late reply
cannot undo a newer focus event.

In GPUI a view owns the `Tree`, and every change goes through `compono_gpui::update`.
`GpuiSpawner` runs tasks on GPUI's background threads, so a GPUI application needs
no other runtime. `compono-gpui` builds on `gpui-pre` 0.3, the published snapshot of
Zed's GPUI that `gpui-component` uses, imported under the name `gpui`. GPUI reports no
minimize event, so its phase is `Visible` or `Focused`.

`ComponentViews` gives a component its own GPUI view. `update` turns `Commit.dirty`
into a redraw of those views, or of the nearest ancestor that has one, and cached
siblings are reused. Rows of a `ListView` have no views of their own and are
drawn by the list's view. An application opens windows with `gpui-pre-platform`. Its
feature `runtime_shaders` compiles shaders at startup for machines without Xcode's
separately installed Metal toolchain. The showcase enables it; release builds should
not. `gpui-pre-macos` needs Rust 1.99, so `showcase-gpui` declares that version while
the libraries keep the minimum in `VERSIONING.md`.

`compono::WakeSignal` is the shared wake-up for any event loop that can await. Any
number of wake-ups before the next wait become one. Its flag and listener change
under one lock, so a wake-up cannot fall between checking and waiting.

## Language boundary

Calls are synchronous on the UI thread, as with JSI, so the first frame does not
flicker.

| Data | Form | Frequency |
|---|---|---|
| Component address | `NodeId::to_bits() -> u64` | Constant |
| UI events | address, variant number, plain fields | Per action |
| Navigation structure | array of numeric `{ key, kind, node, phase }` | Per navigation |
| `Commit` | `u64` array and `can_back` | Once per operation |
| Component model | one record per component | Only when changed |
| Large lists | windows by range, stable keys | On scroll |
| Gesture and animation progress | stays in the UI | Every frame |

JavaScript numbers hold 53 integer bits, so the web boundary must pass a `NodeId` as
two 32-bit numbers rather than `to_bits()`. `to_bits` never returns `u64::MAX`, so
`compono_ffi::NO_NODE`, `-1` as an `i64`, stands for no component.

An application declares its models itself, in functions it exports that read a
component through `NativeTree::read`. Neither the core nor `compono-ffi` knows them.

UniFFI requires `Send + Sync`. The tree lives in a `thread_local!`, and the exported
object holds only an identifier and returns an error when called from another
thread, so no `unsafe impl Send` is needed. The chosen backend is BoltFFI: on the
same scenario, a message that reaches a component and commits back costs 68 ns against
508 ns for UniFFI on the iOS simulator, and 136 ns against 13.5 µs on the Android
emulator. `benchmarks/ffi-boundary` holds the measurement. No device has been measured.

The Swift package and the Compose library know nothing about generated bindings.
Their names match what BoltFFI generates for the standard exports (`pump`,
`set_phase`, `back`, `back_start`, `back_cancel`, `back_commit`, `contains`, `save`
and a listener with `commit` and `schedule_pump`), so connecting them is short:

| | SwiftUI | Compose |
|---|---|---|
| Bindings to tree | `extension App: @retroactive TreeBridge {}` | `treeBridge(app::pump, …)` |
| Listener to tree | `extension ComponoTree: @retroactive Listener {}` | `object : Listener, ComponoListener by it {}` |
| Root and saved state | `ComponoRoot { tree, saved in App(listener: tree, saved: saved) }` | `retainedTree(create, bridge)` |
| Lifecycle and Back | `.componoRoot(root)` | `ComponoHost(root.tree)` |

Kotlin cannot add an interface to a generated class, so Compose takes the bindings'
methods by reference instead. BoltFFI reads exports from source files, so the Rust
side writes the standard methods itself rather than through a macro.

A view calls `tree.observe(node)` for each component it reads, so a commit redraws
only those views, as `ComponentViews` does in GPUI.

| Decompose | SwiftUI | Compose |
|---|---|---|
| `Children(stack)` | `ChildStack(active:)` | `ChildStack(active, key)` |
| `ChildSlot` | `ChildSlot(active:)` | `ChildSlot(active, key)` |
| `ChildPages` | `ChildPages(active:)` | `ChildPages(active, key)` |
| `ChildPanels` | `ChildPanels(panels:)` | `ChildPanels(panels, key)` |
| Lazy list | `ChildList(count:onWindow:row:)` | `ChildList(count, onWindow, row)` |
| Lifecycle and Back | `.componoRoot(root)` or `.componoLifecycle(tree)` | `ComponoHost(tree)` |

A `ChildStack` moves between entries with a `StackAnimation`: its transition, its
timing and its Back gesture, native by default. Compose uses Material's shared axis
on the standard spatial spring and the Material predictive Back, which shrinks the
top entry toward the far edge and follows the touch with `BackEvent`'s edge and
height. SwiftUI slides on UIKit's push spring (response 0.29 s, measured) with a
30 % parallax and a 12 % shade, and settles a released swipe on the same spring,
starting at the finger's speed. The formulas repeat `compono-motion`'s, and each package's `MotionTests` compares them
with the golden table. `accessibilityReduceMotion` turns the SwiftUI stack into a
fade, and an animator duration scale of 0 turns the Compose stack's motion off. It moves
back only when the entry it showed is destroyed and the new one was below it, so
`bring_to_front` moves forward, as a push does. The
component that leaves a stack is destroyed at once, as in Decompose, and the view on
its way out is the one already drawn. `Stack::previous` gives the entry that Back
would reveal, which the stack draws beneath the active one during a gesture. Its
component stays `Hidden` until the gesture completes.

The Compose library uses `PredictiveBackHandler`, and the three-button Back plays the
same animation. The Swift package turns a swipe from the left edge into Back on iOS.
`back_start` names the gesture's target, so a stack moves only when Back is aimed at
the component that owns it and a dialog above it can close first. Progress stays in
the UI, so a gesture costs one commit at each end and none per frame. A Back press
while the previous gesture animates to its end finishes that gesture at once and
starts the new one, on both platforms. An entry restored
but not yet created has nothing to draw beneath, so its first gesture shows the
background.

`native/showcase/ffi` exports a model and actions for every screen. Addresses and
counts cross as `i32` and `i64`, which are plain numbers in both languages.

`compono-ffi` depends on no binding generator. `NativeTree` is that exported object:
it can be held and dropped on any thread. A tree dropped elsewhere asks its platform
for a pump, and is freed by the owner's next call on any tree or by `collect_retired`. Each owning thread keeps its own list of such trees, shared only
with their handles, so a thread that ends leaves nothing behind. A commit is held
until the tree is released, so `Platform::commit` may read the tree back, and it
never names a component that the same call removed. A call from inside a running call
returns `NativeError::Busy`. `NativeError::is_misuse` separates such bugs in the
caller from `Gone`, which a native UI meets in normal use, and the showcase stops a
debug build at them.
Addresses are checked against the component's type by `Tree::handle_of`.

The top-level model also suits a mode where Rust drives native views directly. That
is one more adapter over the same `Commit` and `NodeId`.

## Comparison with Exact

Exact, from the Expo team, is a whole application framework rather than a
component library. Screens are written in its own language, Contract, and run on
its runner over a Rust layout kernel. Its navigation is the `exact-route` crate: a
plain value holding a location and a retained stack per tab, moved by pure functions
(`open`, `push`, `replace`, `back`, `select`, `go`) that return the input and a
refusal when a move is invalid.

Adopted from it: an entry key names a visit rather than a screen, a retained stack
per tab as a first-class model (`Tabs`), and the 53-bit limit on identifiers that
reach JavaScript.

Not adopted: string URLs as the primary route type, which give up compile-time
checks; cloning the router value in `open`, `push` and `replace`; and
`provide`/`inject`, because Compono children receive what they need from constructors.

## Code rules

- Libraries forbid `unwrap`, `expect`, `panic!`, indexing, `unreachable!`, `todo!`
  and `unimplemented!`, through `[workspace.lints]`.
- `unsafe` is forbidden, except in platform crates with a `SAFETY:` comment.
- A user component's `Drop` has no access to the tree.
- One public path per item, with no prelude or glob re-exports.
- Keep what a UI shows but does not draw, such as button lists and dialog text, in the
  components, so that every UI shares it.
- A file describes one thing and stays under about 400 lines where possible.
- Documentation describes the current state. The repository keeps no decision journals.
- The entry points for agents are `AGENTS.md`, this file and compiled doc examples.

## Verification

Besides example tests, the core checks laws on generated input and model-checks its
concurrency:

- `crates/compono/tests/properties.rs`: a `ChildSet` matches every desired list and
  keeps surviving children; a `Stack` follows a plain list and survives a restore.
- `crates/compono/tests/router_laws.rs`: `Slot`, `Pages`, `Panels` and `Items` each
  follow a plain model through any sequence of operations and a restore.
- `crates/compono/tests/tree_laws.rs`: a plain child never exceeds its parent, and
  Back goes to the most recently shown handler; saved bytes hold the state of entries
  that exist and of no other; damaged, changed or shortened bytes never give two
  entries one key.
- `crates/compono/tests/phases.rs`: after each of 4 000 generated grants in a tree
  four levels deep, every component has, and was last told, the lowest phase granted
  on its path.
- `crates/compono/tests/queue.rs`: navigation asked for from a constructor or from
  `phase_changed` starts after the navigation that is running ends.
- `crates/compono-web/tests/history.rs`: browser history ends where the stack is
  after any sequence of application changes, Back and Forward.
- `crates/compono/tests/loom.rs`, under `--cfg loom`: no wake-up is lost when it
  races with a poll, including a local task woken while it is polled, messages sent
  while the tree pumps arrive exactly once, and a message sent while the tree is
  dropped is dropped or refused.

## Performance requirements

`crates/compono/tests/allocations.rs` enforces the allocation budget in CI. Timings are
measured by hand with the examples in `crates/compono/examples`.

- Navigation cost does not depend on stack depth.
- The cost of a phase change does not depend on hidden children, such as the lower
  entries of a stack.
- The cost of `Pages`, `Items`, `Panels` and `Slot` does not depend on the entries
  that are not on screen.
- A push and pop cycle costs at most 3 allocations, including the component.
- An operation with no change causes no `Commit` and no allocation.
- Pumping an empty queue does not allocate.
- Startup after restore creates only visible components.
- Reconciling a list is `O(n)`.
- Decompose and Compono are measured on the same scenarios: JVM, Android, Kotlin/Native.

## Plan

| Stage | Result | Status |
|---|---|---|
| 1. Core | `ChildSet`, `Stack`, `Slot`, `compono-test` | Done |
| 2. UI validation | iced and GPUI | Done |
| 3. Native UI | SwiftUI and Compose on BoltFFI | Done on simulators, with transitions, swipe-back and predictive Back |
| 4. Platforms | Hosts for Android, Apple, web, desktop, checked on devices | Web and desktop done; devices not checked |
| 5. Navigation | `Pages`, `Panels`, `Items`, tabs, web history | Done |
| 6. Comparison | Benchmarks against Decompose | Done on JVM, Kotlin/Native, iOS simulator and Android emulator |
| 7. Agents | Plugin and skill after all tests | |

## Verification status

Measured in release with `panic = "abort"` on one Apple Silicon machine, as the
middle of three runs. Single runs on a busy machine come out up to twice as high,
so compare growth with size rather than absolute numbers.

- `Stack` push and pop: about 150 ns and 3 allocations at depths 1, 10 and 100,
  the same for a stack named with `Cx::named_routers`.
- Hiding and showing the component that owns a `Stack`: about 36 ns at depths 1,
  100 and 10,000.
- `ChildSet`, per child, from 1,000 to 1,000,000 children: create 70 to 150 ns,
  unchanged 1 to 6 ns, full reversal 16 to 66 ns, removing half 21 to 43 ns.

- `Pages`, `Items`, `Panels` and `Slot` with 100, 10,000 and 1,000,000 entries off
  screen (`router_scaling`): selecting a page about 200 ns, scrolling a window of 30
  rows by one row about 550 ns, opening and closing the details panel about 104 ns,
  opening and closing a dialog about 77 ns, at every size.

The showcase components are shared unchanged by the egui, iced and GPUI apps.

Under load (`cargo run -p showcase --release --example load`): scrolling the 100 000 row
feed by one row costs about 660 ns, replacing a whole window of 30 rows about 2.3 µs,
updating all 30 live rows about 720 ns, switching tabs about 140 ns.

Against Decompose 3.6.1 on the same machines, a stack push and pop at depth 100 costs
76 ns against 22 µs on the JVM, and 298 ns against 309 µs on the Android emulator.
`benchmarks/decompose-comparison` has every scenario and its caveats.

The web showcase was run in Chromium: pushing and closing screens, Back and Forward,
a reload that resumes the same screen, a typed address and a link opened in a new
tab. The desktop shortcuts and saved state are covered by tests through each
toolkit; the windows were not closed and reopened by hand.

Not yet verified: Dioxus, Slint, real devices (SwiftUI and Compose ran on a simulator and an emulator), Firefox and Safari, Windows and Linux, and running on an embedded target.

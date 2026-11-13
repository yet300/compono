# Changelog

All crates of the workspace share one version. `VERSIONING.md` says what a version
number promises.

## 1.0.0-rc

The first release candidate. No version was published before it, so "changed"
and "fixed" below compare with the unpublished development tree.

### Added

- `compono-motion`, a new `no_std` crate with no dependencies, holds the router
  transitions every UI shares: `Easing`, `Spring`, `Timing`, `LayerStyle`,
  `Direction`, `Transition`, `BackEvent`, `BackAnimation`, `MotionPreference`,
  `Motion` with the presets `material`, `cupertino`, `desktop`, `web` and `none`,
  `StackChange::between` and `TransitionState`. Its `golden` example writes
  `golden/motion.csv`, the values the Swift and Kotlin tests compare with.
- In egui, a `Stack` drawn with `view` animates push and pop with
  `Motion::desktop()`. `compono_egui::set_motion` sets the motion of every router
  in a context, and `compono_egui::StackView` sets it for one stack. A popped entry
  is replayed from the shapes of its last frame. `showcase-web` uses `Motion::web()`.
- In egui, a `Slot` fades its entry in and out, `Pages` slide by page order, and
  `Panels` in `PanelsMode::Single` slide the deepest panel in and out, with the
  timing of the context's motion. The replay covers every layer an entry paints,
  so a dialog drawn in an `egui::Modal` fades out too.
- In iced and GPUI, `view` on a `Stack`, `Slot`, `Pages` or `Panels` animates the
  same changes with `Motion::desktop()`, and `compono_iced::set_motion` and
  `compono_gpui::set_motion` change the motion of the whole application. Only the
  entry that becomes active moves; the one that leaves disappears at once. GPUI
  applies offsets and opacity but not scale. `compono-iced` now enables iced's
  `advanced` feature for its widget.
- Reduced motion: `compono_egui::set_motion_preference` and
  `compono_iced::set_motion_preference` take the user's `MotionPreference`,
  `compono-gpui` follows GPUI's `App::reduce_motion`, and
  `compono_web::prefers_reduced_motion` reads
  `matchMedia("(prefers-reduced-motion: reduce)")`. `showcase-web` passes it to egui.
- `Tree::set_poll_limit` bounds how many `Cx::perform_local` tasks one pump polls,
  1 024 by default. A limit of 0 is treated as 1.
- `Host::diagnostic`, a provided method that does nothing by default, receives a
  `Diagnostic` when a router or child works but saves nothing: a component's
  numbered router past its 32 767th (`RouterLimit`), a router name given twice
  (`RepeatedRouterName`) and a child name a live sibling uses (`RepeatedChildName`).
  `compono_test`'s recording host keeps them, in `CommitLog::diagnostics`.
- `Cx::named_routers` builds routers that find their saved state by name instead
  of by construction order, so a later version can construct another router before
  them, or one only behind a feature flag. It works for every router that takes its
  id from `Cx::router`, built-in or custom. A name a component has already given
  out gives a router that saves nothing. Named routers take no number, and unnamed
  routers save and restore as before.
- `Cx::replace_router(old, build)` replaces a router: it destroys `old`, keeping
  the saved state under its ids, then builds the new router, which takes those ids
  in order. The new router decides what state to keep, so one restored from the
  old router's snapshot gives entries not yet shown their state when they are.
- `Slot::active_key`, `Pages::child` and `Panels::keys`, with the new
  `PanelKeys`, give the visit key of a slot's entry, the value of any page that
  exists and the visit key of each open panel, so a UI can animate between them.
- `router::Router`, the trait of every router, with `router_ids` and `destroy`.
  The six built-in routers and the showcase's `Filmstrip` implement it.
- `Tree::is_poisoned` and `NativeError::Poisoned`. A tree whose operation unwound
  from a caught panic runs no more user code and commits nothing. Before, it went
  on half-changed, with the component that panicked out of its slot.
- `compono_ffi::NO_NODE`, the address no component has, and the promise that
  `NodeId::to_bits` never returns `u64::MAX`.
- `NativeError::is_misuse` tells a bug in the native caller, such as a call from
  another thread, from `NativeError::Gone`, which a native UI meets in normal use.
- `Cx::reusing_routers` builds a router in the ids of the router it replaces, and
  every router has `router_ids`. Before, a router rebuilt after construction took a
  new id, so its entries lost their saved state on the next launch, and each
  rebuild used up one of the component's 32 767 saved ids.
- `Tree::set_work_limit` bounds the queued messages and outputs one operation
  delivers, 1 048 576 by default, and the `perform_local` tasks one pump polls.
  Past it the operation commits, keeps the rest queued and wakes the host, so
  components that keep answering each other no longer hold the thread forever.
- `Stack::destroy`, `Slot::destroy`, `Pages::destroy`, `Panels::destroy`,
  `Items::destroy` and `ChildSet::destroy` remove a router together with its
  components at once.
- `Items::snapshot`, `Items::restore_or` and `SavedItems` save and restore the window
  of a list and the state of the rows inside it.
- `Panels::snapshot`, `Panels::restore_or` and `SavedPanels` save and restore the
  open panels.
- `Panels::set_main`, `Panels::main_route`, `Panels::details_route` and
  `Panels::extra_route`.
- `ChildSet::forget_waiting` discards every restored key that was not rebuilt.
- `Cx::forget_router` discards the saved state of one router. A custom router that
  starts without saved navigation calls it once.
- `Cx::keep_router_entries` discards the saved state of every entry of one router
  except the keys given. A custom router calls it once when it restores.
- `Cx::perform_local` runs a future that is not `Send`, such as a browser `fetch`, on
  the tree's own thread and delivers its result as a message. It needs no `Spawner`.
- Swift: `ComponoRoot` creates the tree and the bindings once, reads saved state from a
  file and writes it in the background, and `.componoRoot(root)` attaches it to a view.
  `ComponoTree` has `commit` and `schedulePump` as nonisolated methods, so it can be a
  BoltFFI listener directly.
- Compose: `retainedTree` keeps a tree for an Activity across configuration changes,
  saves it in the Activity's saved state, and closes it when the Activity finishes.
  `ComponoListener` and `treeBridge` connect the generated bindings in one expression.
- Compose: `LocalComponoTree`, `observe(node)` and `rememberComponent(key) { }`.
  `ComponoHost` provides the tree, a screen subscribes to its component with
  `observe`, and `rememberComponent` closes a component object when it leaves the
  composition.
- Swift: `EnvironmentValues.componoTree`, set by `.componoRoot(root)`, so a screen
  subscribes with `tree?.observe(component.node())`.
- The native showcase exports one class per component, as Decompose does: screens
  take `CountersComponent` and the like instead of node addresses.
- `skills/` holds agent skills for applications that use Compono:
  `compono-components`, `compono-navigation`, `compono-ui` and
  `compono-native-apps`.
- `Cx::child_keyed` and `Cx::persistent_child_keyed` identify a child in saved
  state by name, so children added around it in a later version leave its state
  alone.
- `RouterId::is_saved`.
- `items::next_window` and `items::OVERSCAN`. Every `ListView` takes
  `.overscan(rows)`, and `compono_gpui::ListView` takes `.window(current)`.
  SwiftUI and Compose `ChildList` take `overscan`.
- `Navigation::toward` turns a path into the change that shows it, for deep links on
  any platform.
- `Handle` implements `Hash`.
- Compiled examples in the documentation of every crate except `compono-gpui`, and
  of `Stack`, `Slot`, `Pages`, `Panels`, `Items`, `ChildSet`, `Persist`, `Cx::child`
  and `Cx::scoped`.
- The example `router_scaling` measures `Pages`, `Items`, `Panels` and `Slot` as the
  entries off screen grow.
- The example `navigation_cost` also measures hiding and showing the component that
  owns a stack.

### Changed

- The Swift package's `StackAnimation` is now a struct with `transition`
  (`StackTransition`), `animation` (a SwiftUI `Animation`) and `back`
  (`BackGestureStyle`). `.slide`, `.fade` and `.none` remain, and `.platform`, the
  default, slides on UIKit's push spring with a 30 % parallax and a 12 % shade, all
  three measured on `UINavigationController` pushes. `Motion::cupertino()` and
  `Spring::CUPERTINO` use the same spring: response 0.29 s, critically damped.
  A released swipe settles on that spring at the finger's speed:
  `ComponoTree.finishGesture(commit:speed:)`. With `accessibilityReduceMotion` the
  stack fades in place.
- The Compose library's `StackAnimation` is now a class with `transition`
  (`StackTransition`), `spec` and `back` (`BackAnimation`). The default is Material:
  shared axis X on `spring(0.9f, 700f)` with `BackAnimation.Predictive`, which
  follows `BackEventCompat`'s edge and touch. `StackAnimation.Slide`, `Fade` and
  `None` remain. `ComponoTree.updateGesture` takes the event's edge and touch, and
  "Remove animations" (animator duration scale 0) turns stack motion off.

- `Tree::set_work_limit` no longer bounds polls of `Cx::perform_local` tasks,
  which have their own limit, `Tree::set_poll_limit`. Before, one limit of
  1 048 576 covered both, and a pump could run that many polls of user code.
- `Option<NodeId>` is 8 bytes instead of 12, and each tree slot 40 bytes smaller
  on 64-bit targets. `NodeId` keeps its generation plus one in a `NonZeroU32`.
  `NodeId::to_bits` gives the same numbers as before for every id a tree issues.
  `NodeId::from_bits` of bits with generation `u32::MAX`, which `to_bits` never
  returns, gives an id that matches no component.
- Saved state records, for each entry, a stable FNV-1a hash of the component's new
  `Persist::KIND` and its new `Persist::VERSION`. `Cx::restored` gives `None` for
  state saved by a component of another kind at the same place, and for bytes left
  after the value; it calls the new `Persist::migrate` for state of another
  version. An empty `KIND`, the default, checks no kind, so existing
  implementations compile and restore as before. The saved envelope changed: bytes
  saved by an earlier version give `RestoreOutcome::Corrupt` or
  `RestoreOutcome::SchemaChanged`, and the application starts clean. The showcase
  sets a `KIND` on every saved component and raises its schema to 3.
- `Cx::destroy` and `Cx::set_child_phase` return `Result<(), NotOwnChild>`, as
  `Cx::send` does. A target that is not a live direct child, such as a grandchild
  or a child already removed, gives `Err(NotOwnChild)`. Before, both did nothing
  and returned `()`.
- `Tree::set_work_limit` now bounds all the work of one operation together:
  delivering queued messages and outputs, and removing the components of dropped
  routers and scopes, one unit per component. Before, removals were not counted and
  each pass of the loop in one operation started a fresh limit, so dropping a large
  router removed all its components in one operation.
- `Cx::reusing_routers` is removed. Use `Cx::replace_router`, which takes the old
  router itself, so two live routers can no longer hold one id. A router id no
  longer has a generation, and the tree keeps no record of moved ids.
- `router_ids` and `destroy` of `Stack`, `Slot`, `Pages`, `Panels`, `Items` and
  `ChildSet` are methods of `router::Router` instead of inherent methods. Import
  the trait to call them. `router_ids` returns `impl IntoIterator<Item = RouterId>`
  instead of an array.
- Numbered children past 2^61 in one parent save nothing, so their keys never
  reach those of named routers.
- `Tree::back_progress` and `BackGesture` are removed: a gesture's progress stays
  in the UI, which animates it. `Tree::gesture` is replaced by
  `Tree::gesture_target`, which returns the gesture's target `NodeId`.
- `compono_ffi::Project`, `NativeChild`, `RouterEntry` and `NativeTree::model` are
  removed. An application reads its models in its own exported functions with
  `NativeTree::read`, as the native showcase does.
- Swift: `ComponoTree` holds its bindings weakly. Before, the bindings held the tree
  as their listener and the tree held the bindings, so neither was ever freed.
  `ComponoRoot` keeps the bindings; code that calls `attach` directly keeps them
  itself.
- `Cx::send` returns `Result<(), NotOwnChild>`. Before, a message to anything but
  the component itself or one of its live children, such as a grandchild or a
  removed child, was dropped without a sign.
- Dropping a `Scope`, and so a router, removes its components when the running
  operation ends. Before, they stayed alive with their last phase and still handled
  Back until their owner was removed.
- Swift: `TreeBridge` takes `setPhase(code:)` and `contains(node:)`, the names BoltFFI
  generates, so generated bindings conform with an empty extension.

- `Paths` and `Navigation` moved from `compono_web::history` to
  `compono::router::link`, so Android and iOS deep links use them without the web
  crate.

- Compose: the library and the showcase compile against SDK 37, which the showcase's
  `adaptive` 1.3 requires.
- A component's routers after the 32 767th save nothing. Before, their ids
  wrapped and a later router overwrote the saved state of the first.
- Lists report the rows on screen plus 10 spare rows on each side, and report
  again only when the rows on screen leave that window. Before, every scroll by
  one row was reported, and a fast scroll drew placeholders for a frame.
- `compono-gpui` builds on `gpui-pre` 0.3.12 instead of `gpui` 0.2.2. Its feature
  `runtime_shaders` is removed: an application enables it on `gpui-pre-platform`.

- `SendError`, `RestoreOutcome`, `NativeError`, `compono_desktop::Key`,
  `Convention`, `PageEvent`, `Address` and `compono_iced::Action` are
  `#[non_exhaustive]`. A `match` on one of them needs a wildcard arm.
- `Commit` and `BackGesture` are `#[non_exhaustive]`. Code outside `compono` reads
  them and cannot construct them.
- A restored router keeps saved state only for the entries its snapshot names.
- A router made with `new` starts with no saved state. Before, its entries could
  receive the state that an earlier launch had saved under the same key.
- `Sender::send` returns `SendError::Closed` once its tree is dropped.
- `Tree::back_progress` ignores a value that is not a number.
- `Platform::commit` no longer names a component that the same call removed.
- `Pages::restore_or` discards saved state of pages outside the retained range.
- A phase change visits only the children granted `Visible` or above. Its cost no
  longer grows with hidden children, such as the lower entries of a `Stack`. Among
  siblings, the one shown most recently falls first and rises last. Before, the
  order followed creation.

### Fixed

- `ChildStack` in the Swift package and the Compose library played the Back
  animation after `bring_to_front` of an older entry, because it compared entry
  keys. It now moves back only when the entry it showed is gone and the new one was
  below it. The Compose `ChildStack` takes a new optional `node` parameter, the
  address of an entry's component, to tell a parked entry from a destroyed one.
- A task started with `Cx::perform` whose future or `to_msg` panics no longer
  leaves its guard in the component until the component is removed. In builds that
  unwind, the panic stops at the task, the next pump releases the guard, and the
  component receives nothing from it.
- A `NativeTree` dropped on a thread other than its owner calls
  `Platform::schedule_pump`, so the owner frees the tree, its `Platform` and the
  native listener. Before, they stayed in memory until the owner happened to make
  another call. `NativeTree` now holds its `Platform`.
- A task started with `Cx::perform_local` is listed for the next pump once, however
  often its waker is called, and the host is woken once for it. Before, every
  wake-up added an entry and woke the host, so a waker called again and again from
  another thread between two pumps grew the list without limit.
- A save made while a work limit had left a dropped router's entries for a later
  operation saved them too, under the keys their replacement now uses. A later
  launch could give the replacement's children that state. Entries queued for
  removal are no longer saved.
- `visible_rows` overflowed on a viewport that is infinite or very large.
- A restored `Stack`, `Slot` or `ChildSet` trusted the keys and the counter in saved
  bytes. Damaged bytes could give two entries one key, and so one entry the state of
  another.
- Destroying, hiding, showing or saving a very deep chain of components overflowed
  the stack. These walks no longer recurse.
- A slot of the tree that had used every generation could match its first id again.
- A `NativeTree` dropped on another thread after its own thread had ended left a
  record behind for ever.
- `VirtualTime::sleep` and `VirtualTime::advance` panicked on a duration too long to
  add.
- `StateFile::store` left its temporary file behind when the write or rename failed.
- `History::start` could overflow on an index read from history state on a 32-bit
  target.
- Compose: a Back gesture whose settle animation was cancelled never ended, and Back
  stopped working until the process restarted.
- Compose: a configuration change, such as a rotation, hid every component and then
  showed it again. The phase now stays.
- Compose and SwiftUI: a Back press while the previous gesture animated to its end
  was dropped. It now finishes that gesture and starts its own.
- SwiftUI: the end of a settle animation could end a newer gesture.
- SwiftUI: `ChildStack` clipped its content, which hid the system tab bar and
  toolbar on iOS 26.
- A restored router carried saved state of entries its snapshot did not name into
  every later save.
- `Items::restore_or` took its identity in saved state before it refused repeated
  keys. A router built in its place then moved the identity of every later router of
  that parent, which lost their saved state or gave it to an entry of another router.

### Licence

- Compono is licensed under Apache-2.0. Each crate's package includes `LICENSE`.

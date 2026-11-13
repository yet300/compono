# Release audit for 1.0.0-rc.1

This report covers the audit made on 2026-11-11 before the first release candidate.
Every result below was read from command output on one Apple Silicon Mac with
Rust 1.96.0.

## Baseline

Taken before any file was changed.

| Check | Result | Detail |
|---|---|---|
| `cargo fmt --check` | pass | |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass | |
| `cargo clippy -p compono --no-default-features -- -D warnings` | pass | |
| `cargo clippy -p compono --no-default-features --features std -- -D warnings` | pass | |
| `cargo clippy -p compono --no-default-features --features serde -- -D warnings` | pass | |
| `cargo test --workspace` | pass | 119 tests, 0 failed |
| `cargo check -p compono --no-default-features --target thumbv7em-none-eabihf` | pass | |
| `cargo clippy -p compono-web -p showcase-web --target wasm32-unknown-unknown -- -D warnings` | pass | |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass | |
| The same for `compono-web` with `--target wasm32-unknown-unknown` | pass | |
| loom: `cargo test -p compono --test loom --release` under `--cfg loom` | pass | 3 models |
| `navigation_cost` example | pass | 264, 249 and 248 ns at depths 1, 10 and 100, 3 allocations each |
| `set_scaling` example | pass | create 123 to 245 ns, unchanged 5 to 16 ns per child from 1,000 to 1,000,000 children |
| `native/showcase/ffi`: `cargo clippy --all-targets -- -D warnings` | pass | |
| `native/apple/Compono`: `swift test` | pass | 8 tests |
| `native/showcase/android`: `gradle :compono:testDebugUnitTest :app:assembleDebug` | pass | Run with `--offline`. Every task was up to date, so Gradle reused earlier test results |
| `boltffi pack apple` and `boltffi pack android` | not run | `boltffi` is not installed on this machine. The existing packed libraries were used |
| `cargo miri test` | not run | No library crate has `unsafe` code. The only `unsafe` is the counting allocator in one test and one example. Miri is also not installed |
| Web showcase in a browser | not run | `wasm-bindgen-cli` was not checked in this step |
| A simulator, an emulator or a device | not run | Not part of the listed checks |

## Comparison

Facts come from the pages named beside them, read on 2026-11-11. Where a page could
not be read, the text says so.

### Decompose 3.6.1 and Essenty

- **Saved state.** A component has a `StateKeeper` from Essenty. Navigation
  configurations are saved with kotlinx-serialization, and `serializer = null`
  turns saving off for one router
  (<https://arkivanov.github.io/Decompose/component/state-preservation/>,
  <https://arkivanov.github.io/Decompose/navigation/stack/overview/>). On restore a
  stack creates its components again from the bottom up (same page). Compono
  creates only the active entry. `InstanceKeeper` keeps instances through an
  Android configuration change
  (<https://arkivanov.github.io/Decompose/component/instance-retaining/>). Compono
  needs no such part, because the tree outlives the Activity.
- **Back.** A component registers a `BackCallback` with a priority. The newest
  enabled callback runs first
  (<https://arkivanov.github.io/Decompose/component/back-button/>). Predictive Back
  is `predictiveBackAnimation` in the Compose extension, marked experimental, and
  iOS swipe-back is written by the application
  (<https://arkivanov.github.io/Decompose/extensions/compose/>). Compono has one
  declarative `back()` per component and the order "deepest, then most recently
  shown". It has no priorities.
- **Side effects.** A coroutine scope follows the component's `Lifecycle` through
  Essenty's `lifecycle-coroutines`
  (<https://arkivanov.github.io/Decompose/component/scopes/>). Tests drive a
  `LifecycleRegistry` by hand and use `decompose-test-utils`
  (<https://github.com/arkivanov/Essenty>,
  <https://github.com/arkivanov/Decompose/tree/master/decompose-test-utils>).
  Navigation called from inside navigation is queued
  (<https://arkivanov.github.io/Decompose/navigation/stack/navigation/>), as in
  Compono.
- **UI and web.** Compose has `Children`, `ChildPages` and `ChildPanels`
  (<https://arkivanov.github.io/Decompose/extensions/compose/>). Web navigation
  covers stack, pages and panels through `childStackWebNavigation`,
  `childPagesWebNavigation` and `childPanelsWebNavigation`, and needs a server that
  answers every path
  (<https://arkivanov.github.io/Decompose/navigation/web-navigation/>). Android deep
  links use `handleDeepLink` and an `initialStack`
  (<https://arkivanov.github.io/Decompose/navigation/stack/deeplinking/>).
- **Functions Compono does not have.** Web navigation for pages and panels. A
  deep-link helper for Android. Back priorities. `navigate(transformer, onComplete)`
  with an arbitrary transformation of the whole stack. A transient saver
  (`transientNavStateSaver`,
  <https://arkivanov.github.io/Decompose/navigation/items/overview/>). Lifecycle
  states per page chosen by a function. Stack animations with per-child animators
  (<https://github.com/arkivanov/Decompose/releases>).
- **Reported defects, and Compono.**

| Decompose issue | Compono |
|---|---|
| [#1019](https://github.com/arkivanov/Decompose/issues/1019), [#987](https://github.com/arkivanov/Decompose/issues/987): duplicate configuration or key crashes | A stack entry is a visit, so equal routes are allowed. `Pages`, `Items` and `ChildSet` return `DuplicateKey` and change nothing |
| [#1032](https://github.com/arkivanov/Decompose/issues/1032): Back to a history entry from an older build crashes | Each entry stores a schema and an epoch. Another schema starts a new run of entries. Covered by `an_entry_from_another_run_is_opened_from_its_path` |
| [#982](https://github.com/arkivanov/Decompose/issues/982): URL not updated when the initial stack has several entries | Covered by `a_link_opens_its_screens_with_back_entries` |
| [#913](https://github.com/arkivanov/Decompose/issues/913): slot loses navigation state when closed | `SavedSlot` keeps the next key when closed |
| [#907](https://github.com/arkivanov/Decompose/issues/907): web history adds several entries | The law `the_browser_always_ends_where_the_stack_is` checks entry counts after any sequence |
| [#874](https://github.com/arkivanov/Decompose/issues/874): popped components stay in memory | A popped entry is destroyed at once. Compono has its own form of this problem: a router value that is dropped or replaced as a field leaves its components alive until the parent goes. See defect D12 |
| [#868](https://github.com/arkivanov/Decompose/issues/868): race in `ChildPages` | The tree has one thread |
| [#875](https://github.com/arkivanov/Decompose/issues/875), [#1036](https://github.com/arkivanov/Decompose/issues/1036): navigation during an animation freezes | Animations are in the UI and the tree never waits for one. The native `ChildStack` views were not tested for this |
| [#807](https://github.com/arkivanov/Decompose/issues/807): a second root crashes | `Tree::mount` replaces the root |

### Crux

- **Saved state.** None is built in. The shell stores data through the `KeyValue`
  capability (<https://github.com/redbadger/crux/tree/master/crux_kv>).
- **Back.** Crux has no navigation model. The core holds behaviour and the shell
  holds presentation (<https://redbadger.github.io/crux/>).
- **Side effects.** `update` returns a `Command`. Effects are data that the shell
  carries out, and a test resolves them by hand with no mock
  (<https://redbadger.github.io/crux/part-2/effects.html>,
  <https://redbadger.github.io/crux/part-2/testing-effects.html>). A command is
  cancelled through `abort_handle()`.
- **UI and web.** The core is packed with BoltFFI for Swift and Kotlin and as
  WebAssembly. Messages cross as Bincode, and types are generated for Swift,
  Kotlin and TypeScript (<https://github.com/redbadger/crux>).
- **Functions Compono does not have.** Effects as inspectable data. Streams from
  the shell. Generated foreign types. Published HTTP, key-value and time
  capabilities.
- **Reported defects, and Compono.**
  [#577](https://github.com/redbadger/crux/issues/577), "Executor in `crux_core`
  can evict a live task": Compono has no executor of its own.
  [#622](https://github.com/redbadger/crux/issues/622), the shell cannot tell that
  the core dropped a stream: in Compono dropping a `TaskGuard` cancels the task on
  its executor.
  [#621](https://github.com/redbadger/crux/issues/621), generated Swift reorders
  stream items: Compono delivers queued messages in the order sent, checked by the
  loom model.

### The Composable Architecture

The documentation site returned 404 and the Swift Package Index pages returned no
article text, so only the README and the issue tracker were read. State saving,
`StackState` and `TestStore` details are therefore not verified here.

- **Side effects.** A reducer returns an `Effect`, such as `.run`. Dependencies
  come from `@Dependency`, and `TestStore` replaces them with
  `withDependencies` (<https://github.com/pointfreeco/swift-composable-architecture>).
- **UI.** SwiftUI and UIKit on Apple systems only (same page).
- **Reported defects, and Compono.**
  [#3663](https://github.com/pointfreeco/swift-composable-architecture/issues/3663),
  an effect is not cancelled when the view is dismissed at the same moment:
  Compono drops a component's guards when it is destroyed and discards a late
  result by its generation
  (`destroying_a_component_cancels_its_task_and_drops_late_results`).
  [#1848](https://github.com/pointfreeco/swift-composable-architecture/issues/1848),
  an effect cannot be cancelled before it runs: `cx.cancel` removes the guard at
  once (`a_cancelled_task_never_delivers_even_if_it_already_finished`).
  [#3673](https://github.com/pointfreeco/swift-composable-architecture/issues/3673),
  dismiss fails after a cancelled swipe: `back_cancel` only clears the gesture
  (`predictive_back_changes_nothing_until_committed`).

### Circuit

- **Saved state.** `rememberSaveableBackStack` survives process death. Savers are
  pluggable through `CircuitSaver`, and a record that cannot be restored is dropped
  (<https://slackhq.github.io/circuit/docs/navigation-persistence/>).
  `rememberRetained` survives the back stack but not process death
  (<https://slackhq.github.io/circuit/docs/presenter/>).
- **Back.** `Navigator.pop` behind a `BackHandler`, and predictive Back in
  `circuitx-gesture-navigation` from Android 14
  (<https://slackhq.github.io/circuit/circuitx/gesture-navigation/>).
- **Side effects.** Compose effects inside `present()`. Tests use
  `Presenter.test {}`, `FakeNavigator` and `TestEventSink`
  (<https://slackhq.github.io/circuit/docs/testing/>).
- **UI.** Compose on Android, desktop and iOS (<https://slackhq.github.io/circuit/>).
- **Functions Compono does not have.** Results from a screen with
  `rememberAnsweringNavigator` (<https://slackhq.github.io/circuit/docs/navigation/>).
  Overlays that return a typed result
  (<https://slackhq.github.io/circuit/docs/overlays/>). A navigation interceptor.
  Shared-element transitions. In Compono a result is an `Output` of the child, so
  no extra part is needed.
- **Reported defects, and Compono.**
  [#2711](https://github.com/slackhq/circuit/issues/2711), saved state of popped
  records leaks into the saved output: Compono removes the saved bytes of a
  destroyed entry, and the law `saved_state_never_names_a_removed_entry` checks it.
  [#2410](https://github.com/slackhq/circuit/issues/2410), retained state starts
  again on return: a parked entry keeps its component.
  [#2272](https://github.com/slackhq/circuit/issues/2272), an effect does not
  restart after the application returns: `phase_changed` is called on every change.

### Workflow

- **Saved state.** A workflow gives a `Snapshot`, and the Compose integration saves
  it through `SaveableStateRegistry`
  (<https://github.com/square/workflow-kotlin/tree/main/workflow-ui/compose>).
- **Back.** `BackStackScreen` and overlays in `workflow-ui`. No page that was read
  describes predictive Back.
- **Side effects.** `runningWorker` and `runningSideEffect` in the render context.
  `testRender` checks which children and effects a render asks for
  (<https://github.com/square/workflow-kotlin/blob/main/workflow-testing/src/main/java/com/squareup/workflow1/testing/RenderTester.kt>).
- **UI.** Android Views and Compose in Kotlin, UIKit and SwiftUI in Swift
  (<https://github.com/square/workflow-swift>). No web UI.
- **Functions Compono does not have.** Render tests that assert the children a
  parent asks for. A render-idempotence checker. Tracing with a viewer.
- **Reported defects, and Compono.**
  [swift #275](https://github.com/square/workflow-swift/issues/275) and
  [#290](https://github.com/square/workflow-swift/issues/290), actions sent during
  propagation: Compono queues them and runs each to completion.
  [swift #289](https://github.com/square/workflow-swift/issues/289), observers are
  told before updates finish: a `Commit` is sent after the queue is empty.

### Navigation 3

- **Saved state.** `rememberNavBackStack` with `@Serializable` keys survives
  process death. Per-entry state comes from entry decorators
  (<https://developer.android.com/guide/navigation/navigation-3/save-state>).
- **Back.** The application owns the list and `NavDisplay` calls `onBack`.
  Predictive Back has its own transition, `predictivePopTransitionSpec`
  (<https://developer.android.com/guide/navigation/navigation-3/animate-destinations>).
- **Side effects.** None of its own. An entry under an overlay is capped at
  `STARTED` (<https://developer.android.com/guide/navigation/navigation-3/basics>).
- **UI.** Compose. Deep-link types exist for Kotlin Multiplatform from 1.2.0
  (<https://developer.android.com/jetpack/androidx/releases/navigation3>).
- **Functions Compono does not have.** Scenes that choose a layout from the back
  stack, such as list-detail and dialogs
  (<https://developer.android.com/guide/navigation/navigation-3/scenes>). Deep-link
  matchers that build a back stack. A result bus between entries.
- **Reported defects, and Compono.** The Google issue tracker asked for a sign-in,
  so these come from the release notes
  (<https://developer.android.com/jetpack/androidx/releases/navigation3>).
  b/516312097, duplicate keys during fast Back: Compono has no transition state in
  the core. b/483966071, entries under an overlay were not capped: in Compono a
  `Slot` does not lower the entry beneath it. The parent chooses, with
  `set_child_phase` or a `Slot` phase. This is a difference in the model and is listed under
  "Decisions for the owner".

## Defects

No critical defect and no security defect that an attacker could reach from outside
the process was found. Saved bytes and history state are the only outside input, and
D3 and D11 are the defects on that path.

| Id | Severity | Crate | File | Description | Test | Status |
|---|---|---|---|---|---|---|
| D1 | medium | compono | `src/router/items.rs`, `visible_rows` | A viewport that is infinite, or very large against the row height, overflowed an addition. That is a panic in a debug build and a wrong window in a release build | `audit.rs`: `a_viewport_too_large_to_count_shows_the_whole_list` | fixed |
| D2 | low | compono | `src/tree/back.rs`, `back_progress` | A progress value that is not a number was stored, so `BackGesture::progress` left its documented range | `audit.rs`: `gesture_progress_that_is_not_a_number_stays_in_range` | fixed |
| D3 | medium | compono | `src/router/stack/saved.rs`, `src/router/set.rs`, `src/router/slot.rs` | A restored router trusted the key counter and the keys in saved bytes. With a low counter or a repeated key, a new entry got the key of an old one and received its saved state | `audit.rs`: `a_saved_stack_with_a_low_counter_...`, `a_saved_set_with_a_low_counter_...`; `properties.rs`: `damaged_saved_state_never_gives_two_entries_one_key` | fixed, in `router/scope.rs` `repair_keys` |
| D4 | low | compono | `src/concurrency/sender.rs` | A `Sender` that outlived its tree kept accepting messages, up to the queue limit, into a queue nobody reads | `audit.rs`: `a_sender_that_outlives_its_tree_is_refused`; loom: `a_send_racing_with_the_trees_drop_is_dropped_or_refused` | fixed. `send` returns `SendError::Closed` |
| D5 | medium | compono | `src/tree/mod.rs` `destroy`, `src/tree/lifecycle.rs` `refresh`, `src/tree/back.rs` `back_target`, `src/persist.rs` `save_subtree` | Each recursed once per level of the component tree. A chain 200,000 deep overflowed the stack | `audit.rs`: `a_deep_chain_changes_phase_saves_and_is_destroyed_without_overflowing` | fixed. All four walk the tree's links with no recursion |
| D6 | low | compono | `src/tree/mod.rs` | A slot that had used every generation wrapped to generation 0, so the first id ever made for it matched the retired slot again | unit test `a_slot_that_runs_out_of_generations_never_matches_its_first_id` | fixed. The last generation is never given to an id |
| D7 | low | compono-ffi | `src/native_tree.rs` | A `NativeTree` dropped on another thread after its owning thread had ended left a record in a global list for ever, and every later call on any thread then took a global lock | unit test `a_tree_whose_thread_has_ended_leaves_no_record_behind` | fixed. Each owning thread has its own list, shared with its handles |
| D8 | low | compono-ffi | `src/native_tree.rs`, `Owned::deliver` | One call that changed a component and then removed it reported the removed component to `Platform::commit` | `native.rs`: `several_commits_in_one_call_never_name_a_removed_component` | fixed |
| D9 | low | compono-test | `src/time.rs` | `VirtualTime::sleep` and `advance` panicked on a duration too long to add | `harness.rs`: `a_delay_too_long_to_add_waits_forever_instead_of_overflowing` | fixed |
| D10 | low | compono-desktop | `src/state_file.rs`, `store` | A failed write or rename left `state.bin.tmp` behind | `desktop.rs`: `a_failed_store_leaves_no_temporary_file` | fixed |
| D11 | low | compono-web | `src/history.rs`, `start` | `index + 1` on an index read from history state overflows where `usize` has 32 bits, which is the browser | none. The overflow cannot occur on a 64-bit host, and the workspace has no test runner for `wasm32` | fixed by inspection, with `checked_add` |
| D12 | medium | compono | `src/router/*.rs` | A router value dropped or replaced while its parent lived left its components alive and hidden until the parent was destroyed, because `Drop` cannot reach the tree | `audit.rs`: `a_router_removed_before_its_owner_takes_its_components_with_it` | fixed by a new `destroy(self, cx)` on `Stack`, `Slot`, `Pages`, `Panels`, `Items` and `ChildSet`. A router that is only dropped still leaves its components, which the documentation of each `destroy` now says |
| D13 | medium | native/android | `ComponoTree.kt`, `settleGesture` | When the settle animation of a Back gesture was cancelled, as when the Activity is recreated in those 200 ms, the gesture never ended. `beginGesture` then refused every later gesture, and the enabled `PredictiveBackHandler` swallowed every Back press | `ComponoTreeTest.kt`: `aGestureWhoseAnimationIsCancelledStillEnds` | fixed |
| D14 | medium | compono | `src/router/stack.rs`, `slot.rs`, `set.rs` | A router made with `new` on a tree that held restored bytes gave its new entries the saved state of whichever entry had the same key in the earlier launch, and carried the rest in every later save | `audit.rs`: `a_router_that_starts_fresh_gives_no_entry_the_state_of_an_earlier_launch` | fixed. `new` calls the new `Cx::forget_router` |
| D15 | low | native/android | `ComponoTree.kt`, `beginGesture` | A Back press that arrives while the previous gesture is still settling is collected and then dropped, because `beginGesture` returns false | `ComponoTreeTest.kt`: `backDuringASettlingGestureEndsItAndStartsAnother` | fixed after the audit. The cause was in `ComponoTree`, not the composable, so no Compose UI test dependency was needed. A press while settling finishes that gesture and starts its own |
| D16 | low | compono | `src/router/scope.rs`, `repair_keys`, and each router's restore | Saved state of entries that a restored snapshot did not name, as after key repair, stayed in the tree and was carried into every later save | `audit.rs`: `saved_state_of_an_entry_the_snapshot_does_not_name_is_not_carried` | fixed after the audit with the new `Cx::keep_router_entries` |
| D17 | low | native/apple | `ComponoTree.swift`, `beginGesture`, `finishGesture` | The same dropped Back as D15 on iOS. Also, the completion of a settle animation ended whatever gesture was current, which could be a newer one | `ComponoTreeTests.swift`: `testBackDuringASettlingGestureEndsItAndStartsAnother` | fixed after the audit. Each gesture has a serial number that its completion checks |

The audit also looked at these areas and found no defect: `Rc` and `Arc` cycles (the
core has none; the cycle between a `WebSpawner` task and its cancel flag is broken
when the task ends or is cancelled); the timer table in `compono-web` (an entry is
removed when its `Sleep` completes or is dropped); listeners of `PageEvents` (removed
on drop; `forever` leaks them by design); task guards (dropped with the component,
and on delivery of the result); `RefCell` borrows in `compono-ffi`, `compono-test`
and `compono-web` (none is held across user code); the thread check of `NativeTree`
(`ThreadId` is never reused); decoding of hostile saved bytes (lengths are capped by
serde, and the law `changed_or_shortened_bytes_never_break_a_restore` covers it);
`StateFile` (written to a temporary file, synced, then renamed).

## Missing functions

| Function | Problem it solves | Who has it | Effort | Recommendation |
|---|---|---|---|---|
| Saving `Items` and `Panels` | A list lost its scroll window and a list-detail screen lost its open panels on restart | Decompose saves every navigation model (<https://arkivanov.github.io/Decompose/navigation/panels/overview/>) | small | **Added** in this release candidate |
| Removing a router before its parent | See defect D12 | Decompose removes a child context with its lifecycle (<https://arkivanov.github.io/Decompose/component/child-components/>) | small | **Added**: `destroy(self, cx)` |
| A transient router that does not save | Large or short-lived navigation should start fresh after process death | `transientNavStateSaver` (<https://arkivanov.github.io/Decompose/navigation/items/overview/>) | small | **Present after this release candidate.** An owner that does not return a router's snapshot from `save` gets a fresh router, and `new` now discards stale state (D14). No new type is needed |
| Replacing the main panel | A list-detail screen that changes its list | `activateMain` in Decompose 3.6.1 | small | **Added**: `Panels::set_main` |
| `spawn_local` for tasks that are not `Send` | In a browser a `fetch` future is not `Send`, so `cx.perform` cannot await it. GPUI and Dioxus also run tasks on their main thread | `wasm_bindgen_futures::spawn_local`; Crux runs commands on the core's own executor (<https://redbadger.github.io/crux/part-2/effects.html>) | medium | **Added** after the audit as `Cx::perform_local`, which needs no `Spawner` and no shared queue |
| Browser history for `Pages` and `Panels` | The address bar does not show the selected page or the open detail | `childPagesWebNavigation`, `childPanelsWebNavigation`, still experimental in 3.6.1 (<https://arkivanov.github.io/Decompose/navigation/web-navigation/>) | large | Add after 1.0. It is additive: new types beside `History` |
| Deep links on Android and iOS | A link should open a screen with the right screens beneath it | `handleDeepLink` (<https://arkivanov.github.io/Decompose/navigation/stack/deeplinking/>); Navigation 3 deep-link matchers (<https://developer.android.com/jetpack/androidx/releases/navigation3>) | small | **Moved to the core** after the audit: `compono::router::link`, with `Navigation::toward` turning a path into the change that shows it |
| Saved-state migration | An update that changes a `Saved` type starts every user from a clean state | None of the compared libraries has it. Circuit drops what it cannot restore (<https://slackhq.github.io/circuit/docs/navigation-persistence/>) | medium | Add after 1.0, as a new constructor beside `Tree::with_saved` |
| Adapters for Dioxus and Slint | Those toolkits have no adapter | none | medium each | Add after 1.0. Each needs a new dependency, so each needs the owner's agreement |
| Results from a screen | A picker returns a value to the screen that opened it | Circuit `rememberAnsweringNavigator` (<https://slackhq.github.io/circuit/docs/navigation/>), Navigation 3 `ResultEventBus` | none | Do not add. A child's `Output` already does this, with types |
| Back priorities | An overlay that must handle Back before deeper components | `BackCallback(priority)` (<https://arkivanov.github.io/Decompose/component/back-button/>) | medium | Do not add. The order "deepest, then most recently shown" has covered every showcase screen, and priorities make Back depend on registration order |
| Effects as data | Tests assert which effects a component asked for | Crux `Command` | large | Do not add. It is another architecture. `compono-test` covers the need with `VirtualTime` and a spawner the test controls |
| Accessibility data from components | Screen readers | none of the compared libraries has it in the logic layer | n/a | Do not add. A component already holds the labels its views share, and the UI toolkit owns the accessibility tree |

## Stability

The experimental list for Decompose was checked against the 3.6.1 artifact in the
Gradle cache (`decompose-jvm-3.6.1.jar`; the cache has no source JAR for 3.6.1, so the
class files were searched for the annotation) and against the 3.5.0 sources. The list
in the task is correct for 3.6.1. These are also experimental there and were missing
from it: `ChildItems`, `ItemsNavigator` and its extensions, `ChildPanels`,
`PanelsNavigator` and its extensions such as `activateMain` and `setMode`,
`PagesNavigator.setItems`, and `DecomposeExperimentFlags`.

| Part | 1 Example | 2 Tests | 3 Law | 4 Flat cost | 5 Saves | 6 Every UI | 7 No open defect | 8 No API change expected |
|---|---|---|---|---|---|---|---|---|
| `Items`, `ListView` | yes, on `Items` | yes: `more_routers.rs`, `saved_routers.rs` including `DuplicateKey` | yes: `a_list_has_a_component_for_each_row_in_its_window_and_no_other` | yes: `router_scaling`, 460 ns at 100 and at 1,000,000 keys | yes, new: `SavedItems` | yes: `ListView` in egui, iced and GPUI, `ChildList` in Swift and Compose | yes | yes |
| `Panels`, `Panel` | yes, on `Panels` | yes: `more_routers.rs`, `saved_routers.rs` | yes: `panels_focus_the_deepest_and_show_what_the_mode_has_room_for` | yes: `router_scaling`, 95 ns at every size | yes, new: `SavedPanels` | yes: `RouterView` in the three adapters, `ChildPanels` in Swift and Compose | yes | yes |
| `compono_web::history` | yes, on the module | yes: 11 tests in `tests/history.rs` | yes: `the_browser_always_ends_where_the_stack_is` | **no**. `sync` allocates nothing when the stack is unchanged, but it compares one key for each entry up to the current one, so it is linear in stack depth. No benchmark measures it | does not apply. It adopts a restored stack after a reload | does not apply. It is for the browser only | yes | yes for `Stack`. History for `Pages` and `Panels` would be new types |
| `Persist`, snapshots | yes, on `Persist` | yes: `design.rs`, `regressions.rs`, `audit.rs` | yes: `saved_state_never_names_a_removed_entry`, `damaged_saved_state_never_gives_two_entries_one_key`, `changed_or_shortened_bytes_never_break_a_restore`, and a restore at the end of every router law | does not apply to saving, which visits every live component. Restoring builds only what is on screen (`restore_builds_only_what_is_on_screen`) | yes | yes: `NativeTree::save` and `restore`, `save()` in Swift and Compose, `StateFile`, `SessionStore` | yes | yes. Migration would be a new constructor |
| `Cx::child`, `Cx::scoped`, `ChildSet` | yes, on `Cx::child`, `Cx::scoped` and `ChildSet` | yes: `design.rs`, `routers.rs`, `regressions.rs` | yes: `a_set_matches_every_desired_list_and_keeps_survivors`, `a_child_never_exceeds_its_parent_and_back_goes_to_the_last_shown_handler` | yes: `set_scaling`, an unchanged child costs 3 to 8 ns from 1,000 to 1,000,000 children | yes | yes: the showcase gallery and cards screens on each UI | yes | yes |

`compono_web::history` is the one part that does not meet every condition. Making
`sync` independent of depth needs the stack to report that it changed, which is a new
public method on `Stack`. The cost today is one integer comparison for each entry.

`Slot`, `Pages` and `Stack` were not in the list. They meet the same conditions: each
has an example, a law in `router_laws.rs` or `properties.rs`, and a line in
`router_scaling` or `navigation_cost`.

## Release candidate

- **Version.** The workspace version is `1.0.0-rc.1`. Dependencies between the crates
  name that version beside their path.
- **`publish`.** The nine crates under `crates/` can be published. The five under
  `examples/` have `publish = false`. Nothing was published.
- **Metadata.** Each crate has `description`, `readme`, `keywords`, `categories`,
  `rust-version` and a short `README.md`. `compono-web` builds its documentation for
  `wasm32-unknown-unknown`. `license` is `Apache-2.0`. `repository` is not set. See "Decisions for the owner".
- **Packaged files.** `cargo package --list` was run for each of the nine crates. Each
  lists its manifest, `README.md`, `src`, `tests` and, for `compono`, `examples`. No
  build output and no file from another crate is listed. Cargo warns only that the
  manifest has no repository.
- **`#[non_exhaustive]`.** `SendError`, `RestoreOutcome`, `NativeError`,
  `compono_desktop::Key`, `Convention`, `PageEvent`, `Address`,
  `compono_iced::Action`, and the structs `Commit` and `BackGesture`. `Phase`,
  `PanelsMode` and `Panel` stay exhaustive: three phases and three panels are the
  model, and applications match on them.
- **API Guidelines.** Every public type has `Debug`. Every error type has `Display`,
  `Error`, `Clone`, `Copy`, `PartialEq` and `Eq`. Fallible functions have an
  `# Errors` section. Getters have no `get_` prefix. `Handle` gained `Hash`, which
  `NodeId` already had. `restore` takes no fallback and `restore_or` takes one, and
  the routers follow that rule. No public item was removed or renamed.
- **New files.** `CHANGELOG.md`, `VERSIONING.md`, a `README.md` in each crate, the
  example `router_scaling`, and the tests `audit.rs`, `saved_routers.rs`,
  `router_laws.rs` and `tree_laws.rs`.
- **Updated.** `ARCHITECTURE.md`, `README.md`, `AGENTS.md`, the `compono-verify`
  skill and its copy, and CI, which gained a job on Rust 1.96.

### Final checks

Run after the last change to any source file.

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| The three `--no-default-features` clippy runs for `compono` | pass |
| `cargo test --workspace` | pass: 166 tests after the owner's decisions, 160 at the end of the audit |
| `cargo test -p compono --no-default-features` | pass |
| `cargo check -p compono --no-default-features --target thumbv7em-none-eabihf`, and the same with `--features serde` | pass |
| `cargo clippy -p compono-web -p showcase-web --target wasm32-unknown-unknown -- -D warnings` | pass |
| `cargo doc --workspace --no-deps` with `-D warnings`, and `compono-web` for `wasm32` | pass |
| loom | pass: 5 models, with `a_local_task_woken_from_another_thread_is_never_lost` added after the audit |
| `navigation_cost`, `set_scaling`, `router_scaling` | pass: flat. 134 to 165 ns and 3 allocations for a push and pop at depths 1 to 100 |
| `native/showcase/ffi` clippy | pass |
| `benchmarks/ffi-boundary`, `benchmarks/decompose-comparison/rust`: `cargo check` | pass |
| `swift test` | pass: 9 tests |
| `xcodebuild` of the iOS showcase for the simulator | pass |
| `gradle :compono:testDebugUnitTest :app:assembleDebug --rerun` | pass: 8 tests |

## Not done

| Item | Reason |
|---|---|
| `compono_web::history` cost independent of stack depth | Needs a new public method on `Stack` |
| Browser history for `Pages` and `Panels`, deep-link helper, migration, Dioxus, Slint | Recommended after 1.0. See "Missing functions" |
| `cargo miri test` | No library crate has `unsafe` code, and Miri is not installed |
| `boltffi pack` for Apple and Android | `boltffi` is not installed. The native showcase was assembled against the libraries packed earlier, so the Rust changes of this audit are not inside the assembled Android application. `native/showcase/ffi` was checked with clippy |
| Running on a simulator, an emulator, a device, or in a browser | Not run in this audit. The fixes are covered by unit tests only. `ARCHITECTURE.md` still lists what was run by hand before |
| A test for D11 | The overflow needs a 32-bit `usize`, and the workspace has no test runner for `wasm32` |
| A compiled example for `compono-gpui` | Every entry point needs a running GPUI application. `tests/host.rs` and `tests/redraw.rs` use GPUI's test support instead |
| An example on each public function | Each part and each crate has one compiled example. The 300 other public functions have prose only |
| The showcase does not save its `Items` window or its `Panels` | The library can now do it. Changing the showcase changes its saved schema and its three UIs, and was left out |
| The Composable Architecture: state saving, navigation and testing details | Its documentation site returned 404 |
| Google issue tracker for Navigation 3 | It asked for a sign-in. Release notes were used |

## Decisions for the owner

The owner decided these after the audit, on 2026-11-11:

1. **Licence: Apache-2.0.** `LICENSE` is at the root and linked into each crate, and
   `[workspace.package]` sets `license`.
2. **Repository URL.** Still open: the directory is not a git repository yet.
3. **`spawn_local`: before 1.0.0.** Added as `Cx::perform_local`. The tree polls such
   futures itself during `Tree::pump`, so no `Spawner` change and no shared queue of
   values that are not `Send` was needed.
4. **Compose UI test dependency: approved, not needed.** D15 was fixed and tested in
   `ComponoTree`. No dependency was added.
5. **Deep-link logic: moved to the core.** `Paths` and `Navigation` are now in
   `compono::router::link`, with `Navigation::toward` for links on any platform.
6. **A screen under a dialog stays `Focused`.** Kept. The documentation of `Slot` says
   so and how a parent lowers it.

Still open:

7. **A changed-stack signal on `Stack`** so that `History::sync` does not depend on
   depth. Recommendation: leave it for 1.1, since it is additive.
8. **Publishing order and names.** All nine crate names start with `compono`. Check
   that they are free on crates.io before the first publish. The order is `compono`,
   `compono-desktop`, `compono-test`, then the rest.
9. **`#[non_exhaustive]` on `Commit` and `BackGesture`.** This stops other crates
   from constructing them. Nothing in the workspace did. Undo it if hosts should
   build a `Commit` in their own tests.

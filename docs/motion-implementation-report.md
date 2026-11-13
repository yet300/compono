# Router animations: implementation report

## 1. Summary

All eight steps are done and every check in section 7 of the prompt passes. Two
things were not run: the Android emulator (none was running) and the web showcase in a
browser. There is a new crate, `compono-motion`. It is `no_std`, has no dependencies
and never allocates, and the egui, iced and GPUI adapters use it every frame. The
Swift package and the Compose library now have native `StackAnimation` types, and
their tests compare against the golden table that `compono-motion` writes.

The frame-by-frame measurement of a UIKit push confirmed the 30 % parallax. The shade
measured 12 %, not 10 %, so I corrected the presets. UIKit's push curve is also much
faster than SwiftUI's default spring. I left the timing as the design specifies; the
owner should decide whether to change it (see section 3).

`cargo test --workspace` went from 258 passing tests to 321, with no failures. The
Swift tests went from 16 to 26 and the Compose unit tests from 13 to 23.

## 2. Steps

### Step 1: direction of stack animations in Swift and Compose

**Change**
- New pure functions:
  - `native/apple/Compono/Sources/Compono/StackDirection.swift`: `stackDirection(shown:active:below:shownRemains:)`.
  - `native/android/compono/src/main/kotlin/compono/android/StackDirection.kt`: `stackDirection(...)`.
- `RouterViews.swift` and `RouterViews.kt` use these functions instead of comparing keys.
- `ComponoTree.swift` and `ComponoTree.kt` gain an internal `contains(node)`.
- `ChildPages` keeps ordering by key, through an internal page-order mode.
- The Compose `ChildStack` takes a new optional `node: ((C) -> Long)?`. The Android
  showcase passes it in `RootContent.kt` and `CountersContent.kt`.

**Tests added**
- `StackDirectionTests.swift` and `StackDirectionTest.kt`, 6 tests each: push, pop,
  pop of several entries, `bring_to_front`, `replace`, and a pop after
  `bring_to_front`.
- Against a stub that kept the old key comparison:
  - Swift failed 3 tests, among them `bring_to_front` ("back" is not "forward") and the pop after `bring_to_front`.
  - Kotlin failed `bringingAnOlderEntryToFrontMovesForward` and `aPopAfterBringToFrontMovesBack`.

**Existing tests changed:** none.

**Decisions**
- **Back only when the shown entry is gone and the new entry was below it.** The
  prompt's literal rule ("back only when the shown entry is no longer in the stack")
  would make `replace` a pop. The design's rule (A1) and Decompose make `replace` a
  push. I used the design's rule, which matches the `replace` test the prompt asks for.
- **"Was below"** means one of two things: the new entry is the `previous` entry that
  the view saw before the change, or it has an older key. One case is still judged
  wrong: `pop_to` several levels after a `bring_to_front` reorder. It needs the whole
  key list, which the native views do not receive.
- **The view asks whether the shown entry still exists** through the tree's
  `contains(node)`. Swift's `ComponoChild` already carries `node`. Kotlin's
  `ChildStack` had no way to get an entry's address, hence the new optional `node`
  parameter.
  - Without `tree` and `node`, Kotlin treats a change as back only when it reveals `previous`.
  - Without a tree, Swift always moves forward. These views are tabs, which is correct for them.

**How the owner can see it:** run the native showcase. Switch tabs in Counters
(`bring_to_front`); it now slides forward. Back still slides backward.

### Step 2: `compono-motion`

**Change**
- New crate `crates/compono-motion`:
  - Modules: `curve`, `spring`, `timing`, `style`, `transition`, `back`, `preset`, `change`, `state`, `math`.
  - Items: `Easing`, `Spring`, `Timing`, `LayerStyle`, `Size`, `Direction`, `Transition`, `SwipeEdge`, `BackEvent`, `StackFrame`, `BackAnimation`, `MotionPreference`, `Motion` (`material`, `cupertino`, `desktop`, `web`, `none`, `with_timing`, `with_transition`, `for_preference`), `StackChange::between`, `TransitionState`, `Frame`, `Layer`.
- `examples/golden/` writes `golden/motion.csv`: 984 rows covering 4 presets × 3 preferences × push, pop, back, back_right, back_finish and back_touch.
- New `README.md` for the crate. The crate is added to the layout tables in `README.md`, `ARCHITECTURE.md` (crate map and a new "Motion" section) and `AGENTS.md`, and to the `AGENTS.md` check list (`cargo check -p compono-motion --target thumbv7em-none-eabihf`).

**Tests added**
- `tests/values.rs` (9): easing points, tween timing, and the exact layers of `Slide`, `Fade`, `Scale`, `SharedAxisX`, `SlideUp`, `None`, `Custom`, swipe, predictive Back, `Fade` and `None` Back.
- `tests/springs.rs` (4): integration against the closed form within 1e-5, settle times, degenerate springs.
- `tests/change.rs` (10): every `StackChange::between` case, including `bring_to_front`, `push_to_front`, `reset`, a pop after a reorder, and key wrap.
- `tests/state.rs` (8): `TransitionState` on virtual time.
- `tests/golden.rs` (2): the checked-in table matches the presets.
- `src/math.rs` (2 unit tests) and the doc example.
- Before the change, every test failed on its assertions against stub bodies, for example "slide … is not 0.75". The doc test failed too. Only `an_unchanged_stack_is_no_change` passed against the stub, as it should.

**Existing tests changed:** none.

**Decisions and corrections to the design**
- **Fourth-order Runge-Kutta instead of semi-implicit Euler.** The prototype's
  integrator differs from the closed form by up to 0.009 for the Material spring.
  The design claimed "under 0.1 %". RK4 at 1 ms stays within 1e-6.
- **Settle times, corrected by the accurate integrator (checked against the closed form):**

  | Spring | Design ("measured") | Correct |
  |---|---|---|
  | Material | 0.233 s | 0.3166 s |
  | Cupertino | 0.817 s | 0.8082 s |

- **Reference value for `FastOutSlowIn(0.5)`:** 0.7756 by double-precision bisection, not the prototype's 0.7742.
- **`Transition::layer` takes a `Size`**, because `SharedAxisX` shifts by dp. `TransitionState::set_size` supplies it.
- **`Transition` has no `PartialEq`**, because `Custom` holds a function pointer.
- **`Motion::web()` uses CSS `ease`** (0.25, 0.1, 0.25, 1) as a literal. The design gives no curve, and a literal avoids a new public constant.
- **`Scale` also fades the front entry**, as Decompose's `fade() + scale()`.
- **`BackAnimation::Fade` is added.** The design's reduced-motion Back ("opacity only") needs a variant.
- **`TransitionState` restarts on a change during a transition** instead of retargeting. This is the simplest correct behaviour.
- **`for_preference` is in this step** because it is part of P1. Step 7 adds its tests.

**How the owner can see it**
```bash
cargo test -p compono-motion
```
```bash
cargo run -p compono-motion --example golden
```

### Step 3: stack transitions in egui

**Change**
- New `crates/compono-egui/src/motion/mod.rs` and `motion/layers.rs`.
- `navigation.rs`: `Stack::view` goes through `StackView`.
- `lib.rs` exports `StackView` and `set_motion`.
- `examples/showcase-egui/src/app.rs` sets `Motion::desktop()`, or `Motion::web()` on `wasm32`.

How it works: each frame the adapter copies the shapes of the active entry from every
egui layer it painted. After a change it replays that copy with the leaving layer's
style. The default is `Motion::desktop()`. `set_motion(ctx, motion)` sets the motion
for a context, and `StackView::new(&stack).motion(m)` sets it for one stack.

**Tests added:** `crates/compono-egui/tests/motion.rs`, with a push fading by
default, a pop replaying the destroyed entry, and the context's motion. Before the
change: "assertion `left == right` failed" (one entry drawn instead of two).

**Existing tests changed:** none.

**Decisions**
- **The covered entry is replayed on a push too**, instead of drawn live: its component is `Hidden`, and a replay takes no input.
- **The motion lives in the egui `Context`**, so the one web/desktop switch in `App::new` serves `showcase-web` too. `showcase-egui` gains a dependency on `compono-motion`.

**How the owner can see it:** run `cargo run -p showcase-egui`, then push and pop in
Counters; screens fade over 150 ms. In a browser, `examples/showcase-web/build.sh`
gives 250 ms fades.

### Step 4: key accessors in the core

**Change**
- `Slot::active_key`, `Pages::child(index)` (and `Pages::active` now uses it).
- `Panels::keys` with the new `PanelKeys`.
- The `dead_code` expectation on `Slot`'s key is removed.
- Lines added to `CHANGELOG.md` and `skills/compono-navigation/SKILL.md`.

**Tests added:** `crates/compono/tests/router_keys.rs` (3 tests), covering each
accessor after `show`, a repeated `show`, `close`, `select`, `set_routes`,
`set_details`, `set_extra`, `set_mode`, `pop` and `set_main`. Against the stubs, all
three failed on their assertions.

**Existing tests changed:** none.

**Decisions**
- **`PanelKeys` is a public struct with public `Option<u64>` fields**, the smallest form of "a key per panel visit".

**How the owner can see it:** nothing changes on screen. Run
`cargo test -p compono --test router_keys`.

### Step 5: `Slot`, `Pages` and `Panels` in egui

**Change**
- `motion/mod.rs` gains `slot`, `pages` and `panels`, and `navigation.rs` routes each `RouterView` through them.
- A `Slot` fades.
- `Pages` slide by order, full width.
- `Panels` in `PanelsMode::Single` slide the deepest panel in with a 0.3 parallax. Wider modes do not move.
- The replay covers every layer an entry paints, so a dialog in its own `egui::Modal` layer fades out.

**Tests added:** `crates/compono-egui/tests/motion_routers.rs` (4 tests): a dialog
fading in and out, pages sliding both ways, narrow panels sliding in and out, wide
panels not moving. Before the change, 3 failed and the wide-panels test passed,
which is correct, since it asserts nothing moves.

**Existing tests changed:** none.

**Decisions**
- **A router-specific transition, with the context motion's timing:** `Slot` fades, `Pages` slide, `Panels` slide.
- **A test dialog turns off egui's own `Area` fade-in**, which multiplies with ours. In the showcase, egui's `Modal` fade-in still runs on top of ours when a dialog opens.

**How the owner can see it:** in `cargo run -p showcase-egui`:
- Menu, open a dialog: it fades in and out.
- Book: pages slide.
- Articles, with a narrow window: the details panel slides in.

### Step 6: iced and GPUI

**Change**
- **iced:**
  - `crates/compono-iced/src/motion.rs`: the `Animated` widget, with its state in the widget tree and time from `RedrawRequested`.
  - No subtree opacity in iced, so a fading entry is covered with the theme's background in a layer above it.
  - Every `RouterView` uses it. `compono_iced::set_motion` is new.
  - `Cargo.toml` enables iced's `advanced` feature.
- **GPUI:**
  - `crates/compono-gpui/src/motion.rs`: the `Animated` element, with its state in GPUI element state and time from the executor's clock.
  - Offsets and opacity through a wrapping `div`.
  - Every `RouterView` uses it. `compono_gpui::set_motion` is new.

**Tests added**
- `crates/compono-iced/src/motion_tests.rs`: 5 tests drawing into a recording renderer (push/pop, `bring_to_front`, slot fade, pages order, wide panels and `Motion::none`).
- `crates/compono-gpui/tests/motion.rs`: 4 tests with `TestAppContext` and `advance_clock` (stack, pages, slot, panels).
- Before the change:
  - GPUI: the stack and pages tests failed ("0 is not 960"). The slot test passed, since it checks only position.
  - iced: only the pages test failed (on a stub `page_motion`). I wrote the widget before its four other tests, so those never failed first.

**Existing tests changed:** none. The UI tests pass: `showcase-iced` (2) and `showcase-gpui` (4).

**Decisions**
- **Simpler pop (owner's decision 1).** The leaving entry disappears at once and only the revealed entry moves in.
- **The same applies to a push:** only the new entry moves in, over the background. Drawing the covered entry would mean building its view on every frame, since a view cannot know a transition is running. This differs from the design's A5, which drew both on a push.
- **GPUI does not apply `scale`**, because it has no subtree transform. No desktop preset scales.
- **The motion is process-global in iced and GPUI** (a `Mutex`), because neither has a context object to hold it.
- **The GPUI element id is the router's address.** This keeps sibling routers apart.
- **The four routers' change rules are in each adapter:** about 30 lines, three times. `compono-motion` cannot see the routers, and making them public would grow its API.

**How the owner can see it:** `cargo run -p showcase-iced` and
`cargo run -p showcase-gpui`. Push in Counters (the new screen fades in), open a
dialog, and turn pages.

### Step 7: reduced motion

**Change**

| Where | Change |
|---|---|
| egui | `compono_egui::set_motion_preference(ctx, pref)` |
| iced | `compono_iced::set_motion_preference(pref)` |
| GPUI | follows `App::reduce_motion` |
| `compono-web` | `prefers_reduced_motion()` via `matchMedia`; web-sys gains the `MediaQueryList` feature |
| `showcase-egui` | `App::reduce_motion(ctx, reduced)` |
| `showcase-web` | passes `prefers_reduced_motion()` to the app |
| SwiftUI `ChildStack` | reads `accessibilityReduceMotion` and uses `StackAnimation.reduced()` (step 8's type) |
| Compose `ChildStack` | reads `Settings.Global.ANIMATOR_DURATION_SCALE` through `motionPreference(scale)`; 0 means `Off` |

**Tests added**
- `crates/compono-motion/tests/presets.rs` (4): each preset's values, and `Full`, `Reduced` and `Off` for every preset. These passed at once because `for_preference` came in step 2. A mutation (`Off` kept the preset, `Reduced` kept the Back) made two of them fail, and the file was restored.
- egui: `reduced_motion_fades_in_place_and_no_motion_switches_at_once`. It failed before the change, because the entries still slid (x −80 instead of 10).
- iced: `the_preference_applies_to_the_application_motion`. It failed before the change.
- GPUI: `gpuis_reduce_motion_setting_fades_in_place`. It failed before the change.
- Kotlin: `removeAnimationsMeansNoMotion`. Swift: `testReducedMotionFadesInPlace`.

**Existing tests changed:** none.

**Decisions**
- **egui and iced take the preference from the application.** `showcase-egui` on the desktop passes `false`, because egui reads no system setting.
- **On Android, scale 0 means `Off`.** The system has no "reduced" setting.
- **On iOS, Reduce Motion means `Reduced`.** iOS has no "off".

**How the owner can see it**
- In a browser, turn on the system's Reduce motion and load `showcase-web`: pushes fade.
- On iOS, Settings › Accessibility › Motion › Reduce Motion: the stack fades instead of sliding.
- On Android, Remove animations: stacks switch at once.

### Step 8: native presets and Material predictive Back

**Change**
- **Swift:**
  - New `Motion.swift`: `StackTransition`, `BackGestureStyle`, `StackAnimation` as a struct with `transition`, `animation` and `back`, plus `.platform`, `.slide`, `.fade`, `.none` and `reduced()`, and the layer formulas.
  - `ChildStack` uses them. The overlay paints the shade.
  - `ComponoTree.finishGesture(commit:speed:)` settles on `interpolatingSpring(Spring(response: 0.55, dampingRatio: 1), initialVelocity:)` at the finger's speed. `Lifecycle.swift` passes `value.velocity.width / width`.
- **Kotlin:**
  - New `Motion.kt`: `StackTransition`, `BackAnimation` (`Predictive`, `Swipe`, `Fade`, `None`), `StackAnimation(transition, spec, back)` with `Platform`, `Slide`, `Fade`, `None` and `forPreference`, and the formulas.
  - `ChildStack` defaults to Material (shared axis, `spring(0.9f, 700f)`, predictive Back) with corners, scale and shade.
  - `BackGestureState` gains `finish`, the edge and the touch's travel. `updateGesture(progress, swipeEdge, touchY)` takes them from `BackEventCompat`.
  - A commit animates `finish` instead of `progress`.
  - `build.gradle.kts` declares the golden CSV as a test input.

**Tests added**
- `MotionTests.swift` (4) compares the Cupertino preset (full and reduced) with every push, pop and Back row of the golden table, using SwiftUI's own `Spring.value` and `UnitCurve`. It also tests `reduced()` and the release velocity.
- `MotionTest.kt` (4) compares the Material preset (full, reduced and off) with every push, pop and Back row, using Compose's `TargetBasedAnimation`. It also tests `motionPreference`.
- On the first run the Kotlin test failed on a real mismatch in its own rule for zero-length tweens, which I fixed in the test.
- I wrote `Motion.swift` before its test. A mutation (parallax 0.25) made 17 assertions fail, and the file was restored.

**Existing tests changed:** none. The existing `ComponoTree` tests pass with the new settle.

**Measurement:** see section 3. The shade in `compono-motion`, `Motion.swift` and
`Motion.kt` changed from 0.1 to 0.12, and the golden table was regenerated.

**Decisions**
- **`StackAnimation.none` in Swift keeps a swipe Back**, as before.
- **The Swift layers are interpolated by SwiftUI between their end values**, which is exact for the iOS slide. Compose evaluates each frame.
- **I did not run Android on the emulator.** `adb devices` listed none.

**How the owner can see it**
- iOS: build `native/showcase/ios` and run it on the simulator. A push slides with a 12 % shade, and a released swipe keeps its speed.
- Android:
  ```bash
  (cd native/showcase/android && gradle :app:installDebug)
  ```
  with `ANDROID_SERIAL=emulator-5554`. Swipe Back: the screen shrinks toward the far edge with rounded corners.

## 3. Measurements

### iOS push, frame by frame

- **Setup:** simulator `F5A2CD3E-6FF7-460A-889D-9126C59F45E9` (iOS 26, "iPhone Duo", primary display 1398 × 2034 px). Settings › General, a `UINavigationController` push. Recorded with `simctl io recordVideo`, 95 frames. Each frame was fitted as the start frame (shifted left and shaded) left of an edge, beside the end frame shifted right of that edge.
- **Covered screen offset:** −0.300 of the width per unit of progress, at every frame from 11 % to 99.7 % of the push. This matches the observed 0.3.
- **Shade:** about 0.12 × progress (0.117–0.127 between 26 % and 93 %), reaching 0.12–0.14 near the end. The design's observed value is 0.1, so I **corrected it to 0.12**.
- **Duration:** 99 % at 0.30 s and 99.7 % at about 0.37 s after the push starts. It fits a critically damped spring with a 0.29 s response (stiffness about 464) that settles within 0.1 % in 0.43 s.
- **Not changed:** SwiftUI's `Animation.default`, which the design uses (response 0.55 s, 0.81 s to settle), is about twice as slow. The design took that timing from Apple's documentation, not from observation, so I left it.
- **Owner's call:** if the Cupertino preset should match UIKit, the change is `Spring::from_response(0.29, 1.0)`.
- **Owner's decision (2026-11-13):** match UIKit. Two more pushes measured with the same
  method fit responses of 0.296 s and 0.28 s, so `Spring::CUPERTINO` is now
  `Spring::from_response(0.29, 1.0)`, settling in 0.43 s; the Swift package uses the same
  spring for pushes and for a released swipe.

### Scaling, after step 4 (release, `panic = abort`)

**`navigation_cost`**

| Case | Depth 1 | Depth 10 | Depth 100 | Depth 10 000 |
|---|---|---|---|---|
| push and pop | 183 ns | 150 ns | 151 ns | — |
| push and pop, named stack | 151 ns | 153 ns | 153 ns | — |
| hide and show the owner | 42 ns | — | 42 ns | 44 ns |

Every push and pop costs 3 allocations.

**`set_scaling`, per child**

| Children | Create | Unchanged | Reverse | Remove half |
|---|---|---|---|---|
| 1 000 | 193 ns | 5 ns | 43 ns | 67 ns |
| 10 000 | 190 ns | 6 ns | 49 ns | 129 ns |
| 100 000 | 147 ns | 13 ns | 45 ns | 56 ns |
| 1 000 000 | 100 ns | 4 ns | 65 ns | 44 ns |

**`router_scaling`**

| Entries | Select page | Scroll one row | Details open and close | Dialog open and close |
|---|---|---|---|---|
| 100 | 322 ns | 653 ns | 103 ns | 76 ns |
| 10 000 | 192 ns | 622 ns | 102 ns | 76 ns |
| 1 000 000 | 189 ns | 626 ns | 104 ns | 89 ns |

No number grows with size. The differences are within the noise that
`ARCHITECTURE.md` describes.

## 4. Checks

Final run, after all steps:

| Command | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass (the only note is the existing future-incompatibility notice for dependency `block 0.1.6`) |
| `cargo clippy -p compono --no-default-features -- -D warnings` | pass |
| `cargo clippy -p compono --no-default-features --features std -- -D warnings` | pass |
| `cargo clippy -p compono --no-default-features --features serde -- -D warnings` | pass |
| `cargo test --workspace` | 321 passed, 0 failed (baseline before any change: 258 passed, 0 failed) |
| `cargo test -p compono --no-default-features` | 62 passed, 0 failed |
| `cargo check -p compono --no-default-features --target thumbv7em-none-eabihf` | pass |
| `cargo check -p compono-motion --target thumbv7em-none-eabihf` | pass |
| `cargo clippy -p compono-web -p showcase-web --target wasm32-unknown-unknown -- -D warnings` | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p compono-web --no-deps --target wasm32-unknown-unknown` | pass |
| `cargo test -p compono --release --test allocations` | 4 passed, 0 failed |
| `navigation_cost`, `set_scaling`, `router_scaling` | flat (section 3) |
| `scripts/sync-skills.sh --check` | pass |
| `(cd native/showcase/ffi && cargo clippy --all-targets -- -D warnings)` | pass |
| `(cd native/apple/Compono && swift test)` | 26 passed, 0 failed |
| `gradle :compono:testDebugUnitTest :app:assembleDebug` | 23 passed, 0 failed; app assembled |
| `xcodebuild` of `native/showcase/ios` for the simulator | BUILD SUCCEEDED |
| UI tests `showcase-egui`, `showcase-iced`, `showcase-gpui` | 2, 2 and 4 passed (inside `cargo test --workspace`) |

## 5. Changed files

`git status --short` at the end. Before I started it showed only `docs/design/` and
the three files in `docs/prompts/`.

```
 M .agents/skills/compono-native/SKILL.md
 M .claude/skills/compono-native/SKILL.md
 M AGENTS.md
 M ARCHITECTURE.md
 M CHANGELOG.md
 M Cargo.lock
 M README.md
 M crates/compono-egui/Cargo.toml
 M crates/compono-egui/src/lib.rs
 M crates/compono-egui/src/navigation.rs
 M crates/compono-gpui/Cargo.toml
 M crates/compono-gpui/src/lib.rs
 M crates/compono-gpui/src/navigation.rs
 M crates/compono-iced/Cargo.toml
 M crates/compono-iced/src/lib.rs
 M crates/compono-iced/src/navigation.rs
 M crates/compono-web/Cargo.toml
 M crates/compono-web/src/lib.rs
 M crates/compono-web/src/page/events.rs
 M crates/compono-web/src/page/mod.rs
 M crates/compono/src/router/pages.rs
 M crates/compono/src/router/panels.rs
 M crates/compono/src/router/slot.rs
 M examples/showcase-egui/Cargo.toml
 M examples/showcase-egui/src/app.rs
 M examples/showcase-web/src/web.rs
 M native/android/compono/build.gradle.kts
 M native/android/compono/src/main/kotlin/compono/android/ComponoTree.kt
 M native/android/compono/src/main/kotlin/compono/android/Lifecycle.kt
 M native/android/compono/src/main/kotlin/compono/android/RouterViews.kt
 M native/apple/Compono/Sources/Compono/ComponoTree.swift
 M native/apple/Compono/Sources/Compono/Lifecycle.swift
 M native/apple/Compono/Sources/Compono/RouterViews.swift
 M native/showcase/android/app/src/main/kotlin/compono/showcase/ui/counters/CountersContent.kt
 M native/showcase/android/app/src/main/kotlin/compono/showcase/ui/root/RootContent.kt
 M skills/compono-native-apps/SKILL.md
 M skills/compono-native-apps/references/compose.md
 M skills/compono-native-apps/references/swiftui.md
 M skills/compono-navigation/SKILL.md
 M skills/compono-ui/references/egui.md
 M skills/compono-ui/references/gpui.md
 M skills/compono-ui/references/iced.md
?? crates/compono-egui/src/motion/
?? crates/compono-egui/tests/motion.rs
?? crates/compono-egui/tests/motion_routers.rs
?? crates/compono-gpui/src/motion.rs
?? crates/compono-gpui/tests/motion.rs
?? crates/compono-iced/src/motion.rs
?? crates/compono-iced/src/motion_tests.rs
?? crates/compono-motion/
?? crates/compono/tests/router_keys.rs
?? docs/design/
?? docs/motion-implementation-report.md
?? docs/prompts/exact-research.md
?? docs/prompts/motion-design.md
?? docs/prompts/motion-implement.md
?? native/android/compono/src/main/kotlin/compono/android/Motion.kt
?? native/android/compono/src/main/kotlin/compono/android/StackDirection.kt
?? native/android/compono/src/test/kotlin/compono/android/MotionTest.kt
?? native/android/compono/src/test/kotlin/compono/android/StackDirectionTest.kt
?? native/apple/Compono/Sources/Compono/Motion.swift
?? native/apple/Compono/Sources/Compono/StackDirection.swift
?? native/apple/Compono/Tests/ComponoTests/MotionTests.swift
?? native/apple/Compono/Tests/ComponoTests/StackDirectionTests.swift
```

## 6. Not done

- **Android was not run on the emulator.** No device was attached (`adb devices` was empty), and booting one is not among the checks. The predictive Back is covered by the golden-table unit tests only.
- **`showcase-web` was not run in a browser.** It is checked by the `wasm32` clippy run only. `prefers_reduced_motion` cannot run natively.
- **The new SwiftUI and Compose animations were not looked at on a screen.** Only the UIKit reference push was measured.
- **The Cupertino timing is unchanged** although UIKit's push is faster (section 3). This needs the owner's decision.
- **Remaining gaps by design:**
  - In iced and GPUI the covered entry is not drawn during a push (step 6).
  - GPUI does not apply scale.
  - One case is still judged wrong in Swift and Kotlin: `pop_to` several levels after a `bring_to_front` reorder (step 1).
- **Golden-table coverage:** the Swift test covers the Cupertino rows and the Kotlin test the Material rows. The desktop and web rows are checked only in Rust.

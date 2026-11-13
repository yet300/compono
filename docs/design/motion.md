# Compono motion design

Repository state read: `compono` at `7fccec1` (main). Decompose at `2cc1a4c` (2026-11-10).
Marks used below: **Fact** (read in code or a guideline, with its source), **Measured**
(from the experiment in section "Architecture, A7"), **Recommendation** (my opinion).

## Summary

One shared design can serve every toolkit, but not one shared *runtime*. The motion
language (which layers move, by how much, along which curve) is the same data for all
six UIs and belongs in one new crate, `compono-motion`, as plain values and pure
functions with no `std` and no dependency. The three Rust adapters (egui, iced, GPUI,
and egui on the web) call it every frame and only apply the resulting `LayerStyle`; this
removes about three times more code than the crate adds. SwiftUI and Compose keep their
own animation engines and their own small formulas, and are held to the shared values by
golden tables generated from `compono-motion` tests, because their packages must not
depend on generated bindings and their engines already do springs, velocity and
interruption better than per-frame calls would. **Recommendation:** adopt this hybrid
(Option 4 below). The largest risk is not the curves but the leaving entry: a component
is destroyed at once, and immediate-mode Rust UIs cannot redraw a destroyed component.
egui can replay the entry's last frame (proved by an experiment); iced and GPUI have no
such mechanism, so a pop in those two needs either a reduced animation or an owner
decision to let the tree keep a departing value until the UI releases it.

## Motion language

### M1. Transitions each router needs

| Router | Change | What moves | In Decompose (commit `2cc1a4c`) |
|---|---|---|---|
| Stack | push, `push_new`, `push_to_front` | new entry enters at the front, old one goes to the back | Yes: `Direction.ENTER_FRONT` / `EXIT_BACK` (`extensions-compose/.../stack/animation/AbstractStackAnimation.kt:102-103`) |
| Stack | pop, `pop_to`, `pop_while` | the top entry exits at the front, the revealed one enters from the back | Yes: `EXIT_FRONT` / `ENTER_BACK` (`AbstractStackAnimation.kt:94-97`) |
| Stack | replace, `replace_all`, `reset` | as a push; the old entry is destroyed | Yes, treated as a push (the "else" branch, `:101-103`) |
| Stack | `bring_to_front` | as a push; the old entry stays alive behind | Yes, as a push, because the new stack is not smaller |
| Stack | predictive Back | top entry follows the gesture, previous entry under it | Yes: three styles, `androidPredictiveBackAnimatableV1/V2`, `materialPredictiveBackAnimatable` (`.../predictiveback/`) |
| Slot | show, close | the entry fades (dialog) or slides up (sheet) | No Compose slot composable exists in `extensions-compose*` at this commit |
| Pages | select | slide by index order; swipe between neighbours | Partly: `ChildPages` is a `HorizontalPager`; programmatic select animates only with `PagesScrollAnimation.Default`, default is `Disabled` (`pages/ChildPages.kt:40`, `:168-170`) |
| Panels | open/close details or extra | in `Single` mode: as a stack push/pop; in `Dual`/`Triple`: the new panel slides in beside | Yes, experimental: `ChildPanelsAnimators` gives a `StackAnimator` per panel and mode (`experimental/panels/ChildPanelsAnimators.kt`) |
| Panels | change of mode | panels resize; one appears or disappears | No dedicated animation; layout jumps |
| Items | insert, remove, move | rows fade and shift | No; Compose's `LazyColumn` `animateItem` does it outside Decompose |
| ChildSet | reconcile | application-defined | No |

**Recommendation:** Compono implements Stack, Slot, Pages and Panels. Items and ChildSet
get no shared motion: a list's insert and remove animation belongs to the toolkit's list
(`LazyColumn.animateItem`, SwiftUI `List`/`ForEach` transitions, egui and iced have none),
and no router needs more.

### M2. Default motion on each platform

| Platform | Push / pop | Back gesture settle | Slot (dialog) | Pages | Source |
|---|---|---|---|---|---|
| Android (Material 3) | 300 ms (`DurationMedium2`), easing Emphasized `cubic(0.2, 0, 0, 1)`, or the M3 standard spatial spring: damping 0.9, stiffness 700 (settles to 0.1 % in **0.23 s**, measured) | Gesture progress through `STANDARD_DECELERATE` `(0, 0, 0, 1)`; commit/cancel duration not stated by the guide | Fade 150–200 ms (`DurationShort3/4`) | `HorizontalPager` fling | M3 tokens: `MotionTokens.kt:24-49`, `StandardMotionTokens.kt:20-31`, `ExpressiveMotionTokens.kt:22-33` in `org.jetbrains.compose.material3:material3:1.10.0-alpha05` sources; Android predictive Back guide |
| Compose default | `tween()` = 300 ms, `FastOutSlowInEasing` `(0.4, 0, 0.2, 1)` | — | `fadeIn()`/`fadeOut()` default spring | — | `animation-core 1.12.0`: `AnimationSpec.kt:39`, `Easing.kt:50` |
| iOS | Spring. SwiftUI `Animation.default` = `spring(response: 0.55, dampingFraction: 1.0)` (settles to 0.1 % in **0.82 s**, measured; visually done near 0.5 s). Covered entry shifted about 30 % left | Follows the finger; release continues the spring with the finger's velocity | Sheet: slides up; alert: fade + scale | `TabView(.page)` | Apple doc for `Animation.default` (read in a browser). **Not documented by Apple:** `UINavigationController`'s push duration and the 30 % parallax; the 30 % value is the observed convention and what `RouterViews.swift:13` uses |
| macOS | No slide for most window content; short cross-fade or none | No gesture (trackpad swipe is a scroll) | Sheet drops from the title bar | — | Observed convention; not stated in the HIG "Motion" page, which gives no navigation values |
| Desktop egui / iced / GPUI | Toolkit defaults: egui `animation_time` 0.2 s, linear (`egui-0.37.0/src/style.rs:1550`, `context.rs:3711`); iced `Animation` 100 ms (`iced_core-0.14.0/src/animation.rs:27`); GPUI requires a duration | — | — | — | Files cited |
| Web | View Transitions: cross-fade, 0.25 s in the user-agent stylesheet | Browser Back has no in-page gesture | — | — | CSS View Transitions spec UA stylesheet (not re-read for this document; verify before shipping) |

Predictive Back in-app values from the Android design guide (Fact, read at
developer.android.com/design/ui/mobile/guides/patterns/predictive-back):

| Value | Full-screen surface | Shared-element surface |
|---|---|---|
| Exit scale | 100 % → 90 % | to 90 %, minimum 90 % |
| Enter scale | 110 % → 100 % | — |
| Exit fade | 100 % → 0 %, reaching 0 at 35 % progress | — |
| Enter fade | 0 % → 100 %, starting at 35 % | — |
| Edge gap | — | 8 dp; X shift `(width / 20) − 8 dp`; Y shift `(height / 20) − 8 dp` |
| Exit interpolator | `(.1, .1, 0, 1)` | — |
| Corner radius, scrim, commit/cancel timing | **Not stated** | **Not stated** |

Decompose fills the unstated values (Fact): corner radius 16 dp grown with progress
(`predictiveback/LayoutCorners.kt:20,33`); enter scrim black 25 % (`MaterialPredictiveBackAnimatable.kt:77`);
commit continues progress by the release velocity, capped at 5 progress units per second
(`PredictiveBackAnimatable.kt:78`, `MaterialPredictiveBackAnimatable.kt:132-134`).

Current Compono values (Fact): Swift and Compose stacks use 280 ms, parallax 0.3,
ease-in-out or `tween` (`RouterViews.swift:11-13`, `RouterViews.kt:46-49`); settle 200 ms
(`ComponoTree.swift:196`, `ComponoTree.kt:39`); the iOS swipe commits past 40 % or when
the predicted end passes 60 % (`Lifecycle.swift:36-37`).

**Recommendation for defaults:**

| Preset | Transition | Timing | Back gesture |
|---|---|---|---|
| `Motion::material()` (Android, and default for egui on Android-like targets) | `SharedAxisX` 30 dp, fade-through at 35 % | spring 0.9 / 700 | `Predictive` (guide values; 16 dp corners and 25 % scrim from Decompose) |
| `Motion::cupertino()` (iOS) | `Slide { parallax: 0.3, dim: 0.1 }` | spring response 0.55 / damping 1.0, i.e. stiffness 130.5 | `Swipe` |
| `Motion::desktop()` (egui, iced, GPUI on desktop, macOS) | `Fade` | tween 150 ms, standard easing | none |
| `Motion::web()` | `Fade` | tween 250 ms | none |

### M3. How a predictive Back gesture moves the screens

Progress `p` goes 0 → 1 with the finger. At release the UI either commits (animate to 1,
then `back_commit`) or cancels (animate to 0, then `back_cancel`). Fact for the current
code: progress never crosses into Rust (`ComponoTree.swift:167-170`, `ComponoTree.kt:41`).

| Value | iOS swipe | Android in-app (guide) | Decompose Material | Decompose Android V2 |
|---|---|---|---|---|
| Top offset X | `p` × width | `(1 − scale)/2 − 8 dp·p`, toward the far edge | same | same, then to +20 % on commit |
| Top offset Y | 0 | follows finger Y, limit `height/20 − 8 dp` | same, factor `min(3p, 1)` | same |
| Top scale | 1 | 1 → 0.9 | `1 − p/10` | 1 → 0.9 |
| Top opacity | 1 | 1 → 0 by 35 % | 1, then → 0 on commit | 1, then → 0 on commit |
| Top corner radius | 0 | not stated | 16 dp × p | 16 dp × p |
| Under offset X | −30 % × (1 − p) | 0 | 0 | −20 % → 0 on commit |
| Under scale | 1 | 1.1 → 1 | 1 | 0.9 → 1 on commit |
| Under opacity | 1 | 0 → 1 after 35 % | 1 | 1 |
| Under dim (scrim) | about 10 % × (1 − p), observed | not stated | 25 %, → 0 on commit | 25 % → 0 |
| Shadow on top | yes, observed | not stated | none | none |
| Commit | continue spring with velocity | not stated | progress + (1 − progress) × velocity factor | same |
| Cancel | spring back | "swiftly return and scale back" | animate progress to 0 | same |

The guide's fade-through ("At the 35% mark, neither screen is showing") conflicts with
Decompose, which keeps both visible during the gesture and fades only on commit. Fact:
the experiment's test `predictive_back_matches_android_guide` encodes the guide literally.
**Recommendation:** ship the Decompose Material behaviour as `Predictive` (it matches what
Android's own apps show), and keep the guide's literal fade-through out until it is
checked on a device (Work plan step 8).

### M4. What an application can change, and what it must not have to know

Can change: the preset (`Motion`), the timing (duration and curve, or a spring), the back
gesture style, a transition for one route (chosen when that entry enters, reused when it
leaves), motion off for one router, and motion off everywhere.

Must not need to know: frame clocks, the gesture's progress, which entry is leaving,
direction (push or pop), the reduced-motion setting, layer bookkeeping, and the
FFI. The application names a preset; the adapter does the rest.

### M5. Reduced motion

| Platform | Setting and how to read it | Source |
|---|---|---|
| iOS | `@Environment(\.accessibilityReduceMotion)`, `UIAccessibility.isReduceMotionEnabled` | Apple API names; the doc pages returned no body to the fetch, so the behaviour text was not re-read |
| macOS | `NSWorkspace.shared.accessibilityDisplayShouldReduceMotion`; System Settings › Accessibility › Motion › Reduce motion | MDN `prefers-reduced-motion` page lists the setting |
| Android | `Settings.Global.ANIMATOR_DURATION_SCALE`; "0.0f will cause animations to end immediately"; Compose scales by `MotionDurationScale` (`animation-core SuspendAnimation.kt:311`). Settings › Accessibility › Remove animations | Android reference; animation-core source |
| Web | `@media (prefers-reduced-motion: reduce)` via `matchMedia` | MDN |
| GPUI | `App::reduce_motion()` (`gpui-pre-0.3.12/src/app.rs:1405`); whether it is filled from the OS is not verified | gpui source |
| egui, iced | No API; the adapter must read the OS setting itself or take it from the application | — |
| Today in Compono | Nothing reads it (searched `crates`, `examples`, `native`) | — |

**Recommendation:** one value, `MotionPreference { Full, Reduced, Off }`, resolved by the
adapter and passed into `compono-motion`. `Reduced` keeps orientation but drops travel:
any spatial transition becomes a 150 ms `Fade`, and a Back gesture keeps following the
finger with opacity only (no scale, offset or corners). `Off` is `Transition::None`.
On Android, a duration scale of 0 means `Off`; Compose already scales tweens, so only the
gesture formulas need the flag.

## Architecture

### A1. One library per toolkit, or one shared design?

| | Option 1: per toolkit | Option 2: shared model, per frame | Option 3: shared specs only | **Option 4: shared model for Rust, golden specs for native** (recommended) |
|---|---|---|---|---|
| What is shared | nothing | presets, curves, springs, layer formulas, direction rule, transition state machine | presets and curves as data | Option 2 for Rust adapters; for native, the same values checked by golden tables |
| Code written | ~6 × 600 lines; Decompose alone is 3 878 lines of Kotlin in `extensions-compose*/commonMain` | ~900 lines shared + ~250–350 per Rust adapter + native unchanged in size | ~150 shared + ~500 per toolkit | ~900 shared + ~300 per Rust adapter + ~250 per native package |
| Consistency | drifts (today Swift and Compose already copy each other by hand) | exact in Rust UIs | each engine interprets the curve | exact in Rust UIs; native within golden tolerance |
| Native fit | best | poor: SwiftUI would set `@State` every frame on the main thread and lose Core Animation's interpolation and velocity hand-off; and the Swift and Compose packages would need bindings, which they deliberately do not know | good | good |
| Testing without a screen | per toolkit, rarely done | exact, in plain `cargo test` | specs only | exact in Rust; native asserts against the same table |
| Main cost | duplicated bugs (see the direction bug below) | native mismatch | Rust UIs still write all formulas three times | two copies of the gesture formulas (Swift, Kotlin), kept honest by tests |

Option 2 is right for the Rust side: egui, iced and GPUI have weak animation engines
(egui: a value toward a target; iced: `Animation<T>` over `lilt`; GPUI: `with_animation`
on a single element), so the frame function must be written anyway, and writing it once
is the saving. Option 3 is right for the native side: SwiftUI and Compose have strong
engines (`withAnimation`, springs, `SeekableTransitionState`, `PredictiveBackHandler`),
and the packages are binding-free by design (ARCHITECTURE.md, "Language boundary").
Option 4 combines them.

A defect that a shared rule removes (Fact): both native stacks decide direction by
comparing keys, `forward = key(active) >= key(shown)` (`RouterViews.swift:136`,
`RouterViews.kt:94`). `bring_to_front` makes an older entry active while the old one
stays parked, so it plays the *pop* animation. Decompose's rule
(`AbstractStackAnimation.kt:94`) treats it as a push. The shared rule:

> Backward if and only if the old active key is gone from the stack and the new active key
> was below it. Otherwise Forward.

### A2. What is shared and what is written per toolkit

```
                         compono (core, no_std)  — unchanged except key accessors (A5)
                                   │ entries(), previous(), active_key(), keys
                                   ▼
             compono-motion  (new, no_std, no deps)                         ~900 lines
   Easing · Spring · Timing · LayerStyle · Direction · StackTransition ·
   BackGestureStyle · Motion presets · MotionPreference · StackChange::between ·
   TransitionState (start / frame(now) / is_running)          + golden table example
          │                               │                              │
   per frame (Rust)                per frame (Rust)                golden values (text)
          ▼                               ▼                              ▼
 compono-egui  ~350             compono-iced ~400 / gpui ~350     Swift ~250 · Kotlin ~300
 clock: ctx.input time          clock: RedrawRequested /          own engine, own formulas
 apply: set_transform_layer,    request_animation_frame           (≈60 lines each), asserted
   painter scrim, alpha         apply: float translate/scale,       against the golden table
 leaving: replay last shapes      overlay scrim; GPUI offset,     leaving: view snapshot is
                                  .opacity()                        free (Compose/SwiftUI)
                                leaving: see A5 (open question)
```

| Part | Where | Estimate |
|---|---|---|
| Curves, springs, timing | `compono-motion/src/curve.rs`, `spring.rs` | 250 lines + 150 test |
| Layer formulas for stack, slot, pages, panels, back gesture | `transition.rs`, `back.rs` | 350 + 250 test |
| Direction rule, transition state machine, presets, preference | `change.rs`, `state.rs`, `preset.rs` | 300 + 150 test |
| Core: key accessors | `compono/src/router/{slot,pages,panels}.rs` | 40 + tests |
| egui adapter | `compono-egui/src/motion.rs` | 350 |
| iced adapter (a widget holding `TransitionState` in its tree state) | `compono-iced/src/motion.rs` | 400 |
| GPUI adapter | `compono-gpui/src/motion.rs` | 350 |
| SwiftUI | `RouterViews.swift` grows, new `Motion.swift` | +250 |
| Compose | `RouterViews.kt` grows, new `Motion.kt`, predictive styles | +300 |

### A3. Can Rust compute each frame for SwiftUI and Compose?

Cost is not the obstacle. Measured: one tween-plus-slide frame is 41 ns, a predictive-Back
frame 2.5 ns; with a 20–150 ns call (`benchmarks/ffi-boundary`), two layers at 120 Hz cost
under 50 µs per second, against an 8.3 ms frame. The obstacles are elsewhere:

1. The Swift package and the Compose library know nothing about generated bindings
   (ARCHITECTURE.md, "Language boundary"). Per-frame calls would force every application
   to export motion functions through its own bindings.
2. SwiftUI interpolates `withAnimation` springs outside `body`; driving it from Rust means
   writing `@State` each frame on the main thread, which gives up retargeting with
   velocity when the user interrupts, and system behaviours such as `navigationTransition`.
3. Compose could call Rust inside a `graphicsLayer {}` lambda without recomposition, but
   the gain is only the 10-line gesture formula.

**Recommendation:** native UIs use their own engines. What crosses the boundary is
per-navigation, not per-frame, and already crosses: entry keys. The preset itself is
chosen in Swift or Kotlin, so nothing new crosses.

### A4. Who owns the clock and the progress?

The UI owns both, as today. Facts: progress is clamped and stored in the native
`BackGestureState` (`ComponoTree.swift:31-35`, `ComponoTree.kt:41-48`); the tree stores
only `gesture: Option<NodeId>` (`tree/back.rs:39`). **Recommendation:** keep it.
`compono-motion` takes time and progress as arguments and holds no clock. Clocks:
egui `ctx.input(|i| i.time)` + `request_repaint`; iced `window::frames()` or the widget's
`RedrawRequested` event with `shell.request_redraw()`; GPUI `Instant::now()` +
`window.request_animation_frame()`; SwiftUI and Compose their frame clocks. Rust desktop
UIs have no Back gesture today (`Convention` maps keys and buttons to `back()`), so they
only play programmatic transitions.

### A5. Entries kept drawn during a transition

| Router | Needs drawn | Available now (Fact) | Missing |
|---|---|---|---|
| Stack, push | entering (active), leaving (now parked, alive) | `active()`, `previous()` returns the parked value | nothing |
| Stack, pop / replace | entering (active), leaving (**destroyed**) | `active()` only | the leaving view |
| Stack, gesture | active, previous | `previous()`, `gesture_target()` | nothing; restored-but-unbuilt previous draws the background (documented) |
| Slot, show / close | the entry; on close it is destroyed | `active()`; **no visit key** (`slot.rs:111-150`) | key; leaving view |
| Pages, select | old and new pages; both alive inside `retain` | only `active()` and `selected()` (`pages.rs:113-139`) | `Pages::child(index)` and a key per page |
| Panels, open / close | the panel; closing destroys it | `visible()`, `main/details/extra()`, `mode()`, `shown()`; **no keys** | keys per panel visit; leaving view |

The leaving view, per toolkit:

- SwiftUI and Compose: free. The view on its way out is the one already drawn
  (`RouterViews.swift:16-17`, `RouterViews.kt:59-61`).
- egui: **proved possible** without the component. The adapter draws each entry into its
  own layer, copies that layer's `ClippedShape`s inside the frame
  (`Context::graphics`), and on later frames repaints them into another layer with
  `set_transform_layer` (experiment `tests/egui_replay.rs`, passes). Cost: one `Vec` of
  shapes per animated stack, freed when the transition ends. Interaction on the ghost is
  off, which is correct.
- iced 0.14 and GPUI 0.3: no public way to keep or replay a subtree's last frame. An
  `Element` borrows the tree and cannot outlive it; GPUI's cached views re-render when
  their bounds move. Two choices, both open (Questions for the owner):
  (a) degrade: on a pop the leaving layer is replaced by the scrim colour sliding out,
  which reads as a pop but shows no content; or
  (b) change the core: the tree detaches a removed component at once (phase, tasks,
  Back, saved state all end inside the operation, as now), but keeps its *value* in a
  departing list readable through its handle until the UI calls `tree.release(id)` or the
  next operation after a time-out. This changes when `Drop` runs, which ARCHITECTURE.md
  promises happens inside the operation.

**Recommendation:** add the key accessors (small, additive); use (a) for iced and GPUI
until the owner decides on (b).

### A6. Staying inside Compono's rules

- No `std` in the core: the core gains only key accessors. `compono-motion` is
  `#![no_std]` too. Fact: `f32::exp` and `sin` do not exist in `core` on rustc 1.99.1
  (checked), so springs are integrated with fixed 1 ms steps instead of a closed form;
  no `libm`. Measured cost 2 µs for a 0.25 s evaluation from zero; the state machine
  integrates incrementally from the last frame, so a frame costs a few steps.
- No new dependency: none in the design. `egui` appears only as a dev-dependency of the
  experiment.
- No `unwrap`, `panic!`, indexing: the prototype has none; clamps replace asserts.
- No logic in one UI that others need: direction, presets, formulas and reduced-motion
  mapping live in `compono-motion`; adapters only clock and apply.
- `cfg(target_os)`: `compono-motion` has none. The adapter or the application chooses
  `Motion::material()` or `cupertino()`.
- Public items follow `VERSIONING.md` and `CHANGELOG.md`; `skills/` examples that draw a
  stack get the new optional call.

### A7. Testing without a screen

- Exact layer values at given `t` and `p`: plain unit tests (`tests/values.rs` in the
  experiment: slide at 0.25 and 0.5, ends at rest for every preset, predictive Back at 0.35
  and 1.0, curve reference points, spring settle times).
- State machine with virtual time: `TransitionState::frame(now)` takes time as a value,
  so `compono-test`'s virtual time drives it.
- Golden table: an example prints `transition, direction, t, offset_x, …` rows; the Swift
  and Kotlin test suites embed the file and assert their formulas to 1e-3.
- Adapter tests: egui headless `Context::run_ui` and inspect `FullOutput.shapes` (as the
  replay experiment does); iced `iced_test` simulator; GPUI `TestAppContext` with
  `run_until_parked` and simulated frames.

Experiment, path and results (outside the repository):
`/private/tmp/claude-501/-Users-yet--claude-spares-spare-ZIhnul/5839e5a5-1611-4b57-b24c-96016103371a/scratchpad/motion-proto`

| File | Proved |
|---|---|
| `src/lib.rs` | A `no_std`, dependency-free model of easing, springs, timing, stack transitions and Back styles fits in about 300 lines |
| `tests/values.rs` (5 tests, pass) | Exact values are testable; M3 standard spatial spring settles in 0.233 s, SwiftUI default in 0.817 s |
| `tests/egui_replay.rs` (pass) | egui can keep drawing a destroyed entry from its last frame's shapes, translated |
| `examples/cost.rs` (release) | 41 ns per tween frame, 2.5 ns per predictive-Back frame, 2 µs per spring evaluated from zero |

## API

### P1. Public API

```rust
// crate compono-motion, #![no_std]

/// A curve from linear time to eased progress.
pub enum Easing { Linear, CubicBezier(f32, f32, f32, f32) }
impl Easing {
    pub const STANDARD: Self;               // M3 (0.2, 0, 0, 1)
    pub const EMPHASIZED_DECELERATE: Self;  // (0.05, 0.7, 0.1, 1)
    pub const STANDARD_DECELERATE: Self;    // (0, 0, 0, 1)
    pub const FAST_OUT_SLOW_IN: Self;       // Compose tween() default
    pub fn ease(self, t: f32) -> f32;
}

/// A spring from 0 to 1 with mass 1.
pub struct Spring { pub damping_ratio: f32, pub stiffness: f32 }
impl Spring {
    pub const fn from_response(response_secs: f32, damping_ratio: f32) -> Self; // SwiftUI form
    pub const MATERIAL_SPATIAL: Self;   // 0.9 / 700
    pub const CUPERTINO: Self;          // response 0.55, damping 1.0
}

pub enum Timing { Tween { millis: u16, easing: Easing }, Spring(Spring) }

/// What a UI applies to one drawn entry. Offsets are shares of the router's size.
pub struct LayerStyle {
    pub offset_x: f32, pub offset_y: f32, pub scale: f32, pub opacity: f32,
    pub corner_radius: f32, // logical px
    pub dim: f32,           // black scrim over the layer, 0..=1
}

/// Decompose's names: the role of one entry in a transition.
pub enum Direction { EnterFront, ExitFront, EnterBack, ExitBack }

pub enum Transition {
    Slide { parallax: f32, dim: f32 },
    Fade,
    Scale { front: f32, back: f32 },
    SharedAxisX { shift_dp: f32 },
    SlideUp,                                  // sheets in a Slot
    None,
    Custom(fn(Direction, f32) -> LayerStyle),
}
impl Transition { pub fn layer(self, direction: Direction, factor: f32) -> LayerStyle; }

pub enum SwipeEdge { Left, Right, None }      // AndroidX BackEvent.SwipeEdge
pub struct BackEvent { pub progress: f32, pub edge: SwipeEdge, pub touch_y: f32 }
pub enum BackAnimation { Swipe { parallax: f32, dim: f32 }, Predictive { corner_radius: f32 }, None }
impl BackAnimation {
    /// Top and bottom layers during the gesture; `finish` runs 0 → 1 after a commit.
    pub fn frame(self, event: BackEvent, finish: f32, size: Size) -> StackFrame;
}

pub enum MotionPreference { Full, Reduced, Off }

/// The preset a router uses.
pub struct Motion { pub transition: Transition, pub timing: Timing, pub back: BackAnimation }
impl Motion {
    pub const fn material() -> Self; pub const fn cupertino() -> Self;
    pub const fn desktop() -> Self;  pub const fn web() -> Self;  pub const fn none() -> Self;
    pub const fn with_timing(self, timing: Timing) -> Self;
    pub const fn with_transition(self, transition: Transition) -> Self;
    pub fn for_preference(self, preference: MotionPreference) -> Self;
}

/// What changed in a stack, from its keys before and after.
pub enum StackChange { None, Forward { from: u64, to: u64 }, Backward { from: u64, to: u64 } }
impl StackChange { pub fn between(before: &[u64], before_active: u64, after: impl Iterator<Item = u64>, after_active: u64) -> Self; }

/// The running transition of one router. An adapter keeps one per router.
pub struct TransitionState { /* keys, start time, motion, last spring step */ }
impl TransitionState {
    pub fn start(&mut self, change: StackChange, motion: Motion, now_secs: f64);
    /// The layers to draw at `now`, bottom first, each with its entry key.
    pub fn frame(&mut self, now_secs: f64) -> Frame; // [(u64, LayerStyle); 2], at most
    pub fn is_running(&self) -> bool;
}
```

Core additions (additive, in `compono`):

```rust
impl<R, H, P> Slot<R, H, P>   { pub fn active_key(&self) -> Option<u64>; }
impl<R, H, P> Pages<R, H, P>  { pub fn child(&self, index: usize) -> Option<&H>; }
impl<T, P> Panels<T, P>       { pub fn keys(&self) -> PanelKeys; } // visit key of each open panel
```

The call an adapter makes each frame (egui):

```rust
let state = ui.data_mut(|d| d.get_temp_mut_or_default::<TransitionState>(id));
let change = StackChange::between(&state.keys, state.active, stack.entries().map(|e| e.0), stack.active_key());
if !matches!(change, StackChange::None) { state.start(change, motion, ui.input(|i| i.time)); }
for (key, style) in state.frame(ui.input(|i| i.time)) {
    apply(ui, key, style);  // set_transform_layer, alpha, scrim, or replay the ghost
}
if state.is_running() { ui.ctx().request_repaint(); }
```

Swift (native package, no bindings):

```swift
public struct StackAnimation: Sendable {
    public var transition: StackTransition      // .slide(parallax:dim:), .fade, .scale(front:back:), .sharedAxis, .none, .custom
    public var animation: Animation             // default .spring(response: 0.55, dampingFraction: 1)
    public var backGesture: BackGestureStyle    // .swipe, .none
    public static let platform, slide, fade, none: StackAnimation
}
ChildStack(tree:owner:active:previous:animation:transition:content:)
//  transition: (Child) -> StackTransition?   chosen when an entry enters
```

Kotlin:

```kotlin
class StackAnimation(
    val transition: StackTransition = StackTransition.SharedAxisX(),
    val spec: FiniteAnimationSpec<Float> = spring(0.9f, 700f),
    val back: BackAnimation = BackAnimation.Predictive(),  // Predictive, Swipe, None
)
@Composable fun <C : Any> ChildStack(active, key, modifier, animation = StackAnimation(),
    transition: (C) -> StackTransition? = { null }, tree, owner, previous, background, content)
```

### P2. Application code

Default motion (no change from today's call):

```rust
root.stack.view(ui, |ui, child| match *child { … });          // egui: Motion::desktop()
```

A different preset for one stack:

```rust
use compono_motion::{Motion, Timing, Easing};
StackView::new(&root.stack)
    .motion(Motion::material().with_timing(Timing::Tween { millis: 200, easing: Easing::STANDARD }))
    .show(ui, |ui, child| …);
```

A custom transition for one route (chosen when the entry enters, reused on its pop):

```rust
StackView::new(&root.stack)
    .transition_for(|child| matches!(child, Child::Compose(_)).then_some(Transition::SlideUp))
    .show(ui, |ui, child| …);
```

```swift
ChildStack(tree: tree, owner: id, active: m.active, previous: m.previous,
           animation: .platform, transition: { $0.kind == .compose ? .slideUp : nil }) { child in … }
```

### P3. Adapter author (a future toolkit such as Exact)

1. Keep one `TransitionState` per router in the toolkit's state store.
2. Each frame: `StackChange::between` on the stack's keys; `start` on a change; `frame(now)`.
3. Draw each `(key, LayerStyle)` bottom first: translate by `offset × size`, scale about
   the centre, set opacity, clip to `corner_radius`, then paint a black quad at `dim`.
   Where a toolkit lacks subtree opacity (iced 0.14), paint a background-coloured quad at
   `1 − opacity` over the layer instead.
4. Keep the leaving entry drawable (A5) or accept the degraded pop.
5. Request frames while `is_running()`; read the reduced-motion setting into
   `MotionPreference`.
6. Run the golden-table test against the adapter's applied values.

### P4. Names

| Compono | Decompose | Kept? |
|---|---|---|
| `Direction::{EnterFront, ExitFront, EnterBack, ExitBack}` | `Direction.ENTER_FRONT…` | Kept |
| `StackAnimation` (Swift, Kotlin) | `StackAnimation` | Kept (already in Compono) |
| `Transition` with `slide`, `fade`, `scale` | `StackAnimator` from `slide()`, `fade()`, `scale()` | Renamed: one enum is clearer than factory functions in Rust; variant names kept |
| `BackAnimation::Predictive` | `materialPredictiveBackAnimatable` | Renamed: "animatable" is a Compose term |
| `BackEvent`, `SwipeEdge` | AndroidX `BackEvent`, `SwipeEdge` | Kept |
| `Motion` | no equivalent (`stackAnimation(animator, predictiveBackParams)`) | New: the whole preset |
| `LayerStyle` | `Modifier` per child | New: data instead of a modifier |
| `TransitionState`, `StackChange` | internal `AbstractStackAnimation` items | New, public so adapters share them |
| `MotionPreference` | none | New |

## Work plan

1. **Fix the stack direction in Swift and Compose.** Files: `RouterViews.swift`,
   `RouterViews.kt`. Test: `bring_to_front` plays a push (XCTest, Compose UI test).
   Owner sees: switching tabs no longer slides backwards.
2. **Create `compono-motion`** with `Easing`, `Spring`, `Timing`, `LayerStyle`,
   `Direction`, `Transition`, `Motion` presets, `StackChange::between`, golden example.
   Tests: exact values, spring settle times, direction cases, thumbv7em check.
   Owner sees: `cargo test -p compono-motion` and the printed golden table.
3. **egui stack transitions** with `TransitionState` and shape replay for pops.
   Files: `compono-egui/src/motion.rs`, `navigation.rs`; `showcase-egui`. Tests: headless
   frames at fixed times. Owner sees: push and pop animate in `showcase-egui` and in the
   browser (`showcase-web`).
4. **Core key accessors**: `Slot::active_key`, `Pages::child`, `Panels::keys`, with
   `CHANGELOG.md` lines. Tests in `crates/compono/tests`. Owner sees: no visible change.
5. **egui Slot, Pages, Panels** transitions. Owner sees: dialogs fade, pages slide by
   order, details panel slides in on a narrow window.
6. **iced and GPUI** stack, slot, pages, panels, with the degraded pop (A5 option a) unless
   the owner chose option b. Tests: `iced_test`, GPUI `TestAppContext`.
   Owner sees: the same screens move in `showcase-iced` and `showcase-gpui`.
7. **Reduced motion**: `MotionPreference` read in `compono-web` (`matchMedia`), GPUI
   (`reduce_motion`), egui/iced from the application; native from the environment and
   `ANIMATOR_DURATION_SCALE`. Owner sees: turning the OS setting on gives fades.
8. **Native presets and predictive styles**: `StackAnimation` gains `transition`,
   `animation`, `back`; Compose gets `BackAnimation.Predictive` with `BackEvent`'s edge and
   touch Y (`PredictiveBackHandler` already delivers them); Swift switches to a spring
   with release velocity. Golden-table tests on both. Measure the iOS push on a simulator
   frame by frame and record the result. Owner sees: Android shows the shrinking card;
   iOS release feels like UIKit.

## Risks

| Risk | Probability | Effect | What reduces it |
|---|---|---|---|
| iced and GPUI cannot draw a destroyed leaving entry | Certain today | Pops show no outgoing content | Degraded pop; owner decision on a departing list (A5) |
| Swift/Kotlin formulas drift from `compono-motion` | Medium | Platforms look different | Golden table checked in both test suites |
| Platform values unsourced (iOS push, scrim) | High | Motion feels slightly off | Measure on simulator and device (step 8); mark values as observed |
| Android guide vs Decompose on fade-through | Medium | Predictive Back looks wrong | Verify on a device before choosing |
| egui shape replay misses textures or callbacks (`PaintCallback`) | Low | Ghost of a 3D view is blank | Accept; it lasts 300 ms |
| Integrated springs differ from closed form | Low | Under 0.1 % error at 1 ms step | Test against closed form under `std` in tests |
| A gesture or transition during another | Medium | Jumps | `TransitionState::start` retargets from the current values; native code already finishes a settling gesture (`ComponoTree.swift:155-158`) |
| Scope creep toward Decompose's 3 878 lines | Medium | Maintenance | Only routers that exist; Items and ChildSet excluded |

## Questions for the owner

1. Leaving entries in iced and GPUI: accept a degraded pop, or let the tree keep a
   departing value until the UI releases it (changes when `Drop` runs)?
2. Should each platform look native by default (`material` on Android, `cupertino` on
   iOS, fades on desktop), or should Compono have one look everywhere?
3. Which Android predictive styles to ship: only Material, or also Decompose's Android V1
   and V2?
4. Is `compono-motion` a separate crate (recommended), or a module of `compono` behind a
   feature?
5. Should the native packages ever call Rust motion functions (needs exports in every
   application's bindings)? This design says no.

## Sources

- Compono `7fccec1`: `ARCHITECTURE.md` (Navigation, Back, Platform contract, UI adapters,
  Language boundary); `AGENTS.md`; `crates/compono/src/router/{stack,slot,pages,panels,items,set}.rs`;
  `crates/compono/src/tree/back.rs`; `crates/compono-{egui,iced,gpui}/src/navigation.rs`;
  `native/apple/Compono/Sources/Compono/{RouterViews,ComponoTree,Lifecycle}.swift`;
  `native/android/compono/src/main/kotlin/compono/android/{RouterViews,ComponoTree,Lifecycle}.kt`.
- Decompose `2cc1a4ccee2c68fc5d717762edc610b671e79cea` (2026-11-10):
  `extensions-compose/src/commonMain/.../stack/animation/{Slide,Fade,Scale,Direction,AbstractStackAnimation,StackAnimation}.kt`,
  `.../predictiveback/{MaterialPredictiveBackAnimatable,AndroidPredictiveBackAnimatableV1,AndroidPredictiveBackAnimatableV2,AndroidPredictiveBackAnimatable,PredictiveBackAnimatable,LayoutCorners}.kt`,
  `.../pages/{ChildPages,PagesScrollAnimation}.kt`,
  `extensions-compose-experimental/src/commonMain/.../panels/{ChildPanels,ChildPanelsAnimators}.kt`.
- Material 3 tokens: `org.jetbrains.compose.material3:material3:1.10.0-alpha05` sources,
  `tokens/MotionTokens.kt`, `StandardMotionTokens.kt`, `ExpressiveMotionTokens.kt`
  (m3.material.io pages could not be read by fetch; the tokens are generated from them).
- Compose `androidx.compose.animation:animation-core-android:1.12.0` sources:
  `AnimationSpec.kt:39`, `Easing.kt:50`, `SuspendAnimation.kt:311`.
- Android predictive Back design guide:
  developer.android.com/design/ui/mobile/guides/patterns/predictive-back; developer guide:
  developer.android.com/guide/navigation/custom-back/predictive-back-gesture (activity
  1.6.0+, activity-compose 1.8.0+, Android 15 removes the developer option).
- `Settings.Global.ANIMATOR_DURATION_SCALE`: developer.android.com/reference/android/provider/Settings.Global.
- Apple: developer.apple.com/documentation/swiftui/animation/default (response 0.55,
  dampingFraction 1.0; easeInOut before iOS 17); HIG Motion page (no navigation values).
  `smooth`, `navigationTransition`, `accessibilityReduceMotion` pages gave no values to the fetch.
- MDN `prefers-reduced-motion`: developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion.
- Toolkits: `egui 0.37.0` (`context.rs:3564, 3707-3790`, `style.rs:1550`, `layers.rs`);
  `iced_core 0.14.0` (`animation.rs`), `iced_widget 0.14.2` (`float.rs`);
  `gpui-pre 0.3.12` (`elements/animation.rs`, `app.rs:1405`, `window.rs:2939`, `styled.rs:746`).
- rustc 1.99.1: `f32::exp` not in `core` (checked).

## Owner's decisions

Recorded on 2026-11-13, in answer to "Questions for the owner":

1. **Leaving entries in iced and GPUI:** a simpler pop. Only the entry underneath
   animates in, and the leaving entry disappears at once. The tree does not keep
   removed components. egui replays the leaving entry's last frame.
2. **Look:** native on each platform: Material on Android, iOS style on Apple
   systems, fades on desktop and the web.
3. **Android predictive Back:** the Material style only.
4. **Location:** a separate `compono-motion` crate, `no_std`, with no
   dependencies.
5. **Native packages:** they do not call Rust motion functions. They keep their
   own animation engines and are checked against golden value tables that
   `compono-motion` generates.

# Prompt: build the motion system of Compono

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
engineering agent as its task.

---

## 1. Your role

You are a software engineer. You build the router animations of Compono from an
approved design. You write code, tests and documentation. A step is complete
only when its code is correct and all its checks pass.

The design is complete and the owner approved it. Do not design again. If the
design is wrong or not complete at one point, select the simplest correct
solution, and record the change and the reason in your report.

## 2. Before you start

1. Read these files fully:
   - `docs/design/motion.md`: the design. The section "Owner's decisions" at the
     end has priority over every other part of the document.
   - `docs/design/motion-proto/`: a working prototype of the shared model and of
     the egui replay. Use it as a start. Do not copy it without review: it is
     prototype code, and the rules in `AGENTS.md` apply to the library.
   - `AGENTS.md` and `ARCHITECTURE.md`.
2. Use these skills from `.agents/skills`, as `AGENTS.md` tells you:
   - `test-driven-development` for each step.
   - `rust-best-practices` for all Rust code.
   - `compono-native` for steps 1 and 8.
   - `compono-component` for step 4.
   - `compono-verify` before you report.
   - `writing-for-agents` if you change a skill, `AGENTS.md` or `CLAUDE.md`.
3. The repository is a git repository. Do not commit. Do not push. Before you
   start, record the output of `git status --short`, so that your report can
   show what you changed.
4. Start all tests one time before you change code. Record the number of tests
   that pass. All must pass.

## 3. The owner's decisions

These decisions are final:

1. **Leaving entries in iced and GPUI:** a simpler pop. Only the entry under the
   leaving entry animates in. The leaving entry disappears at once. Do not
   change the tree to keep removed components. egui replays the last frame of
   the leaving entry.
2. **Look:** native on each platform: Material on Android, iOS style on Apple
   systems, fades on desktop and on the web.
3. **Android predictive Back:** the Material style only. Do not add Decompose's
   Android V1 and V2 styles.
4. **Location:** a new crate, `crates/compono-motion`. It is `#![no_std]` and it
   has no dependencies.
5. **Native packages:** the Swift package and the Compose library do not call
   Rust motion functions. They use their own animation engines. Their tests
   compare their values with golden tables that `compono-motion` makes.

## 4. How to do each step

Do these steps for each step of section 5:

1. Write a test for the new behaviour.
2. Make sure that the test fails for the correct reason. A compile error because
   of a missing item is not sufficient: add a stub first.
3. Write the code.
4. Make sure that the new test and all other tests pass.
5. Do the checks of section 7 that apply to the step.
6. Update the documentation for the step: doc comments, `ARCHITECTURE.md`,
   `CHANGELOG.md` (section "Unreleased"), `README.md` if the crate list changes,
   and the skills in `skills/` (`compono-ui` and `compono-native-apps`) and in
   `.agents/skills`. After a skill change, use `scripts/sync-skills.sh`.
7. Record in your report what the owner can do to see the step: the command to
   start, and what to look at.

Do the steps in the sequence of section 5. Complete each step before you start
the next one. If you cannot complete all steps, stop after a complete step, and
tell in your report which steps are done.

## 5. Steps

### Step 1: fix the direction of stack animations in Swift and Compose

- **Location:** `native/apple/Compono/Sources/Compono/RouterViews.swift:136`
  (`forward = active.key >= shown.key`) and
  `native/android/compono/src/main/kotlin/compono/android/RouterViews.kt:94`.
- **Defect:** Both compare entry keys to select push or pop. After
  `bring_to_front`, an older entry with a lower key comes to the top, so the UI
  plays the Back animation.
- **Required result:** the direction is "back" only when the entry that was
  shown is no longer in the stack. Otherwise it is "forward". Put the rule in a
  small pure function in each package, so that a unit test can examine it. Do
  not add a Compose UI test dependency.
- **Tests:** push, pop, `bring_to_front` and `replace`, in `swift test` and in
  the Compose library's unit tests.

### Step 2: create `compono-motion`

- **Location:** a new crate `crates/compono-motion`.
- **Contents:** `Easing`, `Spring`, `Timing`, `LayerStyle`, `Direction`,
  `Transition`, `BackEvent`, `BackAnimation`, `MotionPreference`, `Motion` with
  its presets, `StackChange::between` and `TransitionState`, as section "P1" of
  the design describes.
- **Rules:**
  - `#![no_std]`, no dependencies, no allocation in `frame` or `layer`.
  - The library rules of `AGENTS.md` apply: no `unwrap`, `expect`, `panic!`,
    indexing or `unsafe`.
  - Each preset value has a comment with its source, as the design gives it.
    A value that the design marks as observed keeps that mark.
- **Golden tables:** an example or a test that writes the values of each preset
  at fixed times and progress values, in a form that Swift and Kotlin tests can
  read. Steps 1 and 8 use these tables.
- **Tests:** exact layer values at fixed times, spring settle times, every case
  of `StackChange::between`, and `cargo check -p compono-motion --target
  thumbv7em-none-eabihf`.
- **Documentation:** a README for the crate, and the crate in the layout tables
  of `README.md`, `ARCHITECTURE.md` and `AGENTS.md`.

### Step 3: stack transitions in egui

- **Location:** `crates/compono-egui/src/motion.rs` (new),
  `crates/compono-egui/src/navigation.rs`, `examples/showcase-egui`.
- **Required result:** push and pop in a `Stack` animate with
  `Motion::desktop()` by default. On a pop, egui draws the last frame of the
  leaving entry, as the prototype does. An application can select another
  preset for one stack.
- **Tests:** draw frames headless at fixed times, and examine the layers.
- **The web:** `showcase-web` uses the same code. Use `Motion::web()` there.

### Step 4: key accessors in the core

- **Location:** `crates/compono/src/router/`.
- **Required result:** `Slot::active_key`, `Pages::child`, and `Panels::keys`,
  as the design gives them. They are additive.
- **Tests:** in `crates/compono/tests`, for each accessor, after each change
  of its router.
- **Rules:** these are public items. Add lines to `CHANGELOG.md`, and update the
  examples in `skills/` that need them.

### Step 5: `Slot`, `Pages` and `Panels` in egui

- **Required result:** a dialog in a `Slot` fades in and out. `Pages` slide in
  the direction of the page order. On a narrow window, the details panel of
  `Panels` slides in.
- **Tests:** headless frames, as in step 3.

### Step 6: iced and GPUI

- **Location:** `crates/compono-iced`, `crates/compono-gpui`, and their
  showcases.
- **Required result:** the same transitions as egui for `Stack`, `Slot`,
  `Pages` and `Panels`, with the simpler pop of decision 1.
- **Tests:** `iced_test` where it is available, GPUI `TestAppContext`, and the
  existing UI tests of both showcases.

### Step 7: reduced motion

- **Required result:** `MotionPreference` changes each preset as the design
  says.
  - The web reads it from `matchMedia("(prefers-reduced-motion: reduce)")` in
    `compono-web`.
  - GPUI reads it from its own setting, if the version in use has one.
  - egui and iced take it from the application.
  - SwiftUI reads `accessibilityReduceMotion`. Compose reads the system
    animator duration scale.
- **Tests:** for each preset, the value with `Reduced` and with `Off`.

### Step 8: native presets and Material predictive Back

- **Location:** the two `RouterViews` files and their tests.
- **Required result:**
  - The `StackAnimation` type of each package gets `transition`, `animation`
    (or `spec`) and `back`, as section "P1" of the design gives them.
  - Compose gets `BackAnimation.Predictive` in the Material style, with the edge
    and the touch position from `BackEvent`.
  - Swift uses a spring with the release velocity of the swipe.
- **Tests:** the Swift and Kotlin values agree with the golden tables of step 2.
- **Measurement:** measure a push on the iOS simulator frame by frame. Use the
  simulator with id `F5A2CD3E-6FF7-460A-889D-9126C59F45E9`. Record the
  duration and the offset of the covered screen. Compare them with the values
  that the design marks as observed, and correct the presets if they differ.
- **Android:** use the emulator `emulator-5554` (`ANDROID_SERIAL`). Do not
  install anything on a physical device.

## 6. Rules

- Do not add a dependency to any crate or package, except a dependency on
  `compono-motion` in the adapters. If a step needs a new dependency, stop that
  step, and record why.
- Do not change the tree to keep removed components (decision 1).
- Do not change files that a step does not need.
- Keep the documentation in English. It describes only the current state.
- Put a time limit on each long command, for example
  `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`. Start builds that
  take minutes in the background. If a command stops at the limit, record it.
- If a step needs a decision that this prompt and the design do not give,
  select the option that keeps the public API smaller. Record the decision and
  the reason in your report.

## 7. Checks

After each step, use the checks that apply to it. Before the report, use all of
them:

- All commands in the list "Before finishing a change" in `AGENTS.md`.
- `cargo check -p compono-motion --target thumbv7em-none-eabihf`.
- `cargo test -p compono --release --test allocations`.
- After step 4: the three examples `navigation_cost`, `set_scaling` and
  `router_scaling`. Make sure that the numbers do not increase as the sizes
  increase.
- After steps 1, 7 and 8: the commands in the `compono-native` skill:
  `swift test` in `native/apple/Compono`, the Gradle unit tests and
  `:app:assembleDebug`, and an `xcodebuild` build of `native/showcase/ios`.
- After steps 3, 5 and 6: the UI tests of `showcase-egui`, `showcase-iced` and
  `showcase-gpui`.
- `scripts/sync-skills.sh --check`.

A step is complete only when its checks pass.

## 8. Report

Write the report in Markdown with these sections, in this sequence:

1. **Summary.** Three to six sentences. Tell which steps are complete.
2. **One section for each step.** In each section, give:
   - the change, with file paths;
   - the tests that you added, and the failure that each gave before the change;
   - existing tests that you changed, and the reason;
   - the decisions that you made, and the reason for each, especially each
     change from the design;
   - how the owner can see the step: the command and what to look at.
3. **Measurements.** The iOS push measurement of step 8, and the scaling numbers
   after step 4.
4. **Checks.** Each command and its result. Give the number of tests that passed
   and failed.
5. **Changed files.** The output of `git status --short` at the end.
6. **Not done.** Each step or item that you did not complete, and the reason.

Do not write that a step is complete if a check did not pass.

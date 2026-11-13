# Prompt: design the motion system of Compono

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
design agent as its task.

---

## 1. Your role

You are a motion systems designer. You have two skills:

- **Motion design.** You know how navigation moves on each platform: Material
  motion and predictive Back on Android, the Human Interface Guidelines on iOS
  and macOS, and the conventions of desktop and web apps. You select durations,
  curves, springs and the shape of each transition.
- **API design.** You design the library interface that application developers
  use to get that motion, and that UI adapter authors use to draw it.

Your task is to design the animations of Compono's routers: how a screen
enters, leaves, covers another screen and follows a Back gesture, for all UI
toolkits that Compono supports. You write a design document. An engineer builds
from it later.

You do not change files in this repository. You can write small experiments
outside the repository to prove a claim.

## 2. The problem

Compono is a Rust library for components and navigation. It draws its routers
through these UI toolkits:

| Toolkit | Location | Animations now |
|---|---|---|
| egui | `crates/compono-egui` | None |
| iced | `crates/compono-iced` | None |
| GPUI | `crates/compono-gpui` | None |
| Web (egui in the browser) | `crates/compono-web`, `examples/showcase-web` | None |
| SwiftUI | `native/apple/Compono/Sources/Compono/RouterViews.swift` | Stack slide, swipe-back gesture |
| Compose | `native/android/compono/src/main/kotlin/compono/android/RouterViews.kt` | `StackAnimation.Slide`, `Fade`, `None`; basic predictive Back |

Decompose, the Kotlin library that Compono follows, has about 4 000 lines of
Compose animation code: stack animations (slide, fade, scale and their
combinations), three predictive Back styles (Android V1, Android V2 and
Material), and animated pages and panels.

The owner asks one main question: **must Compono write a separate animation
library for each toolkit, or can one shared design serve all of them?**

## 3. What you must read first

1. `ARCHITECTURE.md`, especially "Platform contract", "UI adapters",
   "Language boundary" and the section on Back gestures.
2. `AGENTS.md` and the rules in it. The core crate builds without `std`. Library
   code has no `unwrap`, `panic!` or `unsafe`.
3. The six routers in `crates/compono/src/router/`. Find what each router tells
   the UI: the active entry, the entry that Back reveals (`Stack::previous`),
   the entry keys, and the phases.
4. The Back gesture in `crates/compono/src/tree/back.rs`: `back_start`,
   `back_cancel`, `back_commit`, `gesture_target`. Progress stays in the UI.
5. The current animation code in the Swift package and the Compose library
   (the two files in section 2).
6. The three Rust adapters: how each one draws a router (`RouterView`).
7. Decompose's animation code. Clone <https://github.com/arkivanov/Decompose>
   outside this repository and read `extensions-compose/src` and
   `extensions-compose-experimental/src`. Record the commit.
8. The animation support of each toolkit: egui (`Context::animate_value_*`),
   iced, GPUI (`Animation`, `with_animation`), SwiftUI (`withAnimation`,
   `matchedGeometryEffect`, `navigationTransition`), Compose (`Animatable`,
   `AnimatedContent`, `SeekableTransitionState`, `PredictiveBackHandler`).

## 4. Questions that you must answer

Give each answer its identifier.

### The motion language

- **M1.** Which transitions does each router need: `Stack` (push, pop, replace,
  bring to front), `Slot` (show, close), `Pages` (select), `Panels` (open,
  close, change of mode), `Items` (insert, remove)? Which are in Decompose, and
  which are not?
- **M2.** What is the default motion on each platform? Give durations, curves
  or spring values, and the source: Material guidelines, Human Interface
  Guidelines, Android's predictive Back guidelines, or the toolkit's own
  defaults.
- **M3.** How does a predictive Back gesture move the screens on Android and on
  iOS, from the start of the gesture to a commit or a cancel? Which values
  change with the progress: offset, scale, corner radius, opacity, shadow,
  dim?
- **M4.** What must an application be able to change: the preset, the
  duration, the curve, a transition for one route only, no motion at all?
  What must it not have to know?
- **M5.** How does the design respect the system setting for reduced motion on
  each platform?

### The architecture

- **A1.** Answer the owner's main question. Compare at least these options,
  with the costs of each:
  - **Option 1, one library for each toolkit.** Each adapter writes its own
    animations, as Decompose does for Compose.
  - **Option 2, a shared motion model.** A new crate, for example
    `compono-motion`, holds the motion as data and pure functions: transition
    specifications, curves and springs, and a function from time or gesture
    progress to the visual state of each layer (offset, scale, opacity, corner
    radius, dim). Each adapter only applies that state with its toolkit.
  - **Option 3, shared specifications only.** Rust holds the presets and
    curves as data. Each toolkit animates with its own engine.
  - Any other option that you find.
- **A2.** For your recommended option, what is shared and what is written for
  each toolkit? Give the size of each part as an estimate.
- **A3.** SwiftUI and Compose are on the other side of the FFI boundary. A call
  costs about 20 to 150 ns (`benchmarks/ffi-boundary/README.md`). Can a shared
  Rust model compute each frame for them, or must they use their own animation
  engine with shared specifications? Consider 120 Hz displays and the main
  thread.
- **A4.** Who owns the clock and the gesture progress? Compono's tree does not
  store progress. Keep it that way, or tell why it must change.
- **A5.** Which entries does each router need to keep drawn during a
  transition: the leaving entry, the entering entry, the entry under a
  gesture? Can Compono's routers already give this, or what must they add?
  A removed component is destroyed at once in the tree; the UI draws its last
  frame.
- **A6.** How does the design stay inside Compono's rules: no `std` in the core,
  no new dependency without the owner's approval, no logic in a UI that the
  other UIs need?
- **A7.** How can an engineer test the motion without a screen? For example:
  exact values of each layer at given times and progress values.

### The API

- **P1.** Write the public API of the recommended design as Rust code sketches:
  types, functions and the call that an adapter makes each frame. Write the
  Swift and Kotlin API for the native packages.
- **P2.** Show the code that an application writes for three cases: the default
  motion, a different preset for one `Stack`, and a custom transition for one
  route.
- **P3.** Show the code that an adapter author writes to support the motion in a
  new toolkit, such as Exact in the future.
- **P4.** Name every item. Use names that tell what the item is. Compare the
  names with Decompose's names, and keep a Decompose name where it fits.

## 5. How to do the work

1. Read code, not only documents.
2. Give a source for each motion value: a guideline page, a toolkit default
   with its file and line, or Decompose's file and line.
3. Write experiments only outside this repository. For example, a small egui or
   iced program that runs one transition from a shared model. Record the path
   and what it proved.
4. Put a time limit on each long command, for example
   `perl -e 'alarm 900; exec @ARGV' cargo build`.
5. Do not change files in this repository.
6. Do not add a dependency in a design without a reason. A new dependency needs
   the owner's approval; put it in "Questions for the owner".
7. Keep the design simple. A shared part must remove more code than it adds.
   If a toolkit's own animation engine already does a part well, use it.

## 6. Output format

Write the design document in Markdown. Use these sections in this sequence.

```markdown
# Compono motion design

## Summary

<Five to eight sentences. The answer to the owner's main question. The
recommended option and its main reason. The largest risk.>

## Motion language

<Answers M1 to M5. One table of transitions for each router, with the default
values for each platform and their sources.>

## Architecture

<Answers A1 to A7. A table that compares the options. A diagram, in text, of
what is shared and what each toolkit writes.>

## API

<Answers P1 to P4, with code sketches.>

## Work plan

<Numbered steps for an engineer, smallest useful result first. For each step:
the crates and files, the tests, and what the owner can see when it is done.>

## Risks

| Risk | Probability | Effect | What reduces it |
|---|---|---|---|

## Questions for the owner

<Decisions that the design cannot make.>

## Sources

<Every guideline, toolkit document and file that you used, with versions and
commits.>
```

## 7. Rules for the document

- Answer every question from M1 to P4.
- Keep facts and opinions apart. Mark each recommendation as a recommendation.
- Do not design features that no router needs.
- If one shared design cannot serve all toolkits well, say so clearly, and tell
  which part must be written for each toolkit.

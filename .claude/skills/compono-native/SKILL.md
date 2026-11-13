---
name: compono-native
description: Builds, tests and runs Compono's native side, from compono-ffi through BoltFFI to the Swift package, the Compose library and the native showcase. Use when changing any of them, regenerating bindings, or running the showcase on an iOS simulator or Android emulator.
---

# Native UIs

## Layout

- `crates/compono-ffi`: `NativeTree`, `Platform` and the phase codes. It depends
  on no binding generator.
- `native/showcase/ffi`: its own workspace, exporting the showcase with BoltFFI.
  `boltffi.toml` names the Swift module `ShowcaseFFI` and the Kotlin package
  `compono.showcase.ffi`.
- `native/apple/Compono`: the Swift package. `native/android/compono`: the
  Compose library. Neither knows about the generated bindings.
- `native/showcase/ios` and `native/showcase/android`: the apps.

## Building

BoltFFI is pinned at 0.32.1. Install it with
`cargo install boltffi_cli --version 0.32.1 --locked` when `boltffi` is missing.

```bash
(cd native/showcase/ffi && boltffi pack apple --release --deny-skipped)
```

```bash
(cd native/showcase/ffi && ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/28.2.13676358 boltffi pack android --release --deny-skipped)
```

Use NDK 28.2.13676358. `cargo ndk` fails with NDK 30.

## Checks

```bash
(cd native/showcase/ffi && cargo clippy --all-targets -- -D warnings)
```

```bash
(cd native/apple/Compono && swift test)
```

```bash
(cd native/showcase/android && gradle :compono:testDebugUnitTest :app:assembleDebug)
```

The project has no Gradle wrapper. When `gradle` is not on `PATH`, use the
distribution Android Studio downloaded, with Java 21 and the SDK:

```bash
export ANDROID_HOME=~/Library/Android/sdk JAVA_HOME=$(/usr/libexec/java_home -v 21)
GRADLE=$(echo ~/.gradle/wrapper/dists/gradle-9.8.1-bin/*/gradle-9.8.1/bin/gradle)
```

`compono-ffi`'s own tests run with `cargo test -p compono-ffi` in the main workspace.

`MotionTests` in both packages compare the stack formulas with
`crates/compono-motion/golden/motion.csv`. After changing a preset in
`compono-motion`, rewrite the table with
`cargo run -p compono-motion --example golden > crates/compono-motion/golden/motion.csv`,
then change `Motion.swift` and `Motion.kt` until both suites pass.

## Rules that come from bugs already hit

- Export the root's standard methods with exactly these Rust names: `new(listener,
  saved)`, `pump`, `set_phase(code: i32)`, `back`, `back_start`, `back_cancel`,
  `back_commit`, `contains(node: i64)` and `save`, and a listener trait with
  `commit(dirty: Vec<i64>, can_back: bool)` and `schedule_pump`. The Swift package and
  the Compose library are named to match, so `extension App: @retroactive TreeBridge {}`
  stays empty. BoltFFI reads exports from source, so a macro cannot write them.
- On Android, create the tree with `retainedTree` in `onCreate`. Keeping it in an
  `Application` survives the Activity finishing, so a relaunch shows a stale screen.
- Addresses cross the boundary as `i64` and counts as `i32`. A `ULongArray`
  parameter in a callback makes BoltFFI's Kotlin side fail in `JNI_OnLoad`.
- Every `NativeTree` call happens on the thread that created it. Other threads get
  `NativeError::WrongThread`, and a call from inside a running call gets `Busy`.
  The showcase passes every result through `checked` in `component.rs`, which
  ignores `Gone` and stops a debug build on `NativeError::is_misuse`. Route every
  new call through it too.
- A Tokio sleep must be created inside the future (`async move { sleep(d).await }`),
  because the FFI thread has no runtime context.
- AGP 9 reads Kotlin sources from `kotlin.directories.add(...)`, not `java.srcDir`.
- A Compose list must read the model outside the `LazyColumn` block, or it shows
  stale rows.
- Expose each component as an exported class holding `Ctx` (a weak tree) and its
  node, with `node()`, `model()` and its actions. A parent returns child objects
  as `Option`, as `root.tabs(child)` does: it compares the child's kind with
  `==` and gives `None` for a child of another kind in every build. Kotlin objects
  have no cleaner, so Compose creates them with `rememberComponent`, or the showcase's
  `rememberChild` for an `Option`, which close them on dispose. Name an action
  `dismiss`, never `close`, which clashes with `AutoCloseable`.
- A screen subscribes with `observe(component.node())` in Compose and
  `let _ = tree?.observe(component.node())` in SwiftUI. The Compose `observe` is
  `@NonRestartableComposable`; without it the screen does not redraw.
- In SwiftUI, a router view must not clip its content. `.clipped()` hid the system
  tab bar and toolbar on iOS 26.

## Running on a simulator or emulator

- Use the simulators and emulators already installed through Xcode and Android
  Studio. Start the Android "Resizable" emulator rather than creating a new one.
- With more than one Android device attached, set `ANDROID_SERIAL=emulator-5554`.
- Install nothing on a physical phone without the user's consent.
- Read logcat by writing it to a file in the background and searching the file
  with a bounded loop. A `while read` on a live logcat never ends.
- `xcrun simctl io <device> screenshot` can fail with "Invalid argument". Use the
  simulator tool's screenshot instead.

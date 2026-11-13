# Native UIs

`apple/Compono` is a Swift package for SwiftUI, and `android/compono` is an Android
library for Compose. Neither depends on generated bindings.

`showcase` draws the same screens as the Rust UI examples. Its entry points,
`ios/Showcase/ShowcaseMain.swift` and `android/app/src/main/kotlin/compono/showcase/MainActivity.kt`,
show the whole setup an application needs: `ComponoRoot` on Apple systems and
`retainedTree` on Android.

As in Decompose, each screen takes its component object, not a node address. The
Rust side exports one class per component from `showcase/ffi/src/feature/`, with
`model()` and its actions, and a parent hands out its children's objects. Screens
are grouped by feature under `ui/<feature>/` in both apps, drawn with Material 3
and its adaptive layouts on Android and with Liquid Glass on iOS 26, falling back
to materials on iOS 17. A screen subscribes to its component with
`observe(component.node())`.

## Building the showcase

Needs `boltffi` 0.32.1, and an NDK for Android.

```bash
(cd showcase/ffi && boltffi pack apple --release --deny-skipped)
```

```bash
(cd showcase/ffi && ANDROID_NDK_HOME=/path/to/ndk boltffi pack android --release --deny-skipped)
```

Open `showcase/ios/Showcase.xcodeproj` in Xcode, or `showcase/android` in Android Studio.

## Checks

```bash
(cd apple/Compono && swift test)
```

```bash
(cd showcase/android && gradle :compono:testDebugUnitTest :app:assembleDebug)
```

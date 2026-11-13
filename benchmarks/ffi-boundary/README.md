# FFI boundary benchmark

Measures BoltFFI and UniFFI on the same Rust scenario through `compono-ffi`, so that
only the boundary differs. This is a separate Cargo workspace, which keeps binding
generators out of Compono's own dependencies.

`tap_with_commit` is the path a real screen uses: the native side sends a message by
address, the component updates, and the commit calls back into the native side.

## Results

Median nanoseconds per call of 31 rounds, release build, `panic = "abort"`.

### iOS simulator (iOS 27.2, Apple Silicon)

| Scenario | BoltFFI | UniFFI | UniFFI / BoltFFI |
|---|---:|---:|---:|
| `noop` | 0.7 | 1.8 | 2.6 |
| `echo` | 0.7 | 1.8 | 2.6 |
| `row`, one record | 66.9 | 516.7 | 7.7 |
| `rows_30` | 408.0 | 12,280.0 | 30.1 |
| `rows_1000` | 16,288.5 | 402,279.0 | 24.7 |
| `entries_30`, router structure | 489.6 | 12,230.2 | 25.0 |
| `dirty_30`, a commit's addresses | 48.5 | 4,281.9 | 88.3 |
| `article`, record with strings | 246.6 | 1,029.8 | 4.2 |
| `model`, one number from the tree | 8.7 | 10.2 | 1.2 |
| `tap_with_commit` | 68.3 | 507.7 | 7.4 |

### Android emulator (API 36.1, arm64, 16 KB pages, AOT-compiled with `speed`)

| Scenario | BoltFFI | UniFFI | UniFFI / BoltFFI |
|---|---:|---:|---:|
| `noop` | 16.8 | 3,278.0 | 195.1 |
| `echo` | 16.1 | 3,423.4 | 212.6 |
| `row`, one record | 148.1 | 9,160.6 | 61.9 |
| `rows_30` | 534.6 | 9,817.4 | 18.4 |
| `rows_1000` | 14,571.8 | 28,133.6 | 1.9 |
| `entries_30`, router structure | 572.6 | 9,812.4 | 17.1 |
| `dirty_30`, a commit's addresses | 254.4 | 9,475.8 | 37.2 |
| `article`, record with strings | 611.4 | 9,814.1 | 16.1 |
| `model`, one number from the tree | 23.2 | 3,478.3 | 149.9 |
| `tap_with_commit` | 136.0 | 13,497.4 | 99.2 |

Neither is a device: a simulator runs natively on Apple Silicon, and the emulator uses
hardware virtualization. Commit addresses cross as `i64` in both bindings, because
BoltFFI 0.32.1 cannot bind a Kotlin `ULongArray` parameter on a callback: the Kotlin
compiler mangles that method's name, and `JNI_OnLoad` fails.

## Decision

BoltFFI. UniFFI calls through JNA on Android, which costs about 3 µs per call before
any data moves, and `tap_with_commit` is 99 times slower there. On iOS the same path
is 7 times slower. BoltFFI is younger, so the choice stays behind `compono-ffi`, which
depends on no generator.

## Running

Needs `boltffi` 0.32.1, Xcode, and an Android SDK with an NDK.

```bash
(cd bolt && boltffi pack apple --release --deny-skipped)
```

```bash
(cd bolt && ANDROID_NDK_HOME=/path/to/ndk boltffi pack android --release --deny-skipped)
```

```bash
cargo build --release -p uni-bench
```

```bash
cargo build --release -p uni-bench --target aarch64-apple-ios-sim
```

```bash
cargo ndk -t arm64-v8a -P 26 build --release -p uni-bench
```

```bash
cargo run --release -p uni-bench --bin uniffi-bindgen -- generate --library target/release/libuni_bench.dylib --language swift --out-dir uni/dist/swift
```

```bash
cargo run --release -p uni-bench --bin uniffi-bindgen -- generate --library target/release/libuni_bench.dylib --language kotlin --out-dir uni/dist/kotlin
```

```bash
harness/run-apple.sh <simulator udid>
```

```bash
GRADLE=/path/to/gradle ANDROID_SERIAL=<device> harness/run-android.sh
```

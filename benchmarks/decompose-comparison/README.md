# Compono and Decompose

The same scenarios on Compono and on Decompose 3.6.1, on the same machine. Decompose
is used through its public API, with one subscriber on each `Value`, as a UI would
have. Compono has a `Host` that counts commits. Saved state is off on both sides.

Times are median nanoseconds per operation over 31 rounds after 20 rounds of warmup.

## Results

### This Mac: Compono native against Decompose on the JVM

| Scenario | Compono | Decompose, JVM 21 | Ratio |
|---|---:|---:|---:|
| Stack push and pop, depth 1 | 80.0 | 1,416 | 18x |
| Stack push and pop, depth 10 | 75.9 | 2,914 | 38x |
| Stack push and pop, depth 100 | 76.1 | 22,108 | 291x |
| Push 1000 and pop to root, per entry | 64.3 | 58,475 | 909x |
| Slot open and close | 63.3 | 1,028 | 16x |
| Select next of 100 pages | 149 | 9,690 | 65x |
| Move a 30 row window of 100 000 by one | 534 | 3,396 | 6.4x |
| Update a child and notify | 7.5 | 15.5 | 2.1x |
| Create and destroy a tree | 198 | 2,305 | 12x |
| Memory per stack entry, bytes | 440 | 1,590 | 3.6x |

### This Mac: Compono native against Decompose on Kotlin/Native

| Scenario | Compono | Decompose, macOS arm64 | Ratio |
|---|---:|---:|---:|
| Stack push and pop, depth 1 | 80.0 | 8,751 | 109x |
| Stack push and pop, depth 10 | 75.9 | 18,142 | 239x |
| Stack push and pop, depth 100 | 76.1 | 59,415 | 781x |
| Push 1000 and pop to root, per entry | 64.3 | 170,202 | 2,647x |
| Slot open and close | 63.3 | 3,195 | 50x |
| Select next of 100 pages | 149 | 25,495 | 171x |
| Move a 30 row window of 100 000 by one | 534 | 8,631 | 16x |
| Update a child and notify | 7.5 | 78.0 | 10x |
| Create and destroy a tree | 198 | 4,167 | 21x |

### iOS simulator

| Scenario | Compono | Decompose, Kotlin/Native | Ratio |
|---|---:|---:|---:|
| Stack push and pop, depth 1 | 96.5 | 4,591 | 48x |
| Stack push and pop, depth 10 | 86.4 | 9,329 | 108x |
| Stack push and pop, depth 100 | 89.9 | 59,138 | 658x |
| Push 1000 and pop to root, per entry | 106 | 169,131 | 1,602x |
| Slot open and close | 69.2 | 3,179 | 46x |
| Select next of 100 pages | 164 | 25,484 | 155x |
| Move a 30 row window of 100 000 by one | 453 | 8,427 | 19x |
| Update a child and notify | 7.8 | 73.8 | 9.5x |
| Create and destroy a tree | 211 | 4,182 | 20x |

### Android emulator

| Scenario | Compono | Decompose, ART | Ratio |
|---|---:|---:|---:|
| Stack push and pop, depth 1 | 310 | 24,346 | 78x |
| Stack push and pop, depth 10 | 256 | 45,315 | 177x |
| Stack push and pop, depth 100 | 298 | 308,551 | 1,034x |
| Push 1000 and pop to root, per entry | 299 | 767,718 | 2,567x |
| Slot open and close | 232 | 14,870 | 64x |
| Select next of 100 pages | 560 | 112,177 | 200x |
| Move a 30 row window of 100 000 by one | 2,526 | 29,516 | 12x |
| Update a child and notify | 42.0 | 197 | 4.7x |
| Create and destroy a tree | 695 | 20,169 | 29x |
| Memory per stack entry, bytes | 440 | 1,392 | 3.2x |

## Reading these numbers

- Compono's stack costs the same at every depth. Decompose's cost grows with depth.
- Absolute times matter more than ratios for single actions. A navigation that takes
  24 µs in Decompose on the emulator is far below one 16 ms frame. The difference
  shows in deep stacks, in many components, and in memory.
- A Decompose child gets a full `ComponentContext`: lifecycle, state keeper, instance
  keeper and back handler. A Compono child pays for only what it uses.
- On Android, Compono runs as a native program here. Reaching it from Compose adds the
  boundary cost in `benchmarks/ffi-boundary`, about 136 ns for a message and its commit.
- Decompose's Android run uses a tenth of the iterations, because the full run did not
  finish in twenty minutes. Its times are per operation and remain comparable.
- Neither the simulator nor the emulator is a device. Kotlin/Native memory is not
  measured, because the harness has no live heap counter there.

## Running

Needs `GRADLE`, `JAVA_HOME` set to JDK 21, and for devices `ANDROID_NDK_HOME` and
`ANDROID_SERIAL`.

```bash
./run-host.sh
```

```bash
./run-devices.sh <simulator udid>
```

Results are written to `out/`.

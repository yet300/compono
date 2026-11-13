---
name: rust-proof-techniques
description: Proves concurrency, invariants and type sizes in Rust instead of trusting a few examples. Use when changing atomics or wake-up code, a reconciling or mirroring algorithm, or a type whose size the docs promise.
---

# Proving Rust properties

Ordinary tests check the cases someone thought of. These four techniques check
the rest. Each rule file is a full reference; load the one the change needs.

| Change | Technique | Rule |
|---|---|---|
| An atomic load, store or read-modify-write | Choose the weakest correct `Ordering`, and pair every `Release` with an `Acquire` | [conc-atomic-ordering](rules/conc-atomic-ordering.md) |
| A lock-free hand-off between threads | Model-check every interleaving with `loom` | [test-loom-concurrency](rules/test-loom-concurrency.md) |
| An algorithm with a law, such as a round trip or an invariant after any sequence of operations | Generate inputs and shrink failures with `proptest` | [test-proptest-properties](rules/test-proptest-properties.md) |
| A type the documentation gives a size for | Assert the size at compile time | [mem-assert-type-size](rules/mem-assert-type-size.md) |

## Where they apply in Compono

- `crates/compono/src/concurrency` takes its `Mutex` and atomics from
  `concurrency/sync.rs`, which swaps in loom's under `cfg(loom)`. The models are in
  `crates/compono/tests/loom.rs`; `AGENTS.md` has the command. A new primitive in
  that module comes from `sync.rs` and gets a model.
- Keep state that must change together under one lock. loom 0.7 lets an atomic
  swap read a value the C11 model forbids once a mutex orders it, so a protocol that
  mixes a flag with a lock cannot be verified there. `WakeSignal` keeps its flag and
  listener under one mutex for that reason.
- Other atomics, in `crates/compono-ffi/src/native_tree.rs` and
  `crates/compono-egui/src/host.rs`, are not modelled. A change to them states the
  ordering it relies on.
- Properties live in `crates/compono/tests/properties.rs` (`ChildSet`, `Stack` with
  a restore) and in `crates/compono-web/tests/history.rs` (browser history against a
  stack). A new router or mirroring algorithm gets one there.
- Type sizes promised by `ARCHITECTURE.md` are asserted next to the type, as in
  `crates/compono/src/component/handle.rs`.
- Proptest writes a `*.proptest-regressions` file beside a test that fails. Keep it
  when it records a real bug, so the case runs first from then on.

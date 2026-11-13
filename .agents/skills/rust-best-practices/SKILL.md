---
name: rust-best-practices
description: "Write and review production Rust using the official API Guidelines, Style Guide, and this engineering standard: allocation contracts, ownership, Result vs panic, debug_assert, Clippy, tests, rustdoc, unsafe, and Cargo CI. Use when writing, reviewing, or refactoring Rust; choosing borrow vs clone; designing crate APIs; handling errors; bounding heap use; configuring clippy or rustfmt; or when the user runs /rust-best-practices. Do not use for other languages. Triggers: rust, rustc, cargo, clippy, rustfmt, ownership, clone, borrow, Result, unwrap, expect, panic, thiserror, anyhow, heapless, no_std, allocation, debug_assert, type-state, Send, Sync, unsafe, FFI, rustdoc, MSRV, rust-best-practices, rust style, rust guidelines"
license: MIT
metadata:
  author: massimodeluisa
  version: "1.0.0"
  website: https://www.rust-lang.org/
---

# Rust Best Practices

Apply this standard when writing or reviewing Rust. Engineering guide, not a tutorial.

## Authority

1. **Live official docs** for language, naming, formatting, and public API shape. Start with the [Style Guide](https://doc.rust-lang.org/stable/style-guide/) and the [API Guidelines](https://rust-lang.github.io/api-guidelines/). Map: [references/official.md](references/official.md).
2. **This skill** for allocation contracts, bounds, `debug_assert!`, heapless cores, determinism, and CI. Load the matching reference; do not invent a rule that lives there.

MUST / MUST NOT: required unless an approved architecture decision documents an exception.
SHOULD / SHOULD NOT: default; a deviation needs a concrete reason in review.
MAY: optional and context-dependent.

Correctness, safety, determinism, and maintainability outrank cleverness. Measure performance. Design allocation and bounds into the API before profiling.

Fetch current official docs for library or toolchain syntax. Do not invent APIs from memory.

## How to use

- `/rust-best-practices`: apply the standard to the current Rust work.
- `/rust-best-practices <path>`: review that crate or file. For each finding: quote the line, name the rule (official `C-*` id or a heading in this skill), give the fix.

Load only what the task needs, same turn, in parallel:

| Task | Read |
|------|------|
| Naming, rustdoc sections, official checklist | [official.md](references/official.md) |
| Memory profile, heapless, output buffers, hidden alloc | [allocation.md](references/allocation.md) |
| Borrow vs clone, Copy, `into_`/`to_`/`as_` | [ownership.md](references/ownership.md) |
| `Result`, panic, `thiserror`/`anyhow`, checked math, `debug_assert!` | [errors.md](references/errors.md) |
| Functions, iterators, dispatch, type-state, boolean blindness | [design.md](references/design.md) |
| Pointers, `Send`/`Sync`, `unsafe`, FFI | [unsafe.md](references/unsafe.md) |
| Measure, layout, clones, flamegraph | [performance.md](references/performance.md) |
| rustfmt, Clippy, tests, docs, deps, MSRV, CI | [quality.md](references/quality.md) |
| Review pass, anti-patterns | [review.md](references/review.md) |

## Always-on rules

### Types and APIs

- MUST make invalid states hard to represent: newtypes, enums, `Option`, `Result`, type-state where lifecycle matters.
- MUST keep fields private unless direct access is the contract ([C-STRUCT-PRIVATE](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-struct-private)).
- MUST follow official `as_` / `to_` / `into_` conversion naming ([C-CONV](https://rust-lang.github.io/api-guidelines/naming.html#c-conv)).
- SHOULD return values from general library APIs ([C-NO-OUT](https://rust-lang.github.io/api-guidelines/predictability.html#c-no-out)). MUST use caller-owned buffers when the declared memory profile is heapless or allocation-free ([C-CALLER-CONTROL](https://rust-lang.github.io/api-guidelines/flexibility.html#c-caller-control)).
- NEVER use a raw `u32` for every identifier merely because the representation matches.

### Memory

- MUST declare a memory profile for each crate and performance-sensitive public operation: **strict heapless**, **allocation-free steady state**, or **allocation-conscious**. Details: [allocation.md](references/allocation.md).
- MUST keep runtime work bounded (iterations, recursion, queues, retries, output, concurrency).
- MUST report capacity exhaustion as a typed error. MUST NOT spill to the heap, drop entries, or switch to an unbounded fallback in silence.
- NEVER treat `Cow::into_owned()`, `format!()`, or a spill-capable small-vector as heapless.

### Ownership

- SHOULD take `&T`, `&str`, `&[T]`, `&Path` unless ownership transfer is required.
- SHOULD pass small `Copy` values by value. Measure ABI and target when size is in question; do not apply a universal byte cutoff.
- NEVER clone to silence the borrow checker.

### Errors and asserts

- MUST return `Result<T, E>` for recoverable failure. MUST NOT `unwrap()` / `expect()` on production paths.
- MUST use checked arithmetic at trust boundaries before indexing or allocating.
- MUST validate external input in all builds. NEVER use `debug_assert!` as caller validation or as the only `unsafe` precondition.
- SHOULD use `thiserror` in libraries, `anyhow` only at binary boundaries.

### Unsafe and concurrency

- MUST `#![forbid(unsafe_code)]` in crates that do not need unsafe.
- MUST document every unsafe block with `SAFETY:`.
- MUST NOT add unsafe `Send`/`Sync` impls without a written proof.
- SHOULD prefer a single owner or bounded message passing over `Arc<Mutex<T>>` as an ownership escape hatch.

### Quality

- MUST run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` in CI (explicit feature matrix if `--all-features` is invalid).
- SHOULD prefer `#[expect(...)]` over `#[allow(...)]`, local and justified.
- MUST link every committed TODO to an issue: `// TODO(#123): ...`.

## Official docs

Cite the live page. Do not restate official rules as if they originated here. Minimum set: Style Guide, API Guidelines checklist, `debug_assert!`, `no_std`, `alloc`, Clippy configuration, Cargo profiles. Full list: [official.md](references/official.md).

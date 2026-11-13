# Lint, test, docs, dependencies, and CI

## Formatting

Use `rustfmt`. Do not debate formatting it owns. Optional `rustfmt.toml` for import grouping (nightly rustfmt may be required for some options; check current [rustfmt docs](https://rust-lang.github.io/rustfmt/)):

```toml
reorder_imports = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

Order: `std` / `core` / `alloc`, external crates, workspace crates, `super` / `crate`.

## Clippy

CI MUST treat warnings as failures:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

If workspace features are mutually exclusive, replace `--all-features` with an explicit tested matrix. `--all-targets` covers lib, tests, benches, examples. `--locked` requires `Cargo.lock` up to date.

MAY add `-W clippy::pedantic` or `-W clippy::nursery` locally; do not enable a broad set then scatter suppressions. Tune to the codebase and MSRV.

Workspace lints (each member: `[lints] workspace = true`):

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unused_must_use = "deny"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
dbg_macro = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unwrap_used = "deny"
redundant_clone = "deny"
```

Fix warnings. Prefer `#[expect(clippy::lint)]` with a reason when MSRV supports it; otherwise the narrowest `#[allow]` plus why. A suppression MUST be local, tied to one lint, justified, and removable when the constraint changes. Do not disable a lint at workspace scope to spare one call site.

Lints that usually matter: `redundant_clone`, `needless_borrow`, `map_unwrap_or`, `manual_ok_or`, `large_enum_variant`, `unnecessary_wraps`, `clone_on_copy`, `needless_collect`. Index: <https://rust-lang.github.io/rust-clippy/master/index.html>.

### Allocation guardrails (strict crates)

`clippy.toml` (guardrail, not a proof; misses custom types, deps, callbacks, FFI, logging, trait methods):

```toml
disallowed-types = [
  { path = "alloc::boxed::Box", reason = "heap allocation is forbidden in strict runtime code", allow-invalid = true },
  { path = "alloc::string::String", reason = "borrow text or use fixed-capacity storage", allow-invalid = true },
  { path = "alloc::vec::Vec", reason = "use caller-owned slices or fixed-capacity storage", allow-invalid = true },
  { path = "std::boxed::Box", reason = "heap allocation is forbidden in strict runtime code", allow-invalid = true },
  { path = "std::string::String", reason = "borrow text or use fixed-capacity storage", allow-invalid = true },
  { path = "std::vec::Vec", reason = "use caller-owned slices or fixed-capacity storage", allow-invalid = true },
]

disallowed-macros = [
  { path = "alloc::format", reason = "write into a caller-owned or fixed-capacity sink", allow-invalid = true },
  { path = "alloc::vec", reason = "use arrays, slices, or fixed-capacity storage", allow-invalid = true },
  { path = "std::format", reason = "write into a caller-owned or fixed-capacity sink", allow-invalid = true },
  { path = "std::vec", reason = "use arrays, slices, or fixed-capacity storage", allow-invalid = true },
]
```

```rust
#![deny(clippy::disallowed_macros)]
#![deny(clippy::disallowed_types)]
```

## Tests

Tests are executable documentation. A name SHOULD state the condition and expected result: `parse_returns_output_too_small_when_capacity_is_insufficient`. Prefer one behavioral reason for failure per test. Multiple assertions are fine when they jointly establish that one behavior.

Group related tests in a `mod` named after the unit (`process::returns_error_when_empty`) so IDEs can run the module and `cargo test` prints the path.

Share setup and fixtures (`rstest` cases with descriptive names). Keep each test's action and assertion inline even when repetitive. Tests have no tests: logic in a shared helper is untested. A failing test SHOULD be diagnosable on sight.

`Ok` cases SHOULD print the `Err` on failure (`assert!` message or `unwrap` with context). Prefer `assert_eq!` with a useful message. Since Rust 1.96, prefer `assert_matches!` over `assert!(matches!(...))` when available. `#[should_panic]` only when panic is the desired behavior.

### Layers

| Layer | Lives | Scope |
|-------|--------|--------|
| Unit | next to the code, `#[cfg(test)]` | private items, edge cases, error variants |
| Integration | `tests/` | public API, crate boundaries, features, allocation contracts |
| Doc | `///` examples | public usage; `cargo test` runs them, `cargo nextest` does not (`cargo t --doc`) |
| Property | dedicated | broad invariants |
| Fuzz | dedicated | parsers, protocol decoders, unsafe, malformed input |
| Differential | dedicated | optimized impl vs safe reference |
| Concurrency | dedicated | ordering and sync when needed |

Doc-test attributes: `ignore` (prefer `text` for non-code), `should_panic`, `no_run` (compile only), `compile_fail` (illegal use). Hide setup lines with `#`. Examples SHOULD use `?`, not `unwrap` ([C-QUESTION-MARK](https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark)).

### Boundaries to test

Allocation-free and bounded APIs MUST test: empty input; one element; exact capacity; one less than required; maximum declared capacity; malformed offsets and lengths; integer overflow boundaries; duplicate and equal-key behavior; deterministic tie-breaking; repeated warm execution; every recoverable error variant; optional instrumentation paths; feature-disabled configs.

Also run `cargo test --workspace --profile release-assertions`. Isolate debug-only panics in a focused test.

### Allocation counting

Prove a path with a test-only `GlobalAlloc` wrapper (narrow `unsafe`, not production). Capture allocation/deallocation counters **after** fixture setup, run the API, assert counters unchanged. Run serially:

```bash
cargo test --test no_alloc -- --test-threads=1
```

This proves only the paths and inputs exercised. Combine with API review, dependency review, `no_std` / no-`alloc` compilation, malformed-input tests, capacity tests, call-graph inspection, Clippy guardrails.

### Snapshots

[`cargo insta`](https://insta.rs/docs/quickstart/) for structured diagnostics, generated code, canonical serialization, CLI output. Keep snapshots small, deterministic, human-reviewable. Named snapshots. Redact timestamps, uuids, randomness. Commit snapshots. Do not snapshot primitives or critical-path logic that needs a precise `assert_eq!`. YAML snapshots of serializable values review well.

Useful extras: `pretty_assertions` for diffs; `rstest` for table-driven cases.

## Documentation

`//` comments explain **why**: non-obvious invariants, algorithmic rationale, compatibility, safety proofs, measured tradeoffs, spec references, why a simpler-looking impl is wrong. Name them: `SAFETY:`, `PERF:`, `CONTEXT:` with an ADR link.

Do not narrate syntax (`// increment the index`). Stale comments are worse than none. If a comment describes a requirement types, constructors, enums, validation, or tests can enforce, prefer enforcement.

`///` / `//!` explain the **contract**: purpose, inputs/outputs, ownership and lifetimes, **`# Allocation`**, finite limits and capacity, `# Errors`, `# Panics`, `# Safety`, determinism and ordering, complexity when meaningful, examples.

```rust
/// Parses records into caller-owned storage.
///
/// # Allocation
///
/// Performs no heap allocation. Temporary state is held in scalar locals and
/// the caller-provided `output` slice.
///
/// # Errors
///
/// Returns [`ParseError::OutputTooSmall`] without modifying elements beyond
/// the returned initialized prefix.
///
/// # Panics
///
/// Does not panic for malformed input.
```

`//!` at crate/module top: what it is for, exports, invariants, a short example.

Every committed TODO SHOULD reference an issue: `// TODO(#421): Replace the scalar verifier after SIMD equivalence tests exist.`

Lints: `missing_docs`, `broken_intra_doc_links`, `empty_docs`, `missing_panics_doc`, `missing_errors_doc`, `missing_safety_doc`. Libraries MAY `#![deny(missing_docs)]`. [`#[non_exhaustive]`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute) on public enums callers may match.

## Dependencies, features, MSRV

Core runtime crates SHOULD have the smallest practical dependency surface. Every dependency can introduce allocation, feature unification, platform assumptions, unsafe, build scripts, vulnerabilities, binary size, MSRV pressure. Do not add a crate for a trivial helper that is clearer locally.

Disable default features intentionally:

```toml
some-crate = { version = "1", default-features = false, features = ["required-feature"] }
```

Test the intended matrix, including `--no-default-features`. A feature enabled elsewhere in the workspace can change the unified graph. Audit the resolved graph, not one manifest line.

Do not make a heapless core depend on CLI parsing, async runtimes, telemetry, filesystem, HTTP, or rich error frameworks. Those belong in outer crates.

Keep `Cargo.lock` committed for applications and workspaces. Audit licenses and vulnerabilities (`cargo audit`, `cargo deny`) as appropriate. Review build scripts and proc macros as supply-chain code.

If the project promises an MSRV, test it in CI. Do not use new syntax or std APIs without bumping MSRV or providing a compatible alternative.

## Determinism

When output is content-addressed, signed, cached, compared byte-for-byte, or used as a reproducibility artifact, determinism is correctness.

Define and test: input ordering; stable IDs; sorting and tie-breaking; hash-map independence; random seed policy; concurrency merge order; integer overflow semantics; architecture-independent widths and endianness; serialization field order; canonical padding and alignment; error ordering when multiple failures are possible.

Parallel execution MAY change completion time; it MUST NOT change canonical bytes when determinism is in the contract.

After a release-active canonicalize step: `debug_assert!(records.windows(2).all(|pair| pair[0].key <= pair[1].key));`

Do not rely on iteration order from a container unless that order is guaranteed and appropriate for the format.

## Cargo and CI baseline

```toml
[profile.release]
overflow-checks = true

[profile.release-assertions]
inherits = "release"
debug-assertions = true
overflow-checks = true
```

Choose LTO, codegen units, panic strategy, and stripping from measured build, size, diagnostics, and deploy needs. Do not copy a profile blindly across every crate and target.

Required:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --doc --locked
cargo test --workspace --profile release-assertions
cargo test --test no_alloc -- --test-threads=1
```

Add as applicable:

```bash
cargo check -p project-core --no-default-features
cargo miri test -p project-core
cargo test --workspace --release
cargo audit
cargo deny check
```

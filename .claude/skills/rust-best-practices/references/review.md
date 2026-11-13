# Review checklist and anti-patterns

Use this on `/rust-best-practices <path>` and on every Rust change. Quote the line, name the rule, give the fix.

## Correctness and API

- Invalid states represented with types, not conventions?
- Caller-controlled failures returned as typed errors?
- Integer and range calculations checked before indexing?
- Panic conditions rare and documented under `# Panics`?
- Ownership transfer intentional and visible (`into_` / `to_` / `as_`)?
- Public fields and types actually required?
- Official [checklist](https://rust-lang.github.io/api-guidelines/checklist.html) `C-*` ids that apply?

## Allocation and bounds

- Memory profile declared (strict heapless / allocation-free steady state / allocation-conscious)?
- Hot or repeated operations avoid heap allocation and reallocation?
- Inputs borrowed; outputs or scratch caller-owned where the profile requires it?
- All capacities finite and documented?
- Exhaustion returns an explicit error without spilling to the heap?
- Hidden paths through formatting, telemetry, async, traits, callbacks, FFI, and dependencies reviewed?
- Large buffers kept off limited thread stacks?

## `debug_assert!`

- Each assertion a real internal invariant?
- Already established by types or release-active checks?
- Release behavior still correct if the assertion is removed?
- Free of required side effects?
- Independent of unsafe-code soundness?
- Condition and message avoid owned allocation?

## Performance and determinism

- Optimization supported by measurement (`--release`)?
- Data layout contiguous and cache-conscious where relevant?
- Unnecessary clones, copies, and intermediate collections avoided?
- Canonical output independent of thread scheduling and unordered containers?
- Optimized implementation has a reference oracle and equivalence tests?

## Safety and concurrency

- Unsafe forbidden or tightly isolated with `SAFETY:`?
- FFI ownership and allocator rules explicit?
- Queues, workers, retries, and temporary storage bounded?
- Shared ownership used intentionally, not as an escape hatch?

## Tests and documentation

- Success, capacity, malformed-input, and error paths tested?
- Allocation behavior tested after initialization?
- Optimized tests run with debug assertions enabled?
- Public APIs document allocation, errors, panics, bounds, and determinism?
- TODOs linked to issues?
- Examples compile as doc tests where practical?

## Anti-patterns

Avoid unless a documented exception explains why they are correct:

- cloning to silence the borrow checker
- accepting `String` or `Vec<T>` when only a borrow is needed
- returning a `Vec<T>` from every query or parser (especially under a heapless or allocation-free profile)
- `unwrap()` / `expect()` for external input
- `debug_assert!` as validation or a safety precondition
- state changes inside debug assertions
- `format!()` in a strict runtime error path
- preallocating a `Vec` and calling the subsystem "heapless"
- a small-vector type that may spill to the heap on a strict path
- hiding allocations behind logging or error context
- boxing futures or traits in a hot path without measuring or documenting it
- unbounded channels, retries, recursion, or task creation
- assigning canonical IDs based on thread completion order
- replacing clear code with an abstraction that obscures bounds and memory access
- writing unsafe before proving safe code is insufficient
- disabling lints globally to accommodate one call site
- claiming allocation freedom from source inspection or one benchmark input alone
- implementing both `Copy` and `Iterator` on one type
- wildcard imports in production modules
- TODOs with no issue, owner, or removal condition
- comments that restate the next line

## Preferred shape

Production Rust should be easy to reason about under success and failure:

- explicit domain types
- borrowed inputs
- caller-owned or fixed-capacity output and scratch where the profile requires it
- bounded execution
- typed, allocation-free core errors
- release-active validation of external input
- `debug_assert!` for internal invariants already established by correct code
- no unsafe by default
- deterministic output where artifacts or proofs depend on it
- measurement-backed optimization
- tests that verify allocation behavior and boundary conditions
- rich application adapters separated from a lean runtime core

A zero-allocation claim is a contract. Treat it with the same rigor as memory safety, wire-format compatibility, and deterministic output.

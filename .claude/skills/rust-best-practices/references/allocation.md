# Allocation, bounds, and memory profiles

Every crate, module, and performance-sensitive public operation MUST declare one profile.

| Profile | Contract | Typical use |
|---------|----------|-------------|
| **Strict heapless** | No heap-backed storage. Prefer `#![no_std]`. Do not import `alloc`. | Core runtimes, embedded, parsers and kernels that must prove no heap |
| **Allocation-free steady state** | Init MAY allocate. The named operation and its complete transitive call graph perform zero allocation or reallocation after init | Servers, reusable engines, per-request or per-token execution |
| **Allocation-conscious** | Allocation only at explicit boundaries, justified, bounded where possible, measured when performance-sensitive | CLI, compilers, build tools, admin services, offline analysis |

Default for runtime, protocol, parser hot paths, deterministic kernels, and repeated request processing: **strict heapless** or **allocation-free steady state**.

## What a claim actually means

These are different:

- this function does not call `Vec::new()`
- this function performs no allocation on the exercised path
- this function and every transitive callee perform no allocation for all valid inputs
- this subsystem never uses heap-backed storage

Only the last is a strict no-heap guarantee.

A function taking `&[T]` backed by a caller `Vec<T>` MAY itself be allocation-free while the system is not heapless. A preallocated `Vec<T>` MAY satisfy steady-state zero-allocation if it never grows; it still uses heap and is not strict heapless.

## Core principles

### Invalid states

Use the type system: newtypes for ids, offsets, lengths, units, scores, protocol values; enums instead of loosely related booleans or sentinel integers; `Option<T>` not magic values; `Result<T, E>` for recoverable failure; type-state when operations depend on lifecycle; fixed-width integers at serialization and FFI; private fields unless access is the contract.

```rust
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionStatus {
    Exact,
    BackedOff,
    Unsupported,
}
```

### Explicit control flow

Signatures MUST make visible: allocation, capacity limits, error behavior, ownership transfer, mutation, blocking vs async, determinism, panic conditions, thread-safety.

Avoid APIs whose correctness depends on undocumented global state, implicit init, hidden retries, lazy allocation, or environment-dependent defaults.

### Bounded runtime

MUST bound: iterations, candidate counts, recursion depth, queue length, output size, temporary storage, retry count, concurrency, bytes read or written where practical.

On limit: typed error or explicit status. MUST NOT silently allocate more, recurse without a bound, grow an unbounded queue, or switch to an unbounded fallback.

### Rich construction, lean execution

Compiler, CLI, migration, test, and offline-analysis code MAY use rich owned structures. Runtime SHOULD consume compact validated views and caller-owned state:

```text
allocating authoring/compiler layer
    -> validates, normalizes, sorts, packs, serializes
immutable packed artifact
    -> borrowed by
a bounded, allocation-free runtime
```

MUST NOT deserialize a packed artifact into a heap-resident object graph merely for convenience when the runtime can borrow validated slices.

### Reference implementation

Optimized implementations MUST stay behaviorally equivalent to a clear, safe reference path. Keep that path readable as: normative semantics, differential-testing oracle, portability fallback, property-test basis, reviewable spec for the optimized code.

## Allocation is part of the API

Public runtime APIs SHOULD make allocation unnecessary and obvious from signatures.

Prefer: `&T`, `&mut T`, `&[T]`, `&mut [T]`, `&str`; caller-owned output and scratch; fixed-size arrays for genuinely small bounds; fixed-capacity containers that cannot spill; borrowed views into validated bytes; iterators instead of collected results; returned lengths, ranges, and status instead of owned collections; static or enum dispatch instead of boxed trait objects; small copyable error enums.

Avoid:

```rust
fn parse(input: String) -> Vec<Record>;
```

Prefer (heapless / allocation-free):

```rust
fn parse(input: &[u8], output: &mut [Record]) -> Result<usize, ParseError>;
```

Ordinary library APIs that are allocation-conscious SHOULD still return values ([C-NO-OUT](https://rust-lang.github.io/api-guidelines/predictability.html#c-no-out)). Caller-owned buffers are the exception required by the memory profile, not a default for every crate.

## Do not move unbounded work onto the stack

Avoiding the heap is not permission to place arbitrarily large arrays on every thread stack.

Large storage SHOULD be: supplied by the caller; static when lifetime and sync permit; a bounded arena with a documented lifecycle; memory-mapped; partitioned into bounded chunks; a reusable worker-owned scratch region.

Review stack use against the smallest supported thread stack. Avoid unbounded recursion. Recursion is acceptable only when depth is statically or structurally bounded and documented.

## Capacity exhaustion MUST be explicit

A fixed-capacity structure MUST report exhaustion deterministically.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapacityError {
    Full { capacity: usize },
}
```

MUST NOT silently: spill into a heap allocation; discard existing entries; overwrite an unrelated entry; retry indefinitely; switch to an unbounded representation.

## Output-buffer pattern

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterError {
    OutputTooSmall { required: usize, provided: usize },
}

/// Copies even values into `output` and returns the initialized prefix length.
///
/// # Allocation
///
/// This function performs no heap allocation.
pub fn retain_even(input: &[u32], output: &mut [u32]) -> Result<usize, FilterError> {
    let required = input.iter().filter(|value| **value & 1 == 0).count();
    if output.len() < required {
        return Err(FilterError::OutputTooSmall {
            required,
            provided: output.len(),
        });
    }
    debug_assert!(required <= output.len());
    let mut written = 0usize;
    for &value in input {
        if value & 1 == 0 {
            debug_assert!(written < output.len());
            output[written] = value;
            written += 1;
        }
    }
    debug_assert_eq!(written, required);
    Ok(written)
}
```

The capacity branch is required in all builds (caller-controlled input). Later `debug_assert!` checks internal invariants already established by that control flow.

## Hidden allocation to review

Forbidden in strict heapless code. Also forbidden in an allocation-free operation unless constructed outside it and the exercised path is proven not to grow, allocate, or reallocate:

- `Vec<T>`, `String`, `Box<T>`, `Rc<T>`, `Arc<T>`, `PathBuf`, `OsString`
- `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `VecDeque`, `BinaryHeap`
- `Box::new`, `Box::pin`, `Rc::new`, `Arc::new`
- `vec![]`, `format!()`, `to_vec()`, allocating `to_owned()`, `to_string()`
- `collect::<Vec<_>>()`, `collect::<String>()`, equivalent owned collections
- growth through `push`, `insert`, `extend`, `reserve`, or implicit reallocation
- boxed trait objects, boxed iterators, boxed futures
- `Cow::into_owned()` and APIs that silently go from borrowed to owned
- spill-capable "small" containers unless spilling is structurally impossible
- lazy init that creates owned heap data on first use
- logging, tracing, metrics, serialization, backtrace, and error-context paths that have not been allocation-audited
- callbacks, trait methods, FFI, or third-party code whose transitive behavior is unknown

Syntax does not prove allocation. Iterators, closures, formatting arguments, trait calls, and `async fn` are not inherently allocating; a specific adapter, receiver, executor, or impl may be. Review the complete call graph.

## Formatting without owned strings

[`format_args!`](https://doc.rust-lang.org/std/macro.format_args.html) builds borrowed formatting arguments. The destination decides whether formatting allocates.

```rust
use core::fmt::{self, Write as _};

pub fn write_record(sink: &mut impl fmt::Write, id: u32, score: i32) -> fmt::Result {
    write!(sink, "id={id} score={score}")
}
```

Allocation-free only when the sink is. A `String` sink may grow; a fixed-capacity sink SHOULD return `fmt::Error` when full.

Avoid owned diagnostic strings in runtime code. Defer rich formatting to a higher-level adapter after the core returns a typed error.

## Sorting and selection

Where unstable order is acceptable, `sort_unstable*` and `select_nth_unstable*` are in-place and do not allocate. Do not drop a stability requirement merely to avoid allocation. Define required semantics, then choose or implement a bounded algorithm that matches them.

If deterministic output matters, define: total ordering, tie-breaking, equal keys, architecture-independent integers, canonical output order.

## Heapless errors at the core boundary

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    OffsetOutOfBounds { offset: usize, input_len: usize },
    OutputTooSmall { required: usize, provided: usize },
    CapacityExceeded { capacity: usize },
    IntegerOverflow,
}
```

Implement `Display` by writing to the formatter. Do not store a preformatted `String` for context. Application code MAY translate a core error into a richer allocating diagnostic after crossing the allocation-free boundary.

## Isolate strict code structurally

A strict core crate SHOULD start from:

```rust
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_macros)]
#![deny(clippy::disallowed_types)]
```

Do not add `extern crate alloc` to a crate claiming strict no-heap behavior. Filesystem, network, CLI, telemetry, and rich diagnostics belong in adapter crates.

```text
crates/
  project-core/       # no_std, no alloc, bounded algorithms
  project-format/     # packed types and validation
  project-runtime/    # allocation-free repeated execution
  project-std/        # std adapters, I/O, threading, telemetry
  project-cli/        # allocating application boundary
```

Clippy guardrails for disallowed types: [quality.md](quality.md).

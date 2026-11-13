# Functions, iterators, dispatch, and type-state

## Functions

Extract a function when it creates a semantic boundary, improves testing, centralizes an invariant, or removes duplicated **policy**.

Do not extract tiny fragments that obscure control flow, require many pass-through parameters, hide hot-path work, make ownership harder, or invent an abstraction with no stable meaning.

Keep functions single-purpose. Hot paths SHOULD make visible: bounds, memory access, temporary state, allocation, error exits, branch structure, ordering and tie-breaking.

### Duplication vs the wrong abstraction

Duplication is cheaper than the wrong abstraction. Wait for the third similar use before extracting, unless the name itself adds meaning (`is_retryable`, `authorize`).

DRY is about **knowledge**, not matching text. Two blocks that look alike but encode different decisions MUST stay separate. Unifying them with a `bool` flag is the smell.

Extract when: the logic appears in 3+ places **and** is the same decision; the name adds meaning; it is a unit worth testing; the signature stays small and honest.

Do not extract when: 1-2 lines used fewer than 3 times with no naming win; call sites are only almost identical; the only motive is line count; you are guessing a future abstraction.

In tests, tolerate duplication more: each test should read as setup, action, assertion without chasing helpers. Share fixtures; keep action and assertion inline. See [quality.md](quality.md).

If a helper has grown flag parameters and branches, unwind it: re-inline, reduce each site, then reconstruct only what is actually shared.

### Boolean blindness

Instead of `fn execute(strict: bool, retry: bool, audit: bool)`, use enums and policy structs:

```rust
pub enum ValidationMode { Strict, Compatible }

pub struct ExecutionPolicy {
    pub validation: ValidationMode,
    pub retry: RetryPolicy,
    pub audit: AuditPolicy,
}
```

Clippy: `fn_params_excessive_bools`.

### Visibility

Start private. Expand to `pub(crate)` or `pub` only when a real consumer needs it. Public APIs are compatibility obligations. Do not expose implementation collections, sync primitives, or third-party types if callers do not need them.

### Imports

Avoid wildcard imports outside controlled preludes and test modules. Import traits intentionally for extension methods. Grouping: [quality.md](quality.md). Do not alias well-known types unless resolving a collision or expressing domain meaning.

## Iterators and loops

Iterator adapters are generally lazy. Allocation usually happens at `collect` into an owned container, or inside a particular adapter or closure.

Prefer a borrowed iterator when callers can consume incrementally:

```rust
pub fn active_records(records: &[Record]) -> impl Iterator<Item = &Record> {
    records.iter().filter(|record| record.active)
}
```

Do not collect solely to return a convenient intermediate `Vec`.

### `for` vs iterator chains

A direct `for` is often clearest for: multiple mutable accumulators, explicit bounds and capacity checks, early exits, state machines, hot kernels whose generated code is inspected.

```rust
let mut written = 0usize;
for item in input {
    if keep(item) {
        if written == output.len() {
            return Err(CapacityError::Full { capacity: output.len() });
        }
        debug_assert!(written < output.len());
        output[written] = *item;
        written += 1;
    }
}
```

Prefer iterator chains when transforming collections or `Option`/`Result`, composing steps without early exit, using `.enumerate`, `.windows`, `.chunks`, or combining sources without extra collections.

Choose the form that makes correctness and bounds easiest to review. Do not rewrite a readable loop into a complex chain merely to look idiomatic. Do not rewrite a clear chain into a loop merely to look low-level.

Iterators are lazy: `.map` / `.filter` do nothing until a consumer (`.collect`, `.sum`, `.for_each`, `for`).

Prefer `.iter()` over `.into_iter()` unless ownership of the collection is required. For `Copy` element types, prefer `.iter()`.

For summing numbers prefer `.sum` over `.fold`.

### Avoid intermediate collections

Instead of map-collect then filter-collect, fuse the iterator or write into caller-owned storage.

### Closures are not free

A closure can capture owned state, clone, call allocating code, or force dynamic dispatch. Review captures on hot paths. Prefer borrowing captures. Use `move` only when ownership transfer is required.

## Generics and dispatch

Prefer static dispatch (generics / `impl Trait`) in hot and strict paths: no heap, no virtual call, monomorphized.

```rust
pub fn encode<W: ByteSink>(sink: &mut W, value: &Record) -> Result<(), W::Error> {
    Ok(())
}
```

Use static dispatch when implementations are known at compile time, performance matters, the code is a strict allocation-free core, and monomorphization cost is acceptable.

### `dyn Trait`

A borrowed `&dyn Trait` does not itself heap-allocate. `Box<dyn Trait>` does.

Use `dyn Trait` when implementations are selected at runtime, heterogeneous values share one interface, code size matters more than a virtual call, or the dynamic boundary is outside the hot path.

Prefer enum dispatch when the implementation set is closed and small.

Prefer `&dyn Trait` over `Box<dyn Trait>` when ownership is not required. `Arc<dyn Trait>` for shared access across threads.

Do not box inside structs unless beneficial or required (recursion). Box at the public API boundary, not internally.

Object-safe traits only: no generic methods, no `Self: Sized` requirement, methods take `self` / `&self` / `&mut self`.

A control plane MAY choose a backend dynamically while the chosen backend runs statically inside the hot operation. Do not force boxed trait objects through internal APIs because one outer layer needs runtime selection.

### Async does not prove allocation freedom

`async fn` returns a future value and does not inherently box. Allocation MAY still occur through boxed futures, dynamic async trait adapters, task spawning, executor storage, channels, captured owned buffers, I/O libraries, telemetry.

An allocation-free async claim MUST include the executor, spawning policy, adapters, and full call graph.

### Bound concurrency

Concurrency MUST NOT multiply memory without a bound. Define: max workers, queue capacities, per-worker scratch, backpressure, cancellation, deterministic merge order when output is canonical.

Prefer worker-local reusable scratch over shared `Mutex<Vec<_>>` accumulation. Avoid assigning semantic IDs through scheduling-dependent atomics when determinism matters.

## Type-state

Encode lifecycle in types when legal operations depend strongly on state and the extra types make the API clearer.

```rust
use core::marker::PhantomData;

pub struct Unvalidated;
pub struct Validated;

pub struct Artifact<'a, State> {
    bytes: &'a [u8],
    _state: PhantomData<State>,
}

impl<'a> Artifact<'a, Unvalidated> {
    pub fn validate(self) -> Result<Artifact<'a, Validated>, ValidationError> {
        validate_bytes(self.bytes)?;
        Ok(Artifact { bytes: self.bytes, _state: PhantomData })
    }
}

impl Artifact<'_, Validated> {
    pub fn records(&self) -> RecordIter<'_> {
        debug_assert!(header_is_valid(self.bytes));
        RecordIter::new(self.bytes)
    }
}
```

`debug_assert!` after construction checks a property already established. It MUST NOT be the only validation.

Use a small number of meaningful states. Avoid a generic-parameter explosion for incidental flags. When state is dynamic, externally supplied, or persisted, a runtime enum MAY be clearer.

Builders MAY use type-state so required fields are set before `build()` (compile error otherwise). Optional fields stay off the type-state axis.

Use type-state when it prevents illegal operations at compile time (uninitialized use, send-before-connect). Skip it for trivial enums, when runtime flexibility is required, or when generics become the product.

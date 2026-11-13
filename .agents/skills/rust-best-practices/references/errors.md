# Errors, panics, arithmetic, and `debug_assert!`

## `Option` for absence, `Result` for failure

Do not encode absence with empty strings, zero IDs, negative numbers, or invalid pointers.

```rust
pub fn record_at(records: &[Record], index: usize) -> Result<&Record, LookupError> {
    let record = records.get(index).ok_or(LookupError::OutOfBounds {
        index,
        len: records.len(),
    })?;
    debug_assert!(index < records.len());
    Ok(record)
}
```

Use combinators when they improve clarity. Use `match`, `if let`, or `let ... else` when control flow or error context is clearer explicitly.

### Pattern choice

- `match` when you need the inner `T` / `E` variants or a type change (`Result<T, E>` to `Result<Option<T>, E>`).
- `let PATTERN = EXPR else { diverging }` when the else is `return` / `continue` / `break` and does not need extra computation on the failed value.
- `if let` when both branches do real work.
- `?` when you do not need the `Err` value and policy is "propagate".

Prefer `.ok()`, `.ok_or()`, `.ok_or_else()` over a `match` that only converts `Result` to `Option`.

For `Result::Err` that must be logged then translated: `inspect_err` then `map_err`.

## No `unwrap` / `expect` on production paths

Production libraries MUST NOT use `unwrap()` or `expect()` for recoverable conditions. Tests, examples that demonstrate panic, and compile-time-proven constants MAY use them sparingly; a clearer assertion is often better.

Do not convert a recoverable error into a panic because handling it is inconvenient.

Prefer:

```rust
let Ok(json) = serde_json::from_str(&input) else {
    return Err(MyError::InvalidJson);
};
```

`unwrap_or` / `unwrap_or_else` / `unwrap_or_default` when a fallback value is the contract.

## Panic only for programmer defects

Caller-controlled input, malformed artifacts, capacity exhaustion, I/O failures, and unavailable resources are not programmer defects. Return a typed error.

A panic MAY be appropriate when an internal invariant is violated and continuing would indicate a defect. In libraries, keep panic conditions rare and document them under `# Panics` ([C-FAILURE](https://rust-lang.github.io/api-guidelines/documentation.html#c-failure)).

`todo!`, `unreachable!`, `unimplemented!` are explicit; they are still panics. Do not leave them on production paths.

## Typed library errors

Library errors SHOULD be focused, stable, and domain-specific. Do not expose a third-party error type as the core public contract unless that coupling is intentional ([C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err)).

In strict or allocation-free code, error context MUST stay allocation-free (copyable enums, no `String` payload). See [allocation.md](allocation.md).

`thiserror` MAY be used for allocating or `std`-facing crates when compatible with MSRV and feature policy:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("database: {0}")]
    Db(#[from] DbError),
    #[error("invalid data: {0}")]
    InvalidData(String),
}
```

Layered systems: nested enums with `#[from]`. A module with a single failure mode MAY use a struct error instead of a one-variant enum.

`anyhow` is for binary / application boundaries, not core library APIs or strict heapless code. `anyhow::Result` erases context callers need. Keep `context` strings from drifting; prefer a typed error with one `Display`.

Async: errors crossing `.await` or tasks MUST be `Send + Sync + 'static` where the runtime requires it. Avoid `Box<dyn std::error::Error>` in libraries unless required.

## `?` without hiding policy

Prefer `?` for straightforward propagation. Do not use it to hide translation, retry, rollback, or cleanup:

```rust
let header = parse_header(input)?;
let body = parse_body(input, header.body_range).map_err(ParsePacketError::Body)?;
```

## Checked arithmetic at trust boundaries

Offsets, lengths, capacities, and serialized values MUST use checked arithmetic before indexing or allocation decisions.

```rust
let end = start.checked_add(length).ok_or(ParseError::IntegerOverflow)?;
if end > input.len() {
    return Err(ParseError::OffsetOutOfBounds {
        offset: end,
        input_len: input.len(),
    });
}
debug_assert!(start <= end);
debug_assert!(end <= input.len());
```

Document overflow semantics for domain arithmetic: checked and erroring, saturating, wrapping, or explicitly proven impossible. Do not rely on debug-only overflow as the release contract.

## `debug_assert!` for internal invariants

Use `debug_assert!`, `debug_assert_eq!`, `debug_assert_ne!` when **all** of these hold:

1. The condition is established by types or release-active control flow.
2. It is a programming invariant, not caller validation.
3. Release correctness does not depend on the assertion executing.
4. Removing it cannot introduce undefined behavior.
5. Evaluating it has no required side effects.
6. The condition and message do not intentionally allocate.

Good: `debug_assert!(cursor <= input.len())` after a checked bounds calculation.

Use after important state transitions, checked bounds, fixed-capacity writes, parser cursor movement, and canonicalization, when they detect real defects. Do not add assertions mechanically.

Docs: [`debug_assert!`](https://doc.rust-lang.org/std/macro.debug_assert.html).

### External input: all builds

Incorrect:

```rust
pub fn read_byte(input: &[u8], index: usize) -> u8 {
    debug_assert!(index < input.len());
    input[index]
}
```

Correct: `get` + typed error, then `debug_assert!` on the established invariant.

### NEVER as an `unsafe` precondition

```rust
// Incorrect: the proof disappears in optimized builds.
debug_assert!(index < values.len());
let value = unsafe { *values.get_unchecked(index) };
```

Unsafe MUST rely on types, release-active validation, or a documented invariant proven independently of debug assertions. A debug assertion MAY duplicate a valid proof; it cannot be the proof.

### No required side effects

Incorrect: `debug_assert!(advance_cursor(&mut cursor));`

Correct: run the update, then `debug_assert!(advanced);`

The program MUST behave correctly with assertions on or off.

### Allocation-aware diagnostics

Prefer simple conditions and static messages. Avoid `input.to_vec()` or `state.to_string()` inside the assertion. `debug_assert_eq!` formats `Debug` on failure; custom `Debug` MUST NOT allocate on strict paths.

### Invariant panic is not a recoverable path

Panic hooks, backtraces, and diagnostics MAY allocate. An allocation-free guarantee covers success, documented recoverable errors, capacity exhaustion, and malformed input. An internal invariant panic is a defect path.

### Test optimized code with assertions on

```toml
[profile.release-assertions]
inherits = "release"
debug-assertions = true
overflow-checks = true
```

```bash
cargo test --workspace --profile release-assertions
```

CI: [quality.md](quality.md).

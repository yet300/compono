# Ownership, borrowing, and values

## Borrow by default

Take borrowed inputs unless ownership transfer is required.

```rust
fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0u32, |sum, byte| sum.wrapping_add(u32::from(*byte)))
}
```

Use: `&str` not `&String`; `&[T]` not `&Vec<T>`; `&Path` not `&PathBuf`; borrowed domain views rather than cloned domain objects.

A clone is appropriate when independent ownership is semantically required. Make the cost visible and intentional. NEVER clone only to satisfy the borrow checker before the ownership model is understood.

### When clone is justified

- The caller needs an independent snapshot while the original mutates.
- `Arc` / `Rc` (clone is refcount, the value stays heap-backed; not strict heapless).
- The underlying API requires owned data.
- Caching a result that must outlive the borrow.
- Avoiding a massive refactor on a non-hot path, with the cost documented.

### Clone traps

- Auto-cloning inside loops (`.map(|x| x.clone())`). Prefer `.cloned()` / `.copied()` at the iterator edge, or do not clone.
- Cloning large `Vec<T>` or `HashMap<K, V>`.
- Cloning because the API took the wrong ownership (fix the signature: if the callee needs ownership, take `T`, not `&T` then `.clone()`).
- Cloning a `Copy` type (use assignment; Clippy `clone_on_copy`).

Leave a necessary clone as late as possible.

## Pass small `Copy` values by value

Pass scalars, compact newtypes, and small `Copy` structs by value when that is clearer. Do not apply a universal byte threshold; measure ABI and target when size is in question.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Range32 {
    pub start: u32,
    pub end: u32,
}

fn contains(range: Range32, value: u32) -> bool {
    value >= range.start && value < range.end
}
```

Derive `Copy` when every field is `Copy`, the type is a plain data object with no heap ownership, and it does **not** implement `Iterator`.

NEVER put `Copy` and `Iterator` on the same type. Copying an iterator and advancing one copy leaves the other untouched and silently yields wrong results. If a `Copy` type must be iterable, implement `IntoIterator` and return a separate iterator struct (`core::range` does this).

Large structs and non-`Copy` values SHOULD be borrowed unless the function consumes them.

Rust arrays are stack-allocated. A large `Copy` array copied by value can overflow the stack. Box large buffers, or take a slice. See [allocation.md](allocation.md).

Enums sized to their largest variant: a huge payload makes every variant expensive to move. Consider boxing that variant on allocation-conscious code; on heapless code, redesign the layout.

## Ownership transitions in names

Follow [C-CONV](https://rust-lang.github.io/api-guidelines/naming.html#c-conv):

| Prefix | Meaning |
|--------|---------|
| `into_*` | consumes `self` |
| `to_*` | usually creates an owned value; MAY allocate |
| `as_*` | borrowed or inexpensive view |

Document allocation when a conversion creates owned storage.

## Indexes over pointer graphs

Prefer contiguous data and index-based relationships over linked object graphs. Indexes and offsets are easier to serialize, validate, borrow, cache, bound, move across FFI, and execute without allocation.

Use pointer-rich structures only when semantics and a measured workload justify the complexity.

## `Cow`

`Cow<'_, T>` is valid when the API honestly may borrow or own. It is not a heapless escape hatch. `Cow::into_owned()` and APIs that silently transition borrowed to owned are forbidden on strict and allocation-free paths.

## Early allocation in combinators

`or`, `map_or`, `unwrap_or`, `ok_or` evaluate the fallback immediately. If the fallback allocates or does work, use the `_else` form:

```rust
// Fallback is a cheap error value: ok_or is fine.
x.ok_or(ParseError::ValueAbsent)

// Fallback allocates: ok_or_else.
x.ok_or_else(|| ParseError::ValueAbsent)
```

# Pointers, sharing, concurrency, and unsafe

Prefer references for borrowing and slices for contiguous data. Reach for smart pointers only when their ownership semantics are required.

| Type | Owns heap? | `Send` + `Sync` | Use |
|------|------------|-----------------|-----|
| `&T` | no | if `T: Sync` | shared borrow |
| `&mut T` | no | if `T: Send` (not shared) | exclusive borrow |
| `Box<T>` | yes | if `T: Send` / `Sync` | single-owner heap, recursive types, large values |
| `Rc<T>` | yes | no | single-thread shared ownership |
| `Arc<T>` | yes | if `T: Send + Sync` | multi-thread shared ownership |
| `Cell<T>` | no (wrapper) | not `Sync` | `Copy` interior mutability |
| `RefCell<T>` | no (wrapper) | not `Sync` | runtime borrow checks; MAY panic |
| `Mutex<T>` | no (wrapper) | if `T: Send` | exclusive interior mutability, threads |
| `RwLock<T>` | no (wrapper) | if `T: Send + Sync` | many readers or one writer |
| `OnceCell<T>` | no (wrapper) | not `Sync` | one-time init, one thread |
| `LazyCell<T>` | no (wrapper) | not `Sync` | lazy `OnceCell` |
| `OnceLock<T>` | no (wrapper) | yes | one-time init, threads / `static` |
| `LazyLock<T>` | no (wrapper) | yes | lazy `OnceLock` |
| `*const T` / `*mut T` | n/a | manual | FFI / raw memory; `unsafe` |

Cloning `Rc`/`Arc` may not allocate; the value remains heap-backed and is not strict heapless. Interior mutability changes aliasing and synchronization reasoning.

Docs: [std pointers](https://doc.rust-lang.org/std/), [Nomicon](https://doc.rust-lang.org/nomicon/).

## Shared ownership is not the default

Frequent `Arc<Mutex<T>>` often means unclear ownership. Prefer: a single explicit owner; message passing with bounded queues; immutable shared data; scoped threads borrowing state; partitioned state; IDs or handles into a controlled store.

Use locks when they are the clearest correct design. Lock-free is not automatically faster or safer.

`RefCell` borrow conflicts panic at runtime. Do not use it to hide an ownership problem on a library hot path.

## `Send` and `Sync` are commitments

Do not add unsafe `Send` or `Sync` implementations without a written proof covering all interior state, aliases, callbacks, and FFI. [C-SEND-SYNC](https://rust-lang.github.io/api-guidelines/interoperability.html#c-send-sync): implement them where they are actually true.

## Forbid unsafe by default

```rust
#![forbid(unsafe_code)]
```

in crates that do not require unsafe Rust.

When unsafe is necessary:

- isolate it in the smallest possible module
- expose a safe API
- document every unsafe block with `SAFETY:` covering pointer, alignment, initialization, aliasing, lifetime, and concurrency invariants
- enforce preconditions in release-active code or types
- add focused tests and [Miri](https://github.com/rust-lang/miri) coverage where applicable
- retain a safe reference implementation when optimizing
- NEVER rely solely on `debug_assert!` for safety ([errors.md](errors.md))

## FFI is a complete boundary

FFI docs MUST define: ownership transfer; who allocates and deallocates; allocator compatibility; pointer validity and alignment; buffer length and capacity; lifetime; thread affinity; panic behavior; error representation; callback reentrancy.

Rust panics MUST NOT unwind across an FFI boundary unless the ABI explicitly supports and documents it.

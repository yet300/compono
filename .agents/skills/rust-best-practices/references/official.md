# Official Rust documentation

Cite the live page. Fetch it when the task needs current syntax, lint names, or checklist text. Do not invent APIs, lint ids, or rustdoc section names from memory.

## Open first

| Need | Page |
|------|------|
| Formatting, imports, naming in source | [Style Guide](https://doc.rust-lang.org/stable/style-guide/) |
| Public crate API shape | [API Guidelines](https://rust-lang.github.io/api-guidelines/) and the [checklist](https://rust-lang.github.io/api-guidelines/checklist.html) |
| Language and std | [The Book](https://doc.rust-lang.org/book/), [Reference](https://doc.rust-lang.org/reference/), [Rust by Example](https://doc.rust-lang.org/rust-by-example/), [std](https://doc.rust-lang.org/std/) |
| Editions | [Edition Guide](https://doc.rust-lang.org/edition-guide/) |
| Unsafe | [Nomicon](https://doc.rust-lang.org/nomicon/), [Reference: unsafety](https://doc.rust-lang.org/reference/unsafety.html) |
| Tooling | [Cargo book](https://doc.rust-lang.org/cargo/), [rustdoc](https://doc.rust-lang.org/rustdoc/), [Clippy](https://doc.rust-lang.org/clippy/), [rustfmt](https://rust-lang.github.io/rustfmt/) |
| `no_std` / heap | [`#![no_std]`](https://doc.rust-lang.org/stable/std/attribute.no_std.html), [`core`](https://doc.rust-lang.org/stable/core/), [`alloc`](https://doc.rust-lang.org/stable/alloc/) |
| Asserts | [`debug_assert!`](https://doc.rust-lang.org/std/macro.debug_assert.html), [`assert!`](https://doc.rust-lang.org/std/macro.assert.html) |
| Formatting without `String` | [`format_args!`](https://doc.rust-lang.org/std/macro.format_args.html) |
| In-place slice algorithms | [`slice`](https://doc.rust-lang.org/stable/core/primitive.slice.html) (`sort_unstable*`, `select_nth_unstable*`) |
| Profiles | [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) |
| Clippy config | [Clippy configuration](https://doc.rust-lang.org/nightly/clippy/configuration.html), [lint configuration](https://doc.rust-lang.org/nightly/clippy/lint_configuration.html), [lint index](https://rust-lang.github.io/rust-clippy/master/index.html) |
| rustdoc lints | [rustdoc lints](https://doc.rust-lang.org/rustdoc/lints.html) |

Language site: <https://www.rust-lang.org/>. Logo and trademark: [rust-artwork](https://github.com/rust-lang/rust-artwork), [trademark policy](https://rustfoundation.org/policy/rust-trademark-policy/).

## API Guidelines checklist

Treat as the default for public crates. Quote the `C-*` id in review. Full text: [checklist](https://rust-lang.github.io/api-guidelines/checklist.html).

| Area | Ids |
|------|-----|
| Naming | [C-CASE](https://rust-lang.github.io/api-guidelines/naming.html#c-case), [C-CONV](https://rust-lang.github.io/api-guidelines/naming.html#c-conv), [C-GETTER](https://rust-lang.github.io/api-guidelines/naming.html#c-getter), [C-ITER](https://rust-lang.github.io/api-guidelines/naming.html#c-iter), [C-ITER-TY](https://rust-lang.github.io/api-guidelines/naming.html#c-iter-ty), [C-FEATURE](https://rust-lang.github.io/api-guidelines/naming.html#c-feature), [C-WORD-ORDER](https://rust-lang.github.io/api-guidelines/naming.html#c-word-order) |
| Interoperability | [C-COMMON-TRAITS](https://rust-lang.github.io/api-guidelines/interoperability.html#c-common-traits), [C-CONV-TRAITS](https://rust-lang.github.io/api-guidelines/interoperability.html#c-conv-traits), [C-COLLECT](https://rust-lang.github.io/api-guidelines/interoperability.html#c-collect), [C-SERDE](https://rust-lang.github.io/api-guidelines/interoperability.html#c-serde), [C-SEND-SYNC](https://rust-lang.github.io/api-guidelines/interoperability.html#c-send-sync), [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err), [C-NUM-FMT](https://rust-lang.github.io/api-guidelines/interoperability.html#c-num-fmt), [C-RW-VALUE](https://rust-lang.github.io/api-guidelines/interoperability.html#c-rw-value) |
| Macros | [C-EVOCATIVE](https://rust-lang.github.io/api-guidelines/macros.html#c-evocative), [C-MACRO-ATTR](https://rust-lang.github.io/api-guidelines/macros.html#c-macro-attr), [C-ANYWHERE](https://rust-lang.github.io/api-guidelines/macros.html#c-anywhere), [C-MACRO-VIS](https://rust-lang.github.io/api-guidelines/macros.html#c-macro-vis), [C-MACRO-TY](https://rust-lang.github.io/api-guidelines/macros.html#c-macro-ty) |
| Documentation | [C-CRATE-DOC](https://rust-lang.github.io/api-guidelines/documentation.html#c-crate-doc), [C-EXAMPLE](https://rust-lang.github.io/api-guidelines/documentation.html#c-example), [C-QUESTION-MARK](https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark), [C-FAILURE](https://rust-lang.github.io/api-guidelines/documentation.html#c-failure), [C-LINK](https://rust-lang.github.io/api-guidelines/documentation.html#c-link), [C-METADATA](https://rust-lang.github.io/api-guidelines/documentation.html#c-metadata), [C-RELNOTES](https://rust-lang.github.io/api-guidelines/documentation.html#c-relnotes), [C-HIDDEN](https://rust-lang.github.io/api-guidelines/documentation.html#c-hidden) |
| Predictability | [C-SMART-PTR](https://rust-lang.github.io/api-guidelines/predictability.html#c-smart-ptr), [C-CONV-SPECIFIC](https://rust-lang.github.io/api-guidelines/predictability.html#c-conv-specific), [C-METHOD](https://rust-lang.github.io/api-guidelines/predictability.html#c-method), [C-NO-OUT](https://rust-lang.github.io/api-guidelines/predictability.html#c-no-out), [C-OVERLOAD](https://rust-lang.github.io/api-guidelines/predictability.html#c-overload), [C-DEREF](https://rust-lang.github.io/api-guidelines/predictability.html#c-deref), [C-CTOR](https://rust-lang.github.io/api-guidelines/predictability.html#c-ctor) |
| Flexibility | [C-INTERMEDIATE](https://rust-lang.github.io/api-guidelines/flexibility.html#c-intermediate), [C-CALLER-CONTROL](https://rust-lang.github.io/api-guidelines/flexibility.html#c-caller-control), [C-GENERIC](https://rust-lang.github.io/api-guidelines/flexibility.html#c-generic), [C-OBJECT](https://rust-lang.github.io/api-guidelines/flexibility.html#c-object) |
| Type safety | [C-NEWTYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype), [C-CUSTOM-TYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-custom-type), [C-BITFLAG](https://rust-lang.github.io/api-guidelines/type-safety.html#c-bitflag), [C-BUILDER](https://rust-lang.github.io/api-guidelines/type-safety.html#c-builder) |
| Dependability | [C-VALIDATE](https://rust-lang.github.io/api-guidelines/dependability.html#c-validate), [C-DTOR-FAIL](https://rust-lang.github.io/api-guidelines/dependability.html#c-dtor-fail), [C-DTOR-BLOCK](https://rust-lang.github.io/api-guidelines/dependability.html#c-dtor-block) |
| Debuggability | [C-DEBUG](https://rust-lang.github.io/api-guidelines/debuggability.html#c-debug), [C-DEBUG-NONEMPTY](https://rust-lang.github.io/api-guidelines/debuggability.html#c-debug-nonempty) |
| Future proofing | [C-SEALED](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-sealed), [C-STRUCT-PRIVATE](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-struct-private), [C-NEWTYPE-HIDE](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-newtype-hide), [C-STRUCT-BOUNDS](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-struct-bounds) |
| Necessities | [C-STABLE](https://rust-lang.github.io/api-guidelines/necessities.html#c-stable), [C-PERMISSIVE](https://rust-lang.github.io/api-guidelines/necessities.html#c-permissive) |

**C-NO-OUT and caller-owned buffers.** Return values from ordinary library APIs. When the memory profile is heapless or allocation-free, caller-owned output and scratch slices are the contract (C-CALLER-CONTROL), not a `Vec` return.

## Style Guide (source formatting)

[Style Guide](https://doc.rust-lang.org/stable/style-guide/) owns whitespace, wrapping, imports, and punctuation. `rustfmt` is the mechanical enforcer. Do not spend review time on formatting `rustfmt` already owns. Manual decisions stay on naming, module shape, visibility, API semantics, and control flow.

Import order when `rustfmt` is configured for it: `std`/`core`/`alloc`, then external crates, then workspace crates, then `super`/`crate`. Details: [quality.md](quality.md).

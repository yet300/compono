# Versions

## One version for the workspace

Every crate under `crates/` has the same version and is released together. A crate
depends on the others at exactly that release line, so use one version of all of them.

The Swift package in `native/apple/Compono` and the Compose library in
`native/android/compono` carry the same version as the crates they were tested with.

## Semantic versions

Compono follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

| Change | Version |
|---|---|
| Removing or renaming a public item, changing a signature, adding a required trait method, or changing documented behaviour | major |
| A new public item, a new provided trait method, a new variant of a `#[non_exhaustive]` enum, a new field of a `#[non_exhaustive]` struct | minor |
| A fix that makes behaviour match the documentation, a faster or smaller implementation, documentation | patch |

These are also part of the public interface, and changing them is a major change:

- **The format of saved state.** Bytes from `Tree::save` of version 1.x are read by
  every later 1.y, for the same schema number and the same component and route types.
  How a router numbers its entries is part of that format.
- **The order of lifecycle and Back.** Parents rise before children and fall after
  them, an old entry is lowered before a new one is raised, and Back goes to the
  deepest visible component, the most recently shown first.
- **The allocation budget** enforced by `crates/compono/tests/allocations.rs`.
- **The numbers that cross a language boundary:** `NodeId::to_bits` and `phase_code`.
- **Cargo features.** A feature is never removed and never starts to remove items.

These are not part of the public interface:

- Anything under `examples/`, `benchmarks/` and `native/showcase/`.
- The text of `Debug` and `Display` output.
- Exact timings.
- The state that `History` stores in a browser history entry. An entry written by
  another version is treated as an entry the application does not know.

A release candidate, such as `1.0.0-rc.1`, can still change in any way before `1.0.0`.

## UI toolkits

`compono-egui`, `compono-iced` and `compono-gpui` name types of their toolkit in their
own public interface. A toolkit release that breaks those types forces a major
version of that adapter. The whole workspace then takes the major version, even
though the core does not change.

## Minimum Rust version

The minimum supported Rust version is **1.96**, set as `rust-version` in the
workspace manifest and checked by CI on that toolchain.

- Raising it is a minor change, never a patch.
- It is raised only to a version that has been stable for at least six months.
- The changelog names the new minimum in the entry of the release that raises it.

## Deprecation

An item is marked `#[deprecated]` for at least one minor release before a major
release removes it. The note names the replacement.

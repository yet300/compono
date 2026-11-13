---
name: compono-verify
description: Runs the checks a Compono change needs and reports their results. Use before saying a change is done, after editing any crate, example, native package or doc, or when a check hangs or fails in a way that looks environmental.
---

# Verifying a Compono change

Pick the rows that match what changed, run every command in them, and report
counts read from the output. The `verification-before-completion` skill says why
evidence comes first.

## Running commands

- Put a time limit on every long command: `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`.
  A hung build is reported as a hang, not waited on.
- Start commands that take minutes in the background and wait for their exit.
  Never loop on `sleep` to poll, and never pipe into a reader that waits for input.
- zsh does not split `$var` into arguments. Write each feature variant out in full.

## What to run

| Changed | Commands |
|---|---|
| Any Rust | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
| `crates/compono` or its tests | also the three `--no-default-features` clippy runs, `cargo test -p compono --no-default-features` and the `thumbv7em-none-eabihf` check from `AGENTS.md`. A test that uses `pump`, `perform` or saved state needs a `cfg` for `std` or `serde` |
| `crates/compono/src/concurrency` | also the loom command from `AGENTS.md` |
| Tree bookkeeping or a router's cost | also the `navigation_cost`, `set_scaling` and `router_scaling` examples in release, and confirm the numbers stay flat as sizes grow |
| `compono-web`, `showcase-web`, or anything they depend on | `cargo clippy -p compono-web -p showcase-web --target wasm32-unknown-unknown -- -D warnings` |
| Web behaviour | build with `examples/showcase-web/build.sh`, serve `dist` with `python3 -m http.server`, and use the page in a browser |
| Public docs | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and for `compono-web` the same with `--target wasm32-unknown-unknown` |
| `compono-ffi` or `native/` | the commands in the `compono-native` skill |

`AGENTS.md` holds the exact command list. This table only says when each applies.

## Known traps

- `showcase-gpui` needs Rust 1.99 or later, because `gpui-pre-macos` does. It
  enables `runtime_shaders` on `gpui-pre-platform`, so it builds without Xcode's
  Metal toolchain. `compono-gpui` itself opens no window and needs neither.
- Items that exist only on `wasm32` cannot be linked from docs built natively.
  Name them in backticks instead.
- `compono-desktop` is also built for `wasm32`, where `std::fs::File` has no
  `Drop`. Close files by scope, not with `drop(file)`.
- The web build needs `wasm-bindgen-cli` at the exact `wasm-bindgen` version in
  `Cargo.lock`. Point `WASM_BINDGEN` at it when it is not on `PATH`.
- `native/showcase/ffi` and the projects under `benchmarks/` are separate
  workspaces. Workspace commands do not reach them.

## Reporting

State what ran, the pass and fail counts, and anything not run with the reason.
A step skipped because a tool is missing is reported as skipped, never as passed.

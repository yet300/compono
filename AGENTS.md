# Working on Compono

Read `ARCHITECTURE.md` first. It describes the current design.

## Skills

Skills live in `.agents/skills`, with a copy in `.claude/skills`. Load the one
that matches the task before starting it:

| Task | Skill |
|---|---|
| Finishing any change | `compono-verify`, then `verification-before-completion` |
| A component, router, saved state or showcase screen | `compono-component` |
| `compono-ffi`, `native/`, a simulator or an emulator | `compono-native` |
| Writing or reviewing Rust | `rust-best-practices` |
| A feature or a bug fix | `test-driven-development` |
| A failure whose cause is not yet known | `systematic-debugging` |
| Atomics, a reconciling algorithm, or a promised type size | `rust-proof-techniques` |
| Editing this file, `CLAUDE.md` or a skill | `writing-for-agents` |

Edit skills in `.agents/skills` and run `scripts/sync-skills.sh`. Where a vendored
skill disagrees with this file, this file wins. Known differences from
`rust-best-practices`: errors are small hand-written types implementing `Display`
and `Error`, without `thiserror`; `unsafe_code` is denied rather than forbidden, so a
platform crate can allow it locally with a `SAFETY:` comment; the core's memory
profile is allocation-conscious, with budgets enforced by
`crates/compono/tests/allocations.rs`.

## Layout

- `crates/compono/src/component` holds the `Component` trait, `Cx`, `Handle` and `Phase`.
- `crates/compono/src/tree` is the single owner of components and its bookkeeping.
- `crates/compono/src/router` holds `Stack`, `Slot`, `Pages`, `Panels`, `Items` and `ChildSet`. They use only public `Cx` methods.
- `crates/compono/src/concurrency` holds `Sender`, tasks and `WakeSignal`.
- `crates/compono-ffi` is the Rust side of native UIs. It depends on no binding generator.
- `crates/compono-test` has the recording host, virtual time and `check_host`.
- `crates/compono-desktop` and `crates/compono-web` are platform crates. `compono-web/src/history.rs` is plain logic tested natively; `compono-web/src/page` is built only for `wasm32`.
- `examples/showcase/src/feature` shows how to structure components.
- `examples/showcase-egui`, `examples/showcase-iced` and `examples/showcase-gpui` draw those components. None may add logic that the others would need. `examples/showcase-web` runs the egui one in a browser.
- `skills/` holds skills for applications that use Compono. They are not loaded here; `.agents/skills` is for working on Compono.

## Before finishing a change

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p compono --no-default-features -- -D warnings
cargo clippy -p compono --no-default-features --features std -- -D warnings
cargo clippy -p compono --no-default-features --features serde -- -D warnings
cargo test --workspace
cargo test -p compono --no-default-features
cargo check -p compono --no-default-features --target thumbv7em-none-eabihf
cargo clippy -p compono-web -p showcase-web --target wasm32-unknown-unknown -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

When changing `crates/compono/src/concurrency`, also model-check it. The separate
target directory keeps the `--cfg loom` build from rebuilding everything else:

```bash
CARGO_TARGET_DIR=target/loom RUSTFLAGS="--cfg loom" cargo test -p compono --test loom --release
```

When changing tree bookkeeping, also run these and check the numbers stay flat as
sizes grow:

```bash
CARGO_PROFILE_RELEASE_PANIC=abort cargo run -p compono --release --example navigation_cost
```

```bash
CARGO_PROFILE_RELEASE_PANIC=abort cargo run -p compono --release --example set_scaling
```

```bash
CARGO_PROFILE_RELEASE_PANIC=abort cargo run -p compono --release --example router_scaling
```

## Working here

- Give every long command a time limit, such as `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`,
  and run builds that take minutes in the background. A hang is reported, not waited out.
- Commit, push or publish only when asked.
- Ask before installing anything on a physical device.
- The project is licensed under Apache-2.0, in `LICENSE`. Code taken from elsewhere must
  have a compatible licence, recorded where it is used.

## Rules

- Library code has no `unwrap`, `expect`, `panic!`, indexing or `unsafe`. Lints enforce this.
- A child never receives a reference to its parent. Add an `Output` variant instead.
- Add a test that fails first when fixing a bug.
- A new router must take every id from `cx.router()`, so `cx.named_routers` and
  `cx.replace_router` work for it; start clean with `cx.forget_router`; keep only its
  named entries with `cx.keep_router_entries` when it restores; and implement
  `compono::router::Router`.
- A change to a public item follows `VERSIONING.md`, gets a line in `CHANGELOG.md`,
  and updates every example in `skills/` that uses it.
- Keep documentation in English and about the current state only.

# Compono

Compono is a component layer between business logic and any UI, written for Rust's
ownership model. It keeps a tree of components with a lifecycle, navigation, Back
handling and saved state, and it knows nothing about UI toolkits or platforms.

The idea comes from [Decompose](https://github.com/arkivanov/Decompose). The design
does not: one `Tree` owns every component, a child reports upward only through a typed
`Output`, and a running component cannot be reentered.

```rust
use compono::{Component, Cx};

struct Counter {
    value: u32,
}

enum Msg {
    Increment,
}

impl Component for Counter {
    type Msg = Msg;
    type Output = ();

    fn update(&mut self, msg: Msg, _cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Increment => self.value += 1,
        }
    }
}
```

## Status

Release candidate 1.0.0-rc.1, not published. `CHANGELOG.md` lists what it contains,
`VERSIONING.md` says what a version number promises, and the minimum Rust version is
1.96. The core and the egui, iced and GPUI adapters work on
desktop, where state is saved to a file and the system's Back shortcuts work. The egui
showcase also runs in a browser, with browser history and session storage. SwiftUI and
Compose work through `compono-ffi` on simulators. Real devices have not been tested.
`ARCHITECTURE.md` lists what has and has not been verified.

## Layout

| Path | Contents |
|---|---|
| `crates/compono` | The tree, components, routers, tasks and persistence |
| `crates/compono-test` | Recording host, virtual time, host conformance checks |
| `crates/compono-egui`, `-iced`, `-gpui` | UI adapters |
| `crates/compono-motion` | Router transitions: curves, springs, platform presets, layer values; `no_std`, no dependencies |
| `crates/compono-ffi` | The Rust side of SwiftUI and Compose: thread-bound tree, models |
| `crates/compono-tokio` | Task spawner on Tokio |
| `crates/compono-desktop` | Saved-state file and each system's Back shortcuts |
| `crates/compono-web` | Browser history for a stack, page lifecycle, session storage, tasks |
| `native/apple/Compono`, `native/android/compono` | SwiftUI and Compose packages |
| `native/showcase` | The showcase on SwiftUI and Compose |
| `benchmarks/` | FFI backends compared, and Compono against Decompose |
| `examples/showcase` | Every navigation model and a 100 000 row load test, with no UI |
| `examples/showcase-egui`, `-iced`, `-gpui` | The same screens on each UI |
| `examples/showcase-web` | The egui showcase in a browser |
| `skills/` | Agent skills for applications that use Compono |

## Running

```bash
cargo run -p showcase-iced
```

```bash
cargo run -p showcase-egui
```

```bash
cargo run -p showcase-gpui
```

The web build needs `wasm-bindgen-cli` at the version in `Cargo.lock`:

```bash
cargo install wasm-bindgen-cli --version 0.2.130 --locked
```

```bash
examples/showcase-web/build.sh && python3 -m http.server -d examples/showcase-web/dist 8080
```

Custom navigation is ordinary code on the public API. See "Custom navigation" in
`ARCHITECTURE.md` and `examples/showcase/src/feature/gallery/filmstrip.rs`.

See `ARCHITECTURE.md` for the design and `AGENTS.md` for the checks to run before
finishing a change.

## Using Compono with a coding agent

`skills/` holds agent skills for projects that use Compono: components,
navigation, Rust UIs, and SwiftUI and Compose. Copy them into your project's
`.agents/skills` or `.claude/skills`. `skills/README.md` lists them.

## Licence

Compono is licensed under the [Apache License, Version 2.0](LICENSE). The skills in
`.agents/skills` that come from other projects keep their own MIT licences, listed in
`.agents/skills/README.md`.

---
name: compono-native-apps
description: Ships Compono components to SwiftUI and Jetpack Compose through compono-ffi and BoltFFI. Use when exporting a Compono tree to iOS or Android, writing exported component classes, setting up ComponoRoot or retainedTree, drawing routers with ChildStack, ChildSlot, ChildPages, ChildPanels or ChildList, or handling Back gestures on mobile.
---

# Compono in SwiftUI and Compose

The components stay in Rust. A small FFI crate exports the tree and one class per
component with BoltFFI. The Swift package `Compono` (`native/apple/Compono`)
and the Compose library (`native/android/compono`) connect the generated bindings
to the app; neither depends on them.

```text
my-app-ffi/            Rust: BoltFFI exports (cdylib + staticlib)
  src/lib.rs           the app class with the standard methods, and the listener
  src/component.rs     ChildRef, Ctx: shared by every exported component
  src/feature/*.rs     one exported class per component
ios/ui/<feature>/      SwiftUI screens, each taking its component object
android/ui/<feature>/  Compose screens, each taking its component object
```

Use BoltFFI 0.32.1: `cargo install boltffi_cli --version 0.32.1 --locked`, then
`boltffi pack apple --release` and `boltffi pack android --release` in the FFI
crate. Android needs NDK 28.

## The app class

BoltFFI reads exports from the source files, so write the exports by hand; a
macro cannot generate them. Keep these exact Rust names: the Swift package and
the Compose library match them, so the generated class conforms with an empty
extension.

```rust,ignore
#[export]
pub trait Listener: Send + Sync {
    fn commit(&self, dirty: Vec<i64>, can_back: bool);
    fn schedule_pump(&self);
}

#[export]
pub struct MyApp {
    tree: Arc<NativeTree>,
    root: Option<Handle<Root>>,
}

#[export]
impl MyApp {
    pub fn new(listener: Arc<dyn Listener>, saved: Vec<u8>) -> Self { /* NativeTree::new or ::restore, mount_persistent */ }
    pub fn root(&self) -> RootComponent { /* the root's exported class */ }
    pub fn pump(&self) { checked(self.tree.pump()); }
    pub fn set_phase(&self, code: i32) { /* 0 hidden, 1 visible, 2 focused: phase_from_code */ }
    pub fn back(&self) -> bool { checked(self.tree.back()).unwrap_or(false) }
    pub fn back_start(&self) -> i64 { /* target address, or compono_ffi::NO_NODE */ }
    pub fn back_cancel(&self) { checked(self.tree.back_cancel()); }
    pub fn back_commit(&self) -> bool { checked(self.tree.back_commit()).unwrap_or(false) }
    pub fn save(&self) -> Vec<u8> { checked(self.tree.save(SCHEMA)).unwrap_or_default() }
    pub fn contains(&self, node: i64) -> bool { /* tree.contains(NodeId::from_bits(..)) */ }
}
```

Pass every `NativeTree` result through one helper. `NativeError::Gone` is normal
use, such as a tap on a component just removed. Every other error is a bug in the
native caller (`NativeError::is_misuse`), so a debug build stops at it:

```rust,ignore
#[track_caller]
fn checked<T>(result: Result<T, NativeError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            debug_assert!(!error.is_misuse(), "native call failed: {error}");
            None
        }
    }
}
```

`compono_ffi::NativeTree` wraps the tree for this boundary, and
`compono_ffi::Platform` is what it calls back: forward `commit` and
`schedule_pump` to the exported `Listener`.

## Exported components

As in Decompose, each screen takes its component object, never a node address.
An exported component holds the tree weakly and its own address:

```rust,ignore
#[export]
pub struct CountersComponent {
    pub(crate) ctx: Ctx,   // Weak<NativeTree>
    pub(crate) node: u64,
}

#[export]
impl CountersComponent {
    pub fn node(&self) -> i64 { self.node.cast_signed() }
    pub fn model(&self) -> Option<CountersModel> { /* ctx.read::<Counters, _>(node, |c| ...) */ }
    pub fn active(&self) -> Option<ChildRef> { /* key, node and phase of the top entry */ }
    pub fn counter(&self, child: ChildRef) -> CounterComponent { /* child's exported class */ }
    pub fn go_deep(&self) { self.ctx.send::<Counters>(self.node, CountersMsg::GoDeep) }
}
```

- `model()` returns a `#[data]` struct of what the screen draws, or `None` once
  the component is gone.
- A router's children cross as `ChildRef { key, node, phase }`. For a router whose
  children differ, wrap it with a kind enum, such as `ScreenChild { kind, child }`.
- The parent hands out its children's objects, such as `root.tabs(child)`. Each
  accessor checks the kind it is given, `debug_assert!(child.kind ==
  ScreenKind::Tabs)`, so a debug build stops when native code passes a child of
  another kind.
- An address that may be absent crosses as `compono_ffi::NO_NODE` (`-1`), which no
  component has.
- Addresses cross as `i64` and counts as `i32`.
- Name an action `dismiss`, not `close`: `close` clashes with Kotlin's
  `AutoCloseable`.

## Platforms

Read the reference for the platform you are writing:

- SwiftUI: [references/swiftui.md](references/swiftui.md)
- Compose: [references/compose.md](references/compose.md)

Both give the same router views: `ChildStack` (transitions and interactive Back),
`ChildSlot`, `ChildPages`, `ChildPanels` and `ChildList` (reports its window to an
`Items`). A screen subscribes to its component with `observe(component.node())`
and redraws when that component changes.

## Critical warnings

- Every `NativeTree` call happens on the thread that created it, the main thread.
  Other threads get `WrongThread`; a call made while another runs gets `Busy`.
  Through `checked`, both stop a debug build at the call.
- The FFI thread has no Tokio context. Create a Tokio timer inside the future:
  `Box::pin(async move { tokio::time::sleep(d).await })`.
- A `ULongArray` in a callback makes BoltFFI's Kotlin side fail in `JNI_OnLoad`.
  Use `Vec<i64>`.
- Exported Kotlin objects have no cleaner. Free them with `close()`, which
  `rememberComponent` does for you.

---
name: compono-components
description: Writes Compono components and the tree that owns them. Use when writing a type that implements compono::Component, using Cx, Handle, Phase, Persist, cx.perform or fn back, mounting a Tree, saving state across restarts, or testing components with compono-test.
---

# Compono components

Compono is a Rust take on Decompose and Essenty. A **component** is a plain struct
holding business logic and state. It has no UI code. A UI reads it through
`&Tree` and sends it messages.

| Decompose / Essenty | Compono |
|---|---|
| `ComponentContext` | `Cx<'_, Self>`, passed to every constructor and method |
| `Value<T>`, `MutableValue` | Plain public fields, read with `tree.get(handle)` |
| Callbacks passed to a child | The child's `Output` type, handled by the parent |
| `Lifecycle` (created, started, resumed) | `Phase`: `Hidden`, `Visible`, `Focused` |
| `StateKeeper` | `Persist` and `cx.restored()` |
| `InstanceKeeper`, `retainedComponent` | Not needed: the tree outlives the UI. On Android, `retainedTree` |
| `BackHandler` | `fn back(&self) -> Option<Self::Msg>` |
| `coroutineScope` | `cx.perform` and `cx.perform_local`, cancelled with the component |

## Component structure

```rust
use compono::{Component, Cx, Handle, Phase};

/// Input from the UI, the parent and finished tasks.
#[derive(Clone, Debug)]
pub enum ProfileMsg {
    Rename(String),
    Save,
    Saved,
}

/// What the parent hears. Replaces callbacks.
#[derive(Clone, Debug)]
pub enum ProfileOutput {
    Close,
}

#[derive(Debug)]
pub struct Profile {
    pub name: String,
    pub saving: bool,
}

impl Profile {
    pub fn new(_cx: &mut Cx<'_, Self>, name: String) -> Self {
        Self { name, saving: false }
    }
}

impl Component for Profile {
    type Msg = ProfileMsg;
    type Output = ProfileOutput;

    fn update(&mut self, msg: ProfileMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ProfileMsg::Rename(name) => self.name = name,
            ProfileMsg::Save => self.saving = true,
            ProfileMsg::Saved => {
                self.saving = false;
                cx.emit(ProfileOutput::Close);
            }
        }
    }

    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        // Nothing the UI draws changed, so no redraw is needed.
        cx.unchanged();
        let _focused = phase == Phase::Focused;
    }
}

/// A parent creates the child and handles its output in a method.
#[derive(Debug)]
pub struct Account {
    pub profile: Handle<Profile>,
    pub open: bool,
}

impl Account {
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        let profile = cx.child(|cx| Profile::new(cx, "Ada".to_owned()), Self::on_profile);
        Self { profile, open: true }
    }

    fn on_profile(&mut self, output: ProfileOutput, _cx: &mut Cx<'_, Self>) {
        match output {
            ProfileOutput::Close => self.open = false,
        }
    }
}

impl Component for Account {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}
```

Rules:

- Dependencies (repositories, clocks, API clients) are constructor arguments. The
  parent decides what to pass.
- A child talks upward only with `cx.emit(output)`. It never holds its parent's
  handle. When it needs something from above, add an `Output` variant.
- Public fields are the model the UI draws. Keep labels, button lists and
  dialog text in the component, so every UI shares them.
- `cx.send(handle, msg)` reaches the component itself and its own children, and
  returns `Err(NotOwnChild)` for any other handle, a grandchild included. Message a
  grandchild through its parent. `emit` and `send` are queued and run after the
  current method returns.
- Call `cx.unchanged()` when a message changed nothing the UI shows. The UI then
  skips the redraw.

## Lifecycle

The tree grants each component a `Phase`. A child's phase never exceeds its
parent's. Raising goes parent first, lowering goes children first.

| `Phase` | Meaning | Android | iOS |
|---|---|---|---|
| `Hidden` | Alive, not on screen | `onStop` | background |
| `Visible` | On screen, not the input target | `onStart`, `onPause` | inactive |
| `Focused` | On screen and receiving input | `onResume` | active |

`phase_changed` runs once per change. Read the current phase with `cx.phase()`.
A plain child follows its parent. Children of a router get the phase the router
grants: see the `compono-navigation` skill. `cx.set_child_phase(child, phase)`
sets the phase a parent grants a plain child; the child still never exceeds its
parent.

A component lives until its parent destroys it (`cx.destroy(child)`), its router
removes it, or the tree is dropped. A child never outlives its parent. Both calls
return `Err(NotOwnChild)` and do nothing for anything but the component's own live
children, as `cx.send` does.

## Async work

```rust
use compono::{Component, Cx, Phase, TaskId};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// A clock injected by the parent, so tests can use virtual time.
pub type Sleep = Arc<dyn Fn(Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct Ticker {
    pub ticks: u64,
    timer: Option<TaskId>,
    sleep: Sleep,
}

impl Ticker {
    pub fn new(_cx: &mut Cx<'_, Self>, sleep: Sleep) -> Self {
        Self { ticks: 0, timer: None, sleep }
    }

    fn arm(&mut self, cx: &mut Cx<'_, Self>) {
        let wait = (self.sleep)(Duration::from_secs(1));
        // `Err(NoSpawner)` when the tree has no executor: nothing runs.
        self.timer = cx.perform(wait, |()| ()).ok();
    }
}

impl Component for Ticker {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.ticks += 1;
        self.arm(cx);
    }

    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        cx.unchanged();
        if phase == Phase::Focused {
            self.arm(cx);
        } else if let Some(task) = self.timer.take() {
            cx.cancel(task);
        }
    }
}
```

- `cx.perform(future, to_msg)` runs a `Send` future on the tree's `Spawner` and
  delivers its result as a message. Set the spawner once:
  `tree.set_spawner(compono_tokio::TokioSpawner::new(handle))`, or
  `compono_gpui::GpuiSpawner` in GPUI. In a build that unwinds, a `perform` task
  that panics delivers no message and its guard is released at the next pump.
- `cx.perform_local(future, to_msg)` runs a future that is not `Send`, such as a
  browser `fetch`, on the tree's thread. It needs no spawner.
- Both are cancelled when the component is destroyed, and a late result is
  dropped. `cx.cancel(task)` stops one earlier.
- Another thread reaches a component through a `Sender` from `cx.sender()`. Its
  `send` wakes the host, which calls `tree.pump()` on the tree's thread.

## Saving state

State that must survive process death implements `Persist`. Its `Saved` type is
serde data, written with postcard.

```rust
use compono::{Component, Cx, Persist};

pub struct Draft {
    pub text: String,
}

impl Draft {
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        Self { text: cx.restored().unwrap_or_default() }
    }
}

impl Component for Draft {
    type Msg = String;
    type Output = ();
    fn update(&mut self, text: String, _cx: &mut Cx<'_, Self>) {
        self.text = text;
    }
}

impl Persist for Draft {
    type Saved = String;
    const KIND: &'static str = "notes.draft";
    fn save(&self) -> String {
        self.text.clone()
    }
}
```

- Create a saved child with `cx.persistent_child_keyed(name, init, on_output)` and
  the root with `tree.mount_persistent(init)`. A plain `cx.child` is not saved.
- Name saved children outside router entries:
  `cx.persistent_child_keyed("draft", init, on_output)`. The name finds the saved
  state whatever else the parent creates. A child without a name is identified by
  the order in which its parent created children, counting every child, so adding
  a child before it in a later version hands it another child's state. Names are
  unique among one parent's children; a repeated name saves nothing, and the host
  receives `Diagnostic::RepeatedChildName`.
- A plain child that has saved descendants needs a name too: `cx.child_keyed`.
- Give every `Persist` type a `KIND`, a name unique in the app and the same in
  every version. Saved state records it, and `cx.restored()` gives `None` for
  state another kind saved at the same place, or for bytes left over after the
  value.
- When one component's `Saved` changes shape, raise its `VERSION` and read the
  older shapes in `fn migrate(version, bytes) -> Option<Self::Saved>`, where
  `bytes` are the postcard encoding of that version's `Saved`. Without `migrate`
  the component starts clean.
- Save with `tree.save(SCHEMA)` and restore with `Tree::with_saved(host, &bytes,
  SCHEMA)`. Raise `SCHEMA` to discard all saved state at once: older bytes then
  give `RestoreOutcome::SchemaChanged` and the app starts clean.
- Routers save their navigation through their own `snapshot` and `restore_or`, in
  the owner's `Saved` type. See the `compono-navigation` skill.

## Back

`fn back(&self) -> Option<Self::Msg>` returns the message that handles Back, or
`None` to let an ancestor handle it. Compute it from the component's own fields:
the tree caches the answer after each change. `tree.back()` runs it on the
deepest visible component that handles Back, then its ancestors. Platform hosts
call `tree.back()` for the system Back button, gesture and shortcut.

```rust
use compono::router::stack::Stack;
use compono::{Component, Cx};

pub struct Screens {
    pub stack: Stack<u32, (), Self>,
}

impl Component for Screens {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        let _at_root = self.stack.pop(cx);
    }

    fn back(&self) -> Option<()> {
        self.stack.can_pop().then_some(())
    }
}
```

## The tree

```rust
use compono::{Commit, Component, Cx, Host, Phase, Tree};

/// A host is told once per change. UI adapters provide one.
struct Quiet;
impl Host for Quiet {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

struct Counter(u32);
impl Component for Counter {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, by: u32, _cx: &mut Cx<'_, Self>) {
        self.0 += by;
    }
}

fn run() -> Result<(), compono::StaleHandle> {
    let mut tree = Tree::new(Quiet);
    let counter = tree.mount(|_cx| Counter(0));
    tree.set_phase(Phase::Focused);
    tree.send(counter, 2)?;
    assert_eq!(tree.get(counter).map(|counter| counter.0), Some(2));
    Ok(())
}
```

- One `Tree` per window or app. It lives on the thread that created it.
- `tree.send` runs the message and everything it queued before it returns.
- `tree.get(handle)` returns `None` for a removed component. A `Handle` never
  dangles.
- `tree.set_phase` follows the window or app: hidden, visible or focused.
- Log `Host::diagnostic` in debug builds. It reports a child or router that works
  but saves nothing, such as a repeated child name:

  ```rust,ignore
  fn diagnostic(&mut self, diagnostic: &compono::Diagnostic<'_>) {
      #[cfg(debug_assertions)]
      eprintln!("compono: {diagnostic:?}");
  }
  ```

## Testing without a UI

Add `compono-test` as a dev-dependency.

```rust
use compono::{Component, Cx, Phase, Tree};
use compono_test::{VirtualTime, headless};
use std::time::Duration;

struct Clock {
    seconds: u32,
    time: VirtualTime,
}

impl Component for Clock {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.seconds += 1;
        let _task = cx.perform(self.time.sleep(Duration::from_secs(1)), |()| ());
    }
    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        if phase == Phase::Focused {
            let _task = cx.perform(self.time.sleep(Duration::from_secs(1)), |()| ());
        }
    }
}

#[test]
fn ticks_once_a_second_while_focused() {
    let (host, log) = headless();
    let time = VirtualTime::new();
    let mut tree = Tree::new(host);
    tree.set_spawner(time.clone());
    let clock_time = time.clone();
    let clock = tree.mount(move |_cx| Clock { seconds: 0, time: clock_time });
    tree.set_phase(Phase::Focused);
    time.advance(&mut tree, Duration::from_secs(3)).unwrap();
    assert_eq!(tree.get(clock).map(|clock| clock.seconds), Some(3));
    assert!(!log.is_empty(), "every tick was a commit");
}
```

- `headless()` gives a host and a `CommitLog` of what changed.
- `VirtualTime` is both the spawner and the clock. `settle` runs ready work and
  `advance` moves time forward.
- `compono_test::check_host(...)` checks a custom `Host` against the contract.

## Critical warnings

- A child that stores its parent's `Handle` breaks the model. Use `Output`.
- `fn back` that reads anything but `&self` gives stale answers. The tree caches it.
- An unnamed child created conditionally or in a varying order shifts the saved
  state of every unnamed child after it. Name children with saved state, or
  create them inside router entries.
- A `Spawner` is required for `cx.perform`. Without one it returns `Err(NoSpawner)`.
- Every `Tree` call happens on the tree's thread. Other threads use `Sender`.

## Quick reference

| Need | API |
|---|---|
| Child component | `cx.child(init, Self::on_output)` |
| Saved child | `cx.persistent_child_keyed("name", init, Self::on_output)` |
| Tell the parent | `cx.emit(output)` |
| Message another component | `cx.send(handle, msg)`, `tree.send(handle, msg)` |
| Read state | `tree.get(handle)` |
| React to visibility | `fn phase_changed`, `cx.phase()` |
| Background work | `cx.perform`, `cx.perform_local`, `cx.cancel` |
| Back | `fn back`, `tree.back()` |
| Saved state | `Persist`, `cx.restored()`, `tree.save`, `Tree::with_saved` |
| Skip a redraw | `cx.unchanged()` |
| Remove a child | `cx.destroy(handle)`, which returns `Result` |

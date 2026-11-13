# Custom navigation models

The built-in routers use only public `Cx` methods, so a custom one has the same
power. Four calls are all it needs:

| Call | Purpose |
|---|---|
| `cx.router()` | Reserves a `RouterId` once, in the constructor |
| `cx.scoped(router, key, \|cx\| build(cx))` | Builds one entry. Returns its value and a `Scope` naming its components |
| `cx.assign_scope(&scope, phase)` | Grants a phase to every component of an entry |
| `cx.destroy_scope(&scope)` | Removes an entry's components |

`key` is a `u64` that names the entry in saved state. Keep it stable for the
entry's life and unique within the router. A counter of visits works.

```rust
use compono::router::{RouterId, Scope};
use compono::{Component, Cx, Handle, Phase};

pub struct Screen;
impl Component for Screen {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

/// One screen at a time, each a new visit.
pub struct Solo {
    router: RouterId,
    visits: u64,
    pub shown: Option<(Handle<Screen>, Scope)>,
}

pub struct Root {
    pub solo: Solo,
}

impl Root {
    pub fn new(cx: &mut Cx<'_, Self>) -> Self {
        let router = cx.router();
        // A router that starts without saved navigation drops state saved by
        // entries of an earlier launch.
        cx.forget_router(router);
        Self { solo: Solo { router, visits: 0, shown: None } }
    }
}

impl Component for Root {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        let solo = &mut self.solo;
        if let Some((_old, scope)) = solo.shown.take() {
            cx.destroy_scope(&scope);
        }
        solo.visits += 1;
        let (screen, scope) = cx.scoped(solo.router, solo.visits, |cx| {
            cx.child(|_cx| Screen, |_root, (), _cx| {})
        });
        cx.assign_scope(&scope, Phase::Focused);
        solo.shown = Some((screen, scope));
    }
}
```

## Rules

- Take the `RouterId` once, from `cx.router()`, in the router's constructor. The
  owner then names it like a built-in router, with
  `cx.named_routers("strip", |cx| Filmstrip::new(…))`, which an owner should do for
  every router with saved state. An unnamed router is numbered in construction
  order, so the owner constructs it in a fixed order with its other routers.
- A new entry is `Hidden` until you grant it a phase with `assign_scope`.
- Starting fresh: call `cx.forget_router(router)` once.
- Restoring: rebuild the saved entries with their old keys, then call
  `cx.keep_router_entries(router, &keys)` once with the keys the snapshot names.
  Saved state of any other entry is discarded.
- An entry removed before it was rebuilt: `cx.forget_entry(router, key)`.
- Implement `compono::router::Router`: `router_ids` returns the ids it took, in
  order, and `destroy` destroys every scope at once. Hold every entry's `Scope` for
  as long as the entry lives: a dropped `Scope` removes its components when the
  running operation ends.
- An owner that replaces a router calls `cx.replace_router(old, |cx| …)`, which
  builds the new one in the old router's ids. Its documentation says which saved
  state the new router keeps.
- Keep the router a plain struct with public read methods. The UI matches on its
  children the same way as on built-in routers.

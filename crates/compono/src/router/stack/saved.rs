//! The persistable form of a stack.

use super::{Active, Build, Parked, Stack};
use crate::router::scope::repair_keys;
use crate::{Component, Cx, Phase};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// The persistable form of a [`Stack`], oldest entry first.
///
/// It can be saved only if the route can. A stack made with [`Stack::new`]
/// accepts any route, but a route that holds a runtime dependency cannot be
/// part of [`Persist::Saved`](crate::Persist::Saved):
///
/// ```compile_fail,E0277
/// use compono::router::stack::{SavedStack, Stack};
/// use compono::{Component, Cx, Persist};
///
/// /// A service, not data. It has no serialized form.
/// #[derive(Clone)]
/// struct Database;
///
/// struct Screens {
///     stack: Stack<Database, (), Screens>,
/// }
/// # impl Component for Screens {
/// #     type Msg = ();
/// #     type Output = ();
/// #     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
/// # }
///
/// impl Persist for Screens {
///     type Saved = SavedStack<Database>;
///     fn save(&self) -> Self::Saved {
///         self.stack.snapshot()
///     }
/// }
/// ```
///
/// The same component compiles when its route is plain data, and the service
/// is captured by the `build` closure instead:
///
/// ```
/// use compono::router::stack::{SavedStack, Stack};
/// use compono::{Component, Cx, Persist};
///
/// struct Screens {
///     stack: Stack<u32, (), Screens>,
/// }
/// # impl Component for Screens {
/// #     type Msg = ();
/// #     type Output = ();
/// #     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
/// # }
///
/// impl Persist for Screens {
///     type Saved = SavedStack<u32>;
///     fn save(&self) -> Self::Saved {
///         self.stack.snapshot()
///     }
/// }
/// ```
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedStack<R> {
    entries: Vec<(u64, R)>,
    next_key: u64,
}

impl<R: Clone + 'static, H: 'static, P: Component> Stack<R, H, P> {
    /// Captures routes and keys. Component state is saved by the tree.
    pub fn snapshot(&self) -> SavedStack<R> {
        let parked = self
            .parked
            .iter()
            .map(|entry| (entry.key, entry.route.clone()));
        let active = core::iter::once((self.active.key, self.active.route.clone()));
        SavedStack {
            entries: parked.chain(active).collect(),
            next_key: self.next_key,
        }
    }

    /// Rebuilds a stack, creating only the active component.
    ///
    /// Lower entries are created when popped back to, which keeps startup
    /// proportional to what is on screen. An empty or absent `saved` falls
    /// back to `initial`.
    pub fn restore_or(
        saved: Option<SavedStack<R>>,
        initial: R,
        cx: &mut Cx<'_, P>,
        build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static,
    ) -> Self {
        let Some(SavedStack {
            mut entries,
            next_key,
        }) = saved
        else {
            return Self::new(initial, cx, build);
        };
        // The active entry keeps its key if another entry repeats it.
        let next_key = repair_keys(entries.iter_mut().rev().map(|(key, _route)| key), next_key);
        let Some((key, route)) = entries.pop() else {
            return Self::new(initial, cx, build);
        };
        let build: Build<R, H, P> = Box::new(build);
        let router = cx.router();
        let named: Vec<u64> = entries
            .iter()
            .map(|(key, _route)| *key)
            .chain([key])
            .collect();
        cx.keep_router_entries(router, &named);
        let (child, scope) = cx.scoped(router, key, |cx| build(&route, cx));
        cx.assign_scope(&scope, Phase::Focused);
        let parked: Vec<Parked<R, H>> = entries
            .into_iter()
            .map(|(key, route)| Parked {
                key,
                route,
                live: None,
            })
            .collect();
        Self {
            active: Active {
                key,
                route,
                child,
                scope,
            },
            parked,
            next_key,
            router,
            build,
        }
    }
}

//! A stack of routes that always has an active entry.

use crate::router::{Router, RouterId, Scope};
use crate::{Component, Cx, Phase};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;

type Build<R, H, P> = Box<dyn Fn(&R, &mut Cx<'_, P>) -> H>;

struct Active<R, H> {
    key: u64,
    route: R,
    child: H,
    scope: Scope,
}

struct Parked<R, H> {
    key: u64,
    route: R,
    /// `None` for an entry restored but not yet needed.
    live: Option<(H, Scope)>,
}

/// Popping was refused because only the root entry remains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtRoot;

impl fmt::Display for AtRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("stack has only its root entry")
    }
}
impl core::error::Error for AtRoot {}

/// A replacement list had no entries, and a stack is never empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyRoutes;

impl fmt::Display for EmptyRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a stack needs at least one route")
    }
}
impl core::error::Error for EmptyRoutes {}

/// A navigation stack held as a field of its parent `P`.
///
/// `H` is the value the UI matches on, usually an enum of handles. It need not
/// contain a component at all.
///
/// ```
/// use compono::router::stack::Stack;
/// use compono::{Component, Cx, Handle, Phase, Tree};
/// # use compono::{Commit, Host};
/// # struct Quiet;
/// # impl Host for Quiet {
/// #     fn commit(&mut self, _commit: &Commit<'_>) {}
/// # }
/// # struct Screen;
/// # impl Component for Screen {
/// #     type Msg = ();
/// #     type Output = ();
/// #     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
/// # }
/// # fn screen<P: Component>(cx: &mut Cx<'_, P>) -> Handle<Screen> {
/// #     cx.child(|_cx| Screen, |_parent, (), _cx| {})
/// # }
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum Route {
///     Home,
///     Details(u32),
/// }
///
/// struct Root {
///     stack: Stack<Route, Handle<Screen>, Root>,
/// }
///
/// enum Msg {
///     Open(u32),
///     Back,
/// }
///
/// impl Component for Root {
///     type Msg = Msg;
///     type Output = ();
///
///     fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
///         match msg {
///             Msg::Open(id) => self.stack.push(Route::Details(id), cx),
///             Msg::Back => {
///                 // `Err(AtRoot)` at the root, which `back` below rules out.
///                 let _popped = self.stack.pop(cx);
///             }
///         }
///     }
///
///     fn back(&self) -> Option<Msg> {
///         self.stack.can_pop().then_some(Msg::Back)
///     }
/// }
///
/// let mut tree = Tree::new(Quiet);
/// tree.set_phase(Phase::Focused);
/// let root = tree.mount(|cx| Root {
///     stack: Stack::new(Route::Home, cx, |_route, cx| screen(cx)),
/// });
/// tree.send(root, Msg::Open(7))?;
/// assert_eq!(tree.get(root).map(|root| root.stack.depth()), Some(2));
///
/// assert!(tree.back());
/// let route = tree.get(root).map(|root| root.stack.route().clone());
/// assert_eq!(route, Some(Route::Home));
/// assert!(!tree.back(), "nothing handles Back at the root");
/// # Ok::<(), compono::StaleHandle>(())
/// ```
pub struct Stack<R, H, P> {
    active: Active<R, H>,
    parked: Vec<Parked<R, H>>,
    next_key: u64,
    router: RouterId,
    build: Build<R, H, P>,
}

impl<R, H, P> fmt::Debug for Stack<R, H, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stack")
            .field("depth", &(self.parked.len() + 1))
            .finish_non_exhaustive()
    }
}

impl<R: 'static, H: 'static, P: Component> Stack<R, H, P> {
    /// Creates a stack showing `initial`. `build` runs once per materialized entry.
    ///
    /// The stack starts with no saved state. Use `restore_or` to continue from
    /// a snapshot.
    pub fn new(
        initial: R,
        cx: &mut Cx<'_, P>,
        build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static,
    ) -> Self {
        let build: Build<R, H, P> = Box::new(build);
        let router = cx.router();
        cx.forget_router(router);
        let (child, scope) = cx.scoped(router, 0, |cx| build(&initial, cx));
        cx.assign_scope(&scope, Phase::Focused);
        Self {
            active: Active {
                key: 0,
                route: initial,
                child,
                scope,
            },
            parked: Vec::new(),
            next_key: 1,
            router,
            build,
        }
    }

    /// Returns the value of the active entry.
    pub const fn active(&self) -> &H {
        &self.active.child
    }

    /// Returns a number that identifies the active visit, for UI state keys.
    pub const fn active_key(&self) -> u64 {
        self.active.key
    }

    /// Returns the route of the active entry.
    pub const fn route(&self) -> &R {
        &self.active.route
    }

    /// Returns the number of entries, which is always at least one.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.parked.len() + 1
    }

    /// Returns whether [`Stack::pop`] would succeed.
    #[must_use]
    pub const fn can_pop(&self) -> bool {
        !self.parked.is_empty()
    }

    /// Returns the key and value of the entry that Back would reveal.
    ///
    /// A UI draws it beneath the active entry during a Back gesture. It is `None` at
    /// the root and for an entry restored but not yet created.
    pub fn previous(&self) -> Option<(u64, &H)> {
        let entry = self.parked.last()?;
        entry
            .live
            .as_ref()
            .map(|(child, _scope)| (entry.key, child))
    }

    /// Returns the entries below the active one, oldest first.
    ///
    /// The value is `None` for an entry restored but not yet created.
    pub fn parked(&self) -> impl Iterator<Item = (&R, Option<&H>)> {
        self.parked
            .iter()
            .map(|entry| (&entry.route, entry.live.as_ref().map(|(child, _)| child)))
    }

    /// Returns the key and route of every entry, oldest first, ending with the active one.
    ///
    /// A key names one visit, so an adapter that mirrors the stack elsewhere, such
    /// as browser history, can tell a new visit from a returning one.
    pub fn entries(&self) -> impl Iterator<Item = (u64, &R)> {
        self.parked
            .iter()
            .map(|entry| (entry.key, &entry.route))
            .chain(core::iter::once((self.active.key, &self.active.route)))
    }

    const fn mint_key(&mut self) -> u64 {
        let key = self.next_key;
        self.next_key = key.wrapping_add(1);
        key
    }

    /// Pushes a route. The old entry stays alive but hidden.
    pub fn push(&mut self, route: R, cx: &mut Cx<'_, P>) {
        let key = self.mint_key();
        let (child, scope) = cx.scoped(self.router, key, |cx| (self.build)(&route, cx));
        cx.assign_scope(&self.active.scope, Phase::Hidden);
        let old = core::mem::replace(
            &mut self.active,
            Active {
                key,
                route,
                child,
                scope,
            },
        );
        self.parked.push(Parked {
            key: old.key,
            route: old.route,
            live: Some((old.child, old.scope)),
        });
        cx.assign_scope(&self.active.scope, Phase::Focused);
    }

    /// Replaces the active entry with a new visit and returns the old route.
    pub fn replace(&mut self, route: R, cx: &mut Cx<'_, P>) -> R {
        let key = self.mint_key();
        let (child, scope) = cx.scoped(self.router, key, |cx| (self.build)(&route, cx));
        let old = core::mem::replace(
            &mut self.active,
            Active {
                key,
                route,
                child,
                scope,
            },
        );
        cx.destroy_scope(&old.scope);
        cx.assign_scope(&self.active.scope, Phase::Focused);
        old.route
    }

    /// Destroys every entry and shows `route` as the new root.
    pub fn reset(&mut self, route: R, cx: &mut Cx<'_, P>) {
        self.discard_parked(cx);
        let _old = self.replace(route, cx);
    }

    /// Pops until `depth` entries remain and returns how many were removed.
    ///
    /// A `depth` of zero keeps the root entry, because a stack is never empty.
    pub fn pop_to(&mut self, depth: usize, cx: &mut Cx<'_, P>) -> usize {
        let target = depth.max(1);
        let mut removed = 0;
        while self.depth() > target {
            if self.depth() > target + 1 {
                if let Some(parked) = self.parked.pop() {
                    self.discard(parked, cx);
                }
            } else if self.pop(cx).is_err() {
                break;
            }
            removed += 1;
        }
        removed
    }

    /// Pops while `condition` holds for the top entry, and returns how many went.
    ///
    /// The root entry always stays.
    pub fn pop_while(
        &mut self,
        mut condition: impl FnMut(&R) -> bool,
        cx: &mut Cx<'_, P>,
    ) -> usize {
        let poppable = self.depth() - 1;
        let removed = core::iter::once(&self.active.route)
            .chain(self.parked.iter().rev().map(|entry| &entry.route))
            .take(poppable)
            .take_while(|route| condition(route))
            .count();
        self.pop_to(self.depth() - removed, cx)
    }

    /// Replaces every entry with `routes`, oldest first, and shows the last.
    ///
    /// Only the last entry is created now. The others are created when
    /// navigation returns to them.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyRoutes`] without changing anything if `routes` is empty.
    pub fn replace_all(
        &mut self,
        routes: impl IntoIterator<Item = R>,
        cx: &mut Cx<'_, P>,
    ) -> Result<(), EmptyRoutes> {
        let mut routes: Vec<R> = routes.into_iter().collect();
        let top = routes.pop().ok_or(EmptyRoutes)?;
        self.discard_parked(cx);
        let _old = self.replace(top, cx);
        for route in routes {
            let key = self.mint_key();
            self.parked.push(Parked {
                key,
                route,
                live: None,
            });
        }
        Ok(())
    }

    fn discard(&self, parked: Parked<R, H>, cx: &mut Cx<'_, P>) {
        match parked.live {
            Some((_, scope)) => cx.destroy_scope(&scope),
            None => cx.forget_entry(self.router, parked.key),
        }
    }

    fn discard_parked(&mut self, cx: &mut Cx<'_, P>) {
        while let Some(parked) = self.parked.pop() {
            self.discard(parked, cx);
        }
    }

    fn materialize(&self, parked: Parked<R, H>, cx: &mut Cx<'_, P>) -> Active<R, H> {
        let (child, scope) = match parked.live {
            Some(live) => live,
            None => cx.scoped(self.router, parked.key, |cx| {
                (self.build)(&parked.route, cx)
            }),
        };
        Active {
            key: parked.key,
            route: parked.route,
            child,
            scope,
        }
    }

    /// Pops the active entry, destroys its component and returns its route.
    ///
    /// # Errors
    ///
    /// Returns [`AtRoot`] when only the root entry remains.
    pub fn pop(&mut self, cx: &mut Cx<'_, P>) -> Result<R, AtRoot> {
        let parked = self.parked.pop().ok_or(AtRoot)?;
        let next = self.materialize(parked, cx);
        let old = core::mem::replace(&mut self.active, next);
        cx.destroy_scope(&old.scope);
        cx.assign_scope(&self.active.scope, Phase::Focused);
        Ok(old.route)
    }
}

impl<R: 'static, H: 'static, P: Component> Router<P> for Stack<R, H, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        [self.router]
    }

    fn destroy(mut self, cx: &mut Cx<'_, P>) {
        self.discard_parked(cx);
        cx.destroy_scope(&self.active.scope);
    }
}

impl<R: PartialEq + 'static, H: 'static, P: Component> Stack<R, H, P> {
    /// Pushes `route` unless it is already active. Returns whether it was pushed.
    pub fn push_new(&mut self, route: R, cx: &mut Cx<'_, P>) -> bool {
        if self.active.route == route {
            return false;
        }
        self.push(route, cx);
        true
    }

    /// Removes every entry equal to `route`, then pushes it as a new visit.
    pub fn push_to_front(&mut self, route: R, cx: &mut Cx<'_, P>) {
        while let Some(position) = self.position_of(&route) {
            let parked = self.parked.remove(position);
            self.discard(parked, cx);
        }
        if self.active.route == route {
            let _old = self.replace(route, cx);
        } else {
            self.push(route, cx);
        }
    }

    /// Moves the newest entry equal to `route` to the top, keeping its state.
    ///
    /// If no entry matches, this pushes `route`.
    pub fn bring_to_front(&mut self, route: R, cx: &mut Cx<'_, P>) {
        if self.active.route == route {
            return;
        }
        let Some(position) = self.position_of(&route) else {
            self.push(route, cx);
            return;
        };
        let parked = self.parked.remove(position);
        let next = self.materialize(parked, cx);
        cx.assign_scope(&self.active.scope, Phase::Hidden);
        let old = core::mem::replace(&mut self.active, next);
        self.parked.push(Parked {
            key: old.key,
            route: old.route,
            live: Some((old.child, old.scope)),
        });
        cx.assign_scope(&self.active.scope, Phase::Focused);
    }

    fn position_of(&self, route: &R) -> Option<usize> {
        self.parked.iter().rposition(|entry| entry.route == *route)
    }
}

#[cfg(feature = "serde")]
mod saved;

#[cfg(feature = "serde")]
pub use saved::SavedStack;

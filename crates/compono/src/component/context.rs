use crate::router::Router;
use crate::router::scope::{RouterId, Scope};
use crate::tree::{FNV_OFFSET, Route, Tree, Work, fnv1a};
use crate::{Component, Diagnostic, Handle, NodeId, NotOwnChild, Phase};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

/// Everything a running component may do. It offers no path to the parent value.
pub struct Cx<'t, C> {
    pub(crate) tree: &'t mut Tree,
    pub(crate) me: NodeId,
    unchanged: bool,
    /// Ids of a replaced router that [`Cx::router`] hands out before new ones,
    /// last first.
    reuse: Vec<RouterId>,
    /// The name [`Cx::router`] gives new routers, if any.
    naming: Option<Naming>,
    marker: PhantomData<fn() -> C>,
}

/// The name a group of routers saves under, and how many it has named.
#[derive(Clone, Copy)]
struct Naming {
    hash: u64,
    next: u32,
}

impl Naming {
    /// Returns the hash of the next router's name. The first router takes the
    /// name itself, and the routers after it the name followed by their place.
    fn next_hash(&mut self) -> u64 {
        let place = self.next;
        self.next = place.saturating_add(1);
        if place == 0 {
            return self.hash;
        }
        // 0xff never occurs in UTF-8, so no name can produce the same bytes.
        fnv1a(fnv1a(self.hash, &[0xff]), &place.to_le_bytes())
    }
}

impl<C> fmt::Debug for Cx<'_, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cx")
            .field("me", &self.me)
            .finish_non_exhaustive()
    }
}

impl<'t, C: Component> Cx<'t, C> {
    pub(crate) fn new(tree: &'t mut Tree, me: NodeId) -> Self {
        Self {
            tree,
            me,
            unchanged: false,
            reuse: Vec::new(),
            naming: None,
            marker: PhantomData,
        }
    }

    pub(crate) fn finish(self) {
        if !self.unchanged {
            self.tree.mark_dirty(self.me);
        }
    }

    /// Queues an event for the parent. It is delivered after this component returns.
    pub fn emit(&mut self, output: C::Output) {
        self.tree.enqueue(Work::Output {
            from: self.me,
            payload: Box::new(output),
        });
    }

    /// Queues a message for this component or one of its children.
    ///
    /// It is delivered after this component returns, unless the target is
    /// removed before then.
    ///
    /// # Errors
    ///
    /// Returns [`NotOwnChild`] and queues nothing if the target is neither this
    /// component nor one of its live children. A child therefore cannot reach its
    /// parent even if it is handed the parent's handle, and a grandchild is
    /// reached through its own parent.
    pub fn send<K: Component>(
        &mut self,
        target: Handle<K>,
        msg: K::Msg,
    ) -> Result<(), NotOwnChild> {
        let to = target.id();
        if to != self.me && self.tree.parent(to) != Some(self.me) {
            return Err(NotOwnChild);
        }
        self.tree.enqueue(Work::Msg {
            to,
            payload: Box::new(msg),
        });
        Ok(())
    }

    /// Returns this component's effective phase.
    #[must_use]
    pub fn phase(&self) -> Phase {
        self.tree.phase_of(self.me)
    }

    /// Declares that this run changed nothing a UI could show.
    pub fn unchanged(&mut self) {
        self.unchanged = true;
    }

    /// Creates a child. `on_output` is the only link between the child and this parent.
    ///
    /// The child follows this component's phase unless [`Cx::set_child_phase`]
    /// lowers it. A child created for a router entry stays hidden until the
    /// router grants it a phase.
    ///
    /// ```
    /// use compono::{Component, Cx, Handle, Tree};
    /// # use compono::{Commit, Host};
    /// # struct Quiet;
    /// # impl Host for Quiet {
    /// #     fn commit(&mut self, _commit: &Commit<'_>) {}
    /// # }
    ///
    /// struct Form {
    ///     name: Handle<Field>,
    ///     submitted: Option<String>,
    /// }
    ///
    /// impl Component for Form {
    ///     type Msg = ();
    ///     type Output = ();
    ///     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    /// }
    ///
    /// struct Field;
    ///
    /// impl Component for Field {
    ///     type Msg = String;
    ///     /// What the field tells whoever owns it. It never sees the form.
    ///     type Output = String;
    ///
    ///     fn update(&mut self, text: String, cx: &mut Cx<'_, Self>) {
    ///         cx.emit(text);
    ///     }
    /// }
    ///
    /// let mut tree = Tree::new(Quiet);
    /// let form = tree.mount(|cx| Form {
    ///     name: cx.child(
    ///         |_cx| Field,
    ///         |form: &mut Form, text, _cx| form.submitted = Some(text),
    ///     ),
    ///     submitted: None,
    /// });
    /// let field = tree.get(form).map(|form| form.name);
    /// if let Some(field) = field {
    ///     tree.send(field, "Ada".to_owned())?;
    /// }
    /// let submitted = tree.get(form).and_then(|form| form.submitted.clone());
    /// assert_eq!(submitted.as_deref(), Some("Ada"));
    /// # Ok::<(), compono::StaleHandle>(())
    /// ```
    pub fn child<K: Component>(
        &mut self,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
        on_output: impl Fn(&mut C, K::Output, &mut Cx<'_, C>) + 'static,
    ) -> Handle<K> {
        self.child_named(None, init, on_output)
    }

    /// Creates a child that saved state finds by `name` instead of by creation order.
    ///
    /// Outside a router entry, a child's saved state is otherwise matched by the
    /// order in which this parent created its children, so a child added before
    /// it in a later version would take its state. A named child keeps its state
    /// whatever is created around it. Names are unique among one parent's
    /// children: a child whose name a live sibling already uses saves nothing.
    /// Inside a router entry the entry identifies its children, and the name is
    /// not needed.
    ///
    /// The name matters only for saved state, through
    /// [`Cx::persistent_child_keyed`] or a persistent child below this one.
    pub fn child_keyed<K: Component>(
        &mut self,
        name: &str,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
        on_output: impl Fn(&mut C, K::Output, &mut Cx<'_, C>) + 'static,
    ) -> Handle<K> {
        self.child_named(Some(name), init, on_output)
    }

    pub(crate) fn child_named<K: Component>(
        &mut self,
        name: Option<&str>,
        init: impl FnOnce(&mut Cx<'_, K>) -> K,
        on_output: impl Fn(&mut C, K::Output, &mut Cx<'_, C>) + 'static,
    ) -> Handle<K> {
        let route: Route = Box::new(move |value, output, tree, id| {
            let (Some(value), Ok(output)) =
                (value.downcast_mut::<C>(), output.downcast::<K::Output>())
            else {
                return;
            };
            let mut cx = Cx::new(tree, id);
            on_output(value, *output, &mut cx);
            cx.finish();
        });
        let id = self.tree.allocate(Some(self.me), Some(route), name);
        let handle = self.tree.build(id, init);
        self.tree.refresh(id);
        handle
    }

    /// Destroys a child of this component, with its own children.
    ///
    /// # Errors
    ///
    /// Returns [`NotOwnChild`] and removes nothing if the target is not one of
    /// this component's live children, such as a grandchild or a child already
    /// removed.
    pub fn destroy<K>(&mut self, child: Handle<K>) -> Result<(), NotOwnChild> {
        self.destroy_id(child.id())
    }

    /// Sets the phase this parent grants a child.
    ///
    /// # Errors
    ///
    /// Returns [`NotOwnChild`] and changes nothing if the target is not one of
    /// this component's live children.
    pub fn set_child_phase<K>(
        &mut self,
        child: Handle<K>,
        phase: Phase,
    ) -> Result<(), NotOwnChild> {
        self.assign_id(child.id(), phase)
    }

    fn destroy_id(&mut self, id: NodeId) -> Result<(), NotOwnChild> {
        if self.tree.parent(id) != Some(self.me) {
            return Err(NotOwnChild);
        }
        self.tree.destroy(id);
        Ok(())
    }

    fn assign_id(&mut self, id: NodeId, phase: Phase) -> Result<(), NotOwnChild> {
        if self.tree.parent(id) != Some(self.me) {
            return Err(NotOwnChild);
        }
        self.tree.assign(id, phase);
        Ok(())
    }

    /// Grants one phase to every component of a router entry.
    pub fn assign_scope(&mut self, scope: &Scope, phase: Phase) {
        for id in scope.ids() {
            // A component of the entry already removed has no phase to change.
            let _removed = self.assign_id(id, phase);
        }
    }

    /// Destroys every component of a router entry.
    pub fn destroy_scope(&mut self, scope: &Scope) {
        for id in scope.ids() {
            // A component of the entry already removed needs nothing more.
            let _removed = self.destroy_id(id);
        }
    }

    /// Discards saved state of an entry removed before it was ever rebuilt.
    ///
    /// Does nothing while [`Cx::replace_router`] destroys the old router,
    /// because the router that replaces it decides what happens to that state.
    pub fn forget_entry(&mut self, router: RouterId, key: u64) {
        #[cfg(feature = "serde")]
        if self.may_forget(router) {
            let key = router.saved_key(key);
            self.tree.forget_entries(self.me, key..=key, |_| true);
        }
        #[cfg(not(feature = "serde"))]
        let _nothing_saved = (router, key);
    }

    /// Discards the saved state of every entry of a router except `keys`.
    ///
    /// A router restored from a snapshot calls this once with the keys the
    /// snapshot names, after repairing them. State saved for any other entry,
    /// from damaged or older bytes, would otherwise be carried into every later
    /// save. The built-in routers do so when they restore.
    pub fn keep_router_entries(&mut self, router: RouterId, keys: &[u64]) {
        #[cfg(feature = "serde")]
        if self.may_forget(router) {
            let (first, last) = router.saved_keys();
            let mut kept: alloc::vec::Vec<u64> =
                keys.iter().map(|key| router.saved_key(*key)).collect();
            kept.sort_unstable();
            self.tree.forget_entries(self.me, first..=last, |key| {
                kept.binary_search(&key).is_err()
            });
        }
        #[cfg(not(feature = "serde"))]
        let _nothing_saved = (router, keys);
    }

    /// Discards the saved state of every entry of a router.
    ///
    /// A router that starts without saved navigation calls this once, so that
    /// its new entries are not given state saved by other entries of an earlier
    /// launch. The built-in routers do so in `new`.
    pub fn forget_router(&mut self, router: RouterId) {
        #[cfg(feature = "serde")]
        if self.may_forget(router) {
            let (first, last) = router.saved_keys();
            self.tree.forget_entries(self.me, first..=last, |_| true);
        }
        #[cfg(not(feature = "serde"))]
        let _nothing_saved = router;
    }

    /// Returns whether this component may discard saved state of `router`: the
    /// router is its own, and it is not being replaced.
    #[cfg(feature = "serde")]
    fn may_forget(&self, router: RouterId) -> bool {
        router.owner() == self.me && self.tree.keeping_saved != Some(self.me)
    }

    /// Destroys `old`, then builds its replacement with `build`, which takes
    /// the old router's ids in order.
    ///
    /// A router's entries are saved under its ids, and on the next launch the
    /// owner's constructor gives its routers ids by name or in construction
    /// order. A router that replaces another must therefore take the other's
    /// ids, or its entries lose their saved state. Taking them also uses up
    /// none of the component's numbered ids.
    ///
    /// The saved state under the old ids stays while `old` is destroyed, and
    /// the new router decides what to keep: a router made with `new` starts
    /// clean, and one restored from the old router's snapshot gives entries not
    /// yet shown their state when they are. Routers `build` makes past the
    /// old router's ids get new ids.
    ///
    /// Hold a router you replace in an `Option`, so it can be moved out:
    ///
    /// ```
    /// use compono::router::slot::Slot;
    /// use compono::{Component, Cx, Tree};
    /// # use compono::{Commit, Host};
    /// # struct Quiet;
    /// # impl Host for Quiet {
    /// #     fn commit(&mut self, _commit: &Commit<'_>) {}
    /// # }
    ///
    /// struct Menu {
    ///     dialog: Option<Slot<&'static str, &'static str, Menu>>,
    /// }
    ///
    /// impl Component for Menu {
    ///     type Msg = ();
    ///     type Output = ();
    ///     fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
    ///         if let Some(old) = self.dialog.take() {
    ///             let fresh = cx.replace_router(old, |cx| Slot::new(cx, |text, _cx| *text));
    ///             self.dialog = Some(fresh);
    ///         }
    ///     }
    /// }
    ///
    /// let mut tree = Tree::new(Quiet);
    /// let menu = tree.mount(|cx| Menu { dialog: Some(Slot::new(cx, |text, _cx| *text)) });
    /// tree.send(menu, ())?;
    /// assert!(tree.get(menu).is_some_and(|menu| menu.dialog.is_some()));
    /// # Ok::<(), compono::StaleHandle>(())
    /// ```
    pub fn replace_router<R: Router<C>>(
        &mut self,
        old: R,
        build: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let mut ids: Vec<RouterId> = old.router_ids().into_iter().collect();
        ids.reverse();
        let outer = self.tree.keeping_saved.replace(self.me);
        old.destroy(self);
        self.tree.keeping_saved = outer;
        let outer = core::mem::replace(&mut self.reuse, ids);
        let new = build(self);
        self.reuse = outer;
        new
    }

    /// Runs `build`, whose routers find their saved state by `name`.
    ///
    /// A router otherwise saves under a number given in construction order, so
    /// a later version that constructs another router before it, or constructs
    /// a router only behind a feature flag, would give it another router's
    /// state. A named router keeps its state whatever is constructed around it.
    /// The name changes only where the router saves, not what it does, and it
    /// takes no number, so numbered routers keep theirs.
    ///
    /// Each router `build` constructs takes the name, so several routers that
    /// make up one, such as the three of a `Panels`, share it. Names are unique
    /// among one component's routers: a name this component has already given
    /// out gives a router that saves nothing. To replace a named router, use
    /// [`Cx::replace_router`], which gives the replacement the old router's ids.
    ///
    /// ```
    /// use compono::router::stack::Stack;
    /// use compono::{Component, Cx, Tree};
    /// # use compono::{Commit, Host};
    /// # struct Quiet;
    /// # impl Host for Quiet {
    /// #     fn commit(&mut self, _commit: &Commit<'_>) {}
    /// # }
    ///
    /// struct Shell {
    ///     tabs: Stack<u32, (), Shell>,
    /// }
    ///
    /// impl Component for Shell {
    ///     type Msg = u32;
    ///     type Output = ();
    ///     fn update(&mut self, tab: u32, cx: &mut Cx<'_, Self>) {
    ///         self.tabs.bring_to_front(tab, cx);
    ///     }
    /// }
    ///
    /// let mut tree = Tree::new(Quiet);
    /// let shell = tree.mount(|cx| Shell {
    ///     // With saved state, this is `Stack::restore_or(cx.restored(), 0, cx, …)`.
    ///     tabs: cx.named_routers("tabs", |cx| Stack::new(0, cx, |_tab, _cx| ())),
    /// });
    /// tree.send(shell, 2)?;
    /// assert_eq!(tree.get(shell).map(|shell| *shell.tabs.route()), Some(2));
    /// # Ok::<(), compono::StaleHandle>(())
    /// ```
    pub fn named_routers<T>(&mut self, name: &str, build: impl FnOnce(&mut Self) -> T) -> T {
        let naming = Naming {
            hash: fnv1a(FNV_OFFSET, name.as_bytes()),
            next: 0,
        };
        let outer = self.naming.replace(naming);
        let value = build(self);
        self.naming = outer;
        value
    }

    /// Reserves the identity of a new router owned by this component.
    ///
    /// Inside the `build` of [`Cx::replace_router`] it is the replaced router's
    /// next id. Otherwise, inside [`Cx::named_routers`] it is the name, and
    /// elsewhere the next number in construction order. Every router, built-in
    /// or custom, calls this once in its constructor.
    pub fn router(&mut self) -> RouterId {
        let named = self.naming.as_mut().map(Naming::next_hash);
        if let Some(id) = self.reuse.pop() {
            return id;
        }
        if let Some(hash) = named {
            return self
                .tree
                .name_router(self.me, RouterId::named(self.me, hash));
        }
        let Some(slot) = self.tree.slot_mut(self.me) else {
            return RouterId::unsaved(self.me);
        };
        let number = slot.next_router;
        // The counter stops at the id that saves nothing, so it never wraps.
        slot.next_router = number.saturating_add(1);
        let id = RouterId::numbered(self.me, number);
        if !id.is_saved() {
            self.tree
                .diagnose(&Diagnostic::RouterLimit { owner: self.me });
        }
        id
    }

    /// Runs `build` and records the children it creates as one router entry.
    ///
    /// `key` identifies the entry in saved state, so it must stay stable for
    /// the life of the entry and be unique within `router`.
    /// Together with [`Cx::assign_scope`] and [`Cx::destroy_scope`] this is
    /// all a custom navigation model needs; the built-in routers use nothing else.
    ///
    /// ```
    /// use compono::router::{RouterId, Scope};
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
    ///
    /// /// A custom router: one screen at a time, each a new visit.
    /// struct Solo {
    ///     router: RouterId,
    ///     visits: u64,
    ///     shown: Option<(Handle<Screen>, Scope)>,
    /// }
    ///
    /// struct Root {
    ///     solo: Solo,
    /// }
    ///
    /// impl Component for Root {
    ///     type Msg = ();
    ///     type Output = ();
    ///
    ///     fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
    ///         let solo = &mut self.solo;
    ///         if let Some((_old, scope)) = solo.shown.take() {
    ///             cx.destroy_scope(&scope);
    ///         }
    ///         solo.visits += 1;
    ///         let (screen, scope) = cx.scoped(solo.router, solo.visits, |cx| {
    ///             cx.child(|_cx| Screen, |_root, (), _cx| {})
    ///         });
    ///         cx.assign_scope(&scope, Phase::Focused);
    ///         solo.shown = Some((screen, scope));
    ///     }
    /// }
    ///
    /// let mut tree = Tree::new(Quiet);
    /// tree.set_phase(Phase::Focused);
    /// let root = tree.mount(|cx| {
    ///     let router = cx.router();
    ///     cx.forget_router(router);
    ///     Root { solo: Solo { router, visits: 0, shown: None } }
    /// });
    /// tree.send(root, ())?;
    /// let first = tree.get(root).and_then(|root| root.solo.shown.as_ref().map(|shown| shown.0));
    /// assert_eq!(first.and_then(|screen| tree.phase(screen)), Some(Phase::Focused));
    ///
    /// tree.send(root, ())?;
    /// assert!(first.is_some_and(|screen| tree.get(screen).is_none()), "the old visit is gone");
    /// # Ok::<(), compono::StaleHandle>(())
    /// ```
    pub fn scoped<T>(
        &mut self,
        router: RouterId,
        key: u64,
        build: impl FnOnce(&mut Self) -> T,
    ) -> (T, Scope) {
        // An id of another component saves nothing here.
        let base = if router.owner() == self.me {
            router.saved_key(key)
        } else {
            crate::saved_key::UNSAVED
        };
        let previous = self.tree.begin_scope(self.me, base);
        let mark = self.tree.created_len();
        let value = build(self);
        let scope = self.tree.end_scope(previous, mark);
        (value, scope)
    }
}

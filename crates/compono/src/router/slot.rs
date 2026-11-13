//! Zero or one child, for dialogs, sheets and similar overlays.

use crate::router::{Router, RouterId, Scope};
use crate::{Component, Cx, Phase};
use alloc::boxed::Box;
use core::fmt;

type Build<R, H, P> = Box<dyn Fn(&R, &mut Cx<'_, P>) -> H>;

struct Entry<R, H> {
    #[cfg_attr(
        not(feature = "serde"),
        expect(dead_code, reason = "only persistence reads this")
    )]
    key: u64,
    route: R,
    child: H,
    scope: Scope,
}

/// An optional child held as a field of its parent `P`.
///
/// `H` is the value the UI matches on. A plain dialog can be a value with no
/// component behind it.
///
/// Opening a slot does not lower the parent's other children. A screen under a
/// dialog stays `Focused` unless the parent lowers it with `Cx::set_child_phase`,
/// because only the parent knows whether the dialog is modal.
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
/// struct Editor {
///     /// A confirmation with nothing but its text needs no component.
///     dialog: Slot<String, String, Editor>,
/// }
///
/// enum Msg {
///     AskToDelete,
///     Dismiss,
/// }
///
/// impl Component for Editor {
///     type Msg = Msg;
///     type Output = ();
///
///     fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
///         match msg {
///             Msg::AskToDelete => {
///                 self.dialog.show("Delete this note?".to_owned(), cx);
///             }
///             Msg::Dismiss => {
///                 self.dialog.close(cx);
///             }
///         }
///     }
///
///     fn back(&self) -> Option<Msg> {
///         self.dialog.is_open().then_some(Msg::Dismiss)
///     }
/// }
///
/// let mut tree = Tree::new(Quiet);
/// let editor = tree.mount(|cx| Editor {
///     dialog: Slot::new(cx, |text: &String, _cx| text.clone()),
/// });
/// tree.send(editor, Msg::AskToDelete)?;
/// let shown = tree.get(editor).and_then(|editor| editor.dialog.active().cloned());
/// assert_eq!(shown.as_deref(), Some("Delete this note?"));
/// tree.send(editor, Msg::Dismiss)?;
/// assert_eq!(tree.get(editor).map(|editor| editor.dialog.is_open()), Some(false));
/// # Ok::<(), compono::StaleHandle>(())
/// ```
pub struct Slot<R, H, P> {
    entry: Option<Entry<R, H>>,
    next_key: u64,
    phase: Phase,
    router: RouterId,
    build: Build<R, H, P>,
}

impl<R, H, P> fmt::Debug for Slot<R, H, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slot")
            .field("open", &self.entry.is_some())
            .finish_non_exhaustive()
    }
}

impl<R: 'static, H: 'static, P: Component> Slot<R, H, P> {
    /// Creates a closed slot. `build` runs once each time an entry is shown.
    pub fn new(cx: &mut Cx<'_, P>, build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static) -> Self {
        let router = cx.router();
        cx.forget_router(router);
        Self {
            entry: None,
            next_key: 0,
            phase: Phase::Focused,
            router,
            build: Box::new(build),
        }
    }

    /// Returns the value of the open entry.
    pub fn active(&self) -> Option<&H> {
        self.entry.as_ref().map(|entry| &entry.child)
    }

    /// Returns the route of the open entry.
    pub fn route(&self) -> Option<&R> {
        self.entry.as_ref().map(|entry| &entry.route)
    }

    /// Returns whether an entry is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.entry.is_some()
    }

    /// Returns the phase granted to the entry, open now or shown later.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Sets the phase granted to the entry, now and when it is next shown.
    pub fn set_phase(&mut self, phase: Phase, cx: &mut Cx<'_, P>) {
        self.phase = phase;
        if let Some(entry) = &self.entry {
            cx.assign_scope(&entry.scope, phase);
        }
    }

    /// Shows `route`, destroying any open entry, and returns the old route.
    pub fn show(&mut self, route: R, cx: &mut Cx<'_, P>) -> Option<R> {
        let old = self.close(cx);
        let key = self.next_key;
        self.next_key = key.wrapping_add(1);
        self.open(key, route, cx);
        old
    }

    /// Destroys the open entry and returns its route.
    pub fn close(&mut self, cx: &mut Cx<'_, P>) -> Option<R> {
        let entry = self.entry.take()?;
        cx.destroy_scope(&entry.scope);
        Some(entry.route)
    }

    fn open(&mut self, key: u64, route: R, cx: &mut Cx<'_, P>) {
        let (child, scope) = cx.scoped(self.router, key, |cx| (self.build)(&route, cx));
        cx.assign_scope(&scope, self.phase);
        self.entry = Some(Entry {
            key,
            route,
            child,
            scope,
        });
    }
}

impl<R: 'static, H: 'static, P: Component> Router<P> for Slot<R, H, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        [self.router]
    }

    fn destroy(mut self, cx: &mut Cx<'_, P>) {
        let _route = self.close(cx);
    }
}

#[cfg(feature = "serde")]
mod saved {
    use super::Slot;
    use crate::router::scope::repair_keys;
    use crate::{Component, Cx, Phase};
    use alloc::boxed::Box;

    /// The persistable form of a [`Slot`].
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct SavedSlot<R> {
        entry: Option<(u64, R)>,
        next_key: u64,
    }

    impl<R: Clone + 'static, H: 'static, P: Component> Slot<R, H, P> {
        /// Captures the route and key. Component state is saved by the tree.
        pub fn snapshot(&self) -> SavedSlot<R> {
            SavedSlot {
                entry: self
                    .entry
                    .as_ref()
                    .map(|entry| (entry.key, entry.route.clone())),
                next_key: self.next_key,
            }
        }

        /// Rebuilds a slot. An absent `saved` gives a closed slot.
        pub fn restore(
            saved: Option<SavedSlot<R>>,
            cx: &mut Cx<'_, P>,
            build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static,
        ) -> Self {
            Self::restore_at(saved, Phase::Focused, cx, build)
        }
    }

    impl<R: 'static, H: 'static, P: Component> Slot<R, H, P> {
        /// Rebuilds a slot whose entry is granted `phase`.
        pub(crate) fn restore_at(
            saved: Option<SavedSlot<R>>,
            phase: Phase,
            cx: &mut Cx<'_, P>,
            build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static,
        ) -> Self {
            let Some(saved) = saved else {
                let mut slot = Self::new(cx, build);
                slot.phase = phase;
                return slot;
            };
            let (mut entry, next_key) = (saved.entry, saved.next_key);
            let next_key = repair_keys(entry.iter_mut().map(|(key, _route)| key), next_key);
            let router = cx.router();
            let named: alloc::vec::Vec<u64> = entry.iter().map(|(key, _route)| *key).collect();
            cx.keep_router_entries(router, &named);
            let mut slot = Self {
                entry: None,
                next_key,
                phase,
                router,
                build: Box::new(build),
            };
            if let Some((key, route)) = entry {
                slot.open(key, route, cx);
            }
            slot
        }
    }
}

#[cfg(feature = "serde")]
pub use saved::SavedSlot;

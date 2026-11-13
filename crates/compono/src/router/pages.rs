//! A list of pages with one selected, keeping only pages near it alive.

use super::set::{ChildSet, DuplicateKey, check_unique};
use super::{Router, RouterId};
use crate::{Component, Cx, Phase};
use alloc::vec::Vec;
use core::fmt;
use core::hash::Hash;

/// Pages held as a field of their parent `P`.
///
/// The selected page is `Focused`. Pages within `retain` of it exist but are
/// `Hidden`, so swiping to them is instant. Other pages have no component.
///
/// ```
/// use compono::router::pages::Pages;
/// use compono::{Component, Cx, Handle, Tree};
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
/// struct Book {
///     pages: Pages<u32, Handle<Screen>, Book>,
/// }
///
/// impl Component for Book {
///     /// The index of the page to turn to.
///     type Msg = usize;
///     type Output = ();
///
///     fn update(&mut self, index: usize, cx: &mut Cx<'_, Self>) {
///         if !self.pages.select(index, cx) {
///             cx.unchanged();
///         }
///     }
/// }
///
/// impl Book {
///     fn new(chapters: Vec<u32>, cx: &mut Cx<'_, Self>) -> Self {
///         // One page on each side of the selected one stays alive.
///         let build = |_chapter: &u32, cx: &mut Cx<'_, Self>| screen(cx);
///         let pages = Pages::new(chapters, 0, 1, cx, build)
///             // A chapter that repeats is refused. This book then has no pages.
///             .or_else(|_repeated| Pages::new(Vec::new(), 0, 1, cx, build));
///         match pages {
///             Ok(pages) => Self { pages },
///             Err(_) => unreachable!("an empty list has no repeats"),
///         }
///     }
/// }
///
/// let mut tree = Tree::new(Quiet);
/// let book = tree.mount(|cx| Book::new((1..=100).collect(), cx));
/// tree.send(book, 50)?;
/// let pages = tree.get(book).map(|book| (book.pages.selected(), book.pages.live()));
/// assert_eq!(pages, Some((50, 3)));
/// # Ok::<(), compono::StaleHandle>(())
/// ```
pub struct Pages<R, H, P> {
    routes: Vec<R>,
    selected: usize,
    retain: usize,
    children: ChildSet<R, H, P>,
}

impl<R, H, P> fmt::Debug for Pages<R, H, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pages")
            .field("pages", &self.routes.len())
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl<R: Clone + Eq + Hash + 'static, H: 'static, P: Component> Pages<R, H, P> {
    /// Creates pages showing `selected`, which is clamped to the list.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateKey`] if a route repeats.
    pub fn new(
        routes: Vec<R>,
        selected: usize,
        retain: usize,
        cx: &mut Cx<'_, P>,
        build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static,
    ) -> Result<Self, DuplicateKey> {
        check_unique(&routes)?;
        let selected = selected.min(routes.len().saturating_sub(1));
        let mut pages = Self {
            routes,
            selected,
            retain,
            children: ChildSet::new(cx, build),
        };
        pages.sync(cx);
        Ok(pages)
    }

    /// Returns every route.
    #[must_use]
    pub fn routes(&self) -> &[R] {
        &self.routes
    }

    /// Returns the index of the selected page.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// Returns the value of the selected page.
    pub fn active(&self) -> Option<&H> {
        let route = self.routes.get(self.selected)?;
        self.children
            .iter()
            .find(|(key, _child, _phase)| *key == route)
            .map(|(_key, child, _phase)| child)
    }

    /// Returns the number of pages that currently have a component.
    #[must_use]
    pub const fn live(&self) -> usize {
        self.children.len()
    }

    /// Selects a page. Returns whether `index` exists.
    pub fn select(&mut self, index: usize, cx: &mut Cx<'_, P>) -> bool {
        if index >= self.routes.len() {
            return false;
        }
        self.selected = index;
        self.sync(cx);
        true
    }

    /// Replaces the pages. Surviving routes keep their components.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateKey`] without changing anything if a route repeats.
    pub fn set_routes(&mut self, routes: Vec<R>, cx: &mut Cx<'_, P>) -> Result<(), DuplicateKey> {
        check_unique(&routes)?;
        self.selected = self.selected.min(routes.len().saturating_sub(1));
        self.routes = routes;
        self.sync(cx);
        Ok(())
    }

    fn sync(&mut self, cx: &mut Cx<'_, P>) {
        let start = self.selected.saturating_sub(self.retain);
        let end = self
            .selected
            .saturating_add(self.retain)
            .saturating_add(1)
            .min(self.routes.len());
        let selected = self.selected;
        let desired = self
            .routes
            .iter()
            .enumerate()
            .take(end)
            .skip(start)
            .map(|(index, route)| {
                let phase = if index == selected {
                    Phase::Focused
                } else {
                    Phase::Hidden
                };
                (route.clone(), phase)
            });
        // Routes were checked for repeats when they were set.
        let _unique = self.children.reconcile(desired, cx);
    }
}

impl<R: Clone + Eq + Hash + 'static, H: 'static, P: Component> Router<P> for Pages<R, H, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        self.children.router_ids()
    }

    fn destroy(self, cx: &mut Cx<'_, P>) {
        self.children.destroy(cx);
    }
}

#[cfg(feature = "serde")]
mod saved {
    use super::{Pages, check_unique};
    use crate::router::set::{ChildSet, DuplicateKey, SavedChildSet};
    use crate::{Component, Cx};
    use alloc::vec::Vec;
    use core::hash::Hash;

    /// The persistable form of [`Pages`].
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct SavedPages<R> {
        routes: Vec<R>,
        selected: usize,
        children: SavedChildSet<R>,
    }

    impl<R: Clone + Eq + Hash + 'static, H: 'static, P: Component> Pages<R, H, P> {
        /// Captures routes and selection. Component state is saved by the tree.
        pub fn snapshot(&self) -> SavedPages<R> {
            SavedPages {
                routes: self.routes.clone(),
                selected: self.selected,
                children: self.children.snapshot(),
            }
        }

        /// Rebuilds pages, or creates them from `initial` if nothing was saved.
        ///
        /// # Errors
        ///
        /// Returns [`DuplicateKey`] if a route repeats.
        pub fn restore_or(
            saved: Option<SavedPages<R>>,
            initial: Vec<R>,
            retain: usize,
            cx: &mut Cx<'_, P>,
            build: impl Fn(&R, &mut Cx<'_, P>) -> H + 'static,
        ) -> Result<Self, DuplicateKey> {
            let Some(saved) = saved else {
                return Self::new(initial, 0, retain, cx, build);
            };
            check_unique(&saved.routes)?;
            let selected = saved.selected.min(saved.routes.len().saturating_sub(1));
            let mut pages = Self {
                routes: saved.routes,
                selected,
                retain,
                children: ChildSet::restore(Some(saved.children), cx, build),
            };
            pages.sync(cx);
            // A page outside the retained range has no component and no state.
            pages.children.forget_waiting(cx);
            Ok(pages)
        }
    }
}

#[cfg(feature = "serde")]
pub use saved::SavedPages;

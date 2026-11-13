//! A long keyed list whose components exist only inside the visible window.

use super::set::{ChildSet, DuplicateKey};
use super::{Router, RouterId};
use crate::{Component, Cx, Phase};
use alloc::vec::Vec;
use core::fmt;
use core::hash::Hash;
use core::ops::Range;

/// Returns the rows on screen for a scroll offset, for UIs that report pixels.
///
/// One row past each edge is included, so a partly visible row is alive.
#[must_use]
pub fn visible_rows(offset: f32, viewport: f32, row_height: f32, len: usize) -> Range<usize> {
    if row_height <= 0.0 {
        return 0..0;
    }
    let first = (offset.max(0.0) / row_height) as usize;
    let rows = viewport.max(0.0) / row_height;
    let whole = rows as usize;
    // `f32::ceil` needs `std`.
    let count = whole
        .saturating_add(usize::from(rows > whole as f32))
        .saturating_add(1);
    let start = first.saturating_sub(1).min(len);
    start..first.saturating_add(count).min(len)
}

/// How many spare rows a list keeps alive on each side of the rows on screen.
///
/// The UI adapters use it by default. A fast scroll then reaches rows whose
/// components already exist instead of drawing placeholders for a frame.
pub const OVERSCAN: usize = 10;

/// Returns the window a list should report, or `None` if the current one still
/// covers the rows on screen.
///
/// The window is the visible rows plus `overscan` spare rows on each side,
/// within `0..len`. While the visible rows stay inside `current`, nothing is
/// reported, so scrolling through spare rows costs no message.
#[must_use]
pub fn next_window(
    visible: Range<usize>,
    current: Range<usize>,
    overscan: usize,
    len: usize,
) -> Option<Range<usize>> {
    let end = visible.end.min(len);
    let start = visible.start.min(end);
    let covered = current.start <= start && end <= current.end;
    if covered && current.end <= len {
        return None;
    }
    let window = start.saturating_sub(overscan)..end.saturating_add(overscan).min(len);
    (window != current).then_some(window)
}

/// A list held as a field of its parent `P`.
///
/// Every key is known, but only keys inside the window have a component. The
/// UI reports what is on screen with [`Items::set_window`], so the cost of a
/// list follows the screen and not its length.
///
/// ```
/// use compono::router::items::Items;
/// use compono::{Component, Cx, Handle, Tree};
/// use core::ops::Range;
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
/// struct Feed {
///     items: Items<u32, Handle<Screen>, Feed>,
/// }
///
/// impl Component for Feed {
///     /// The rows on screen, as the UI reports them while it scrolls.
///     type Msg = Range<usize>;
///     type Output = ();
///
///     fn update(&mut self, window: Range<usize>, cx: &mut Cx<'_, Self>) {
///         // `Ok(false)` when the window is the one already shown, and
///         // `Err(DuplicateKey)` when a key repeats inside it.
///         if self.items.set_window(window, cx) != Ok(true) {
///             cx.unchanged();
///         }
///     }
/// }
///
/// let mut tree = Tree::new(Quiet);
/// let feed = tree.mount(|cx| Feed {
///     items: Items::new((0..100_000).collect(), cx, |_post, cx| screen(cx)),
/// });
/// tree.send(feed, 10..40)?;
/// let items = tree.get(feed).map(|feed| &feed.items);
/// assert_eq!(items.map(Items::len), Some(100_000));
/// assert_eq!(items.map(Items::live), Some(30));
/// assert!(items.is_some_and(|items| items.child(10).is_some()));
/// assert!(items.is_some_and(|items| items.child(50).is_none()));
/// # Ok::<(), compono::StaleHandle>(())
/// ```
pub struct Items<K, H, P> {
    keys: Vec<K>,
    window: Range<usize>,
    children: ChildSet<K, H, P>,
}

impl<K, H, P> fmt::Debug for Items<K, H, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Items")
            .field("keys", &self.keys.len())
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

impl<K: Clone + Eq + Hash + 'static, H: 'static, P: Component> Items<K, H, P> {
    /// Creates a list with an empty window.
    pub fn new(
        keys: Vec<K>,
        cx: &mut Cx<'_, P>,
        build: impl Fn(&K, &mut Cx<'_, P>) -> H + 'static,
    ) -> Self {
        Self {
            keys,
            window: 0..0,
            children: ChildSet::new(cx, build),
        }
    }

    /// Returns the number of keys.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.keys.len()
    }

    /// Returns whether the list has no keys.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Returns the key at `index`.
    #[must_use]
    pub fn key(&self, index: usize) -> Option<&K> {
        self.keys.get(index)
    }

    /// Returns the current window.
    #[must_use]
    pub fn window(&self) -> Range<usize> {
        self.window.clone()
    }

    /// Returns the value at `index` if it is inside the window.
    #[must_use]
    pub fn child(&self, index: usize) -> Option<&H> {
        let position = index.checked_sub(self.window.start)?;
        self.children.child_at(position)
    }

    /// Returns the number of live components.
    #[must_use]
    pub const fn live(&self) -> usize {
        self.children.len()
    }

    /// Returns index, key and value of each live row.
    pub fn visible(&self) -> impl Iterator<Item = (usize, &K, &H)> {
        let start = self.window.start;
        self.children
            .iter()
            .enumerate()
            .map(move |(offset, (key, child, _phase))| (start + offset, key, child))
    }

    /// Shows `window`, clamped to the list. Returns whether anything changed.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateKey`] without changing anything if a key repeats
    /// inside the window.
    pub fn set_window(
        &mut self,
        window: Range<usize>,
        cx: &mut Cx<'_, P>,
    ) -> Result<bool, DuplicateKey> {
        let end = window.end.min(self.keys.len());
        let window = window.start.min(end)..end;
        if window == self.window {
            return Ok(false);
        }
        let shown = self.keys.get(window.clone()).unwrap_or(&[]);
        let desired = shown.iter().map(|key| (key.clone(), Phase::Visible));
        self.children.reconcile(desired, cx)?;
        self.window = window;
        Ok(true)
    }

    /// Replaces the keys. Components of keys still in the window survive.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateKey`] without changing anything if a key repeats
    /// inside the window.
    pub fn set_keys(&mut self, keys: Vec<K>, cx: &mut Cx<'_, P>) -> Result<(), DuplicateKey> {
        let end = self.window.end.min(keys.len());
        let window = self.window.start.min(end)..end;
        let shown = keys.get(window.clone()).unwrap_or(&[]);
        let desired = shown.iter().map(|key| (key.clone(), Phase::Visible));
        self.children.reconcile(desired, cx)?;
        self.keys = keys;
        self.window = window;
        Ok(())
    }
}

impl<K: Clone + Eq + Hash + 'static, H: 'static, P: Component> Router<P> for Items<K, H, P> {
    fn router_ids(&self) -> impl IntoIterator<Item = RouterId> {
        self.children.router_ids()
    }

    fn destroy(self, cx: &mut Cx<'_, P>) {
        self.children.destroy(cx);
    }
}

#[cfg(feature = "serde")]
mod saved {
    use super::Items;
    use crate::router::set::{ChildSet, DuplicateKey, SavedChildSet, check_unique};
    use crate::{Component, Cx};
    use alloc::vec::Vec;
    use core::hash::Hash;

    /// The persistable form of [`Items`]: the window and the rows inside it.
    ///
    /// The keys are not saved. The parent supplies them again, as it did at first.
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct SavedItems<K> {
        window: (usize, usize),
        children: SavedChildSet<K>,
    }

    impl<K: Clone + Eq + Hash + 'static, H: 'static, P: Component> Items<K, H, P> {
        /// Captures the window. Component state is saved by the tree.
        pub fn snapshot(&self) -> SavedItems<K> {
            SavedItems {
                window: (self.window.start, self.window.end),
                children: self.children.snapshot(),
            }
        }

        /// Rebuilds a list over `keys`, or creates one with an empty window if
        /// nothing was saved.
        ///
        /// The saved window is clamped to `keys`. A row that is no longer inside
        /// it loses its saved state, as a row does when it scrolls away.
        ///
        /// # Errors
        ///
        /// Returns [`DuplicateKey`] without changing anything if a key repeats
        /// inside the window. The list then has taken no identity in saved
        /// state, so the owner can build another router in its place and the
        /// routers after it keep their own state.
        pub fn restore_or(
            saved: Option<SavedItems<K>>,
            keys: Vec<K>,
            cx: &mut Cx<'_, P>,
            build: impl Fn(&K, &mut Cx<'_, P>) -> H + 'static,
        ) -> Result<Self, DuplicateKey> {
            let Some(saved) = saved else {
                return Ok(Self::new(keys, cx, build));
            };
            // Checked before the list takes its identity in saved state.
            let end = saved.window.1.min(keys.len());
            let start = saved.window.0.min(end);
            check_unique(keys.get(start..end).unwrap_or(&[]))?;
            let mut items = Self {
                keys,
                window: 0..0,
                children: ChildSet::restore(Some(saved.children), cx, build),
            };
            let shown = items.set_window(saved.window.0..saved.window.1, cx);
            items.children.forget_waiting(cx);
            shown.map(|_changed| items)
        }
    }
}

#[cfg(feature = "serde")]
pub use saved::SavedItems;

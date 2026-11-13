//! Draws routers. Every UI adapter offers [`RouterView`] and [`ListView`].

use crate::{TreeView, update};
use compono::router::items::{OVERSCAN, next_window};
use compono::router::pages::Pages;
use compono::router::panels::{Panel, PanelTypes, Panels};
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono::{Component, Handle};
use core::cell::Cell;
use core::fmt;
use core::hash::Hash;
use core::ops::Range;
use gpui::prelude::*;
use gpui::{AnyElement, Context, ElementId, UniformList, div, px, uniform_list};
use std::rc::Rc;

/// Builds what a router currently shows.
pub trait RouterView<'a> {
    /// What the content receives. A router with several visible children calls
    /// the content once for each.
    type Child;
    /// The result: `Option` when the router can be empty.
    type Output;

    /// Builds the router's visible children.
    fn view(&'a self, content: impl FnMut(Self::Child) -> AnyElement) -> Self::Output;
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Stack<R, H, P> {
    type Child = &'a H;
    type Output = AnyElement;

    fn view(&'a self, mut content: impl FnMut(&'a H) -> AnyElement) -> AnyElement {
        content(self.active())
    }
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Slot<R, H, P> {
    type Child = &'a H;
    type Output = Option<AnyElement>;

    fn view(&'a self, content: impl FnMut(&'a H) -> AnyElement) -> Option<AnyElement> {
        self.active().map(content)
    }
}

impl<'a, R, H: 'static, P: Component> RouterView<'a> for Pages<R, H, P>
where
    R: Clone + Eq + Hash + 'static,
{
    type Child = &'a H;
    type Output = Option<AnyElement>;

    fn view(&'a self, content: impl FnMut(&'a H) -> AnyElement) -> Option<AnyElement> {
        self.active().map(content)
    }
}

impl<'a, T: PanelTypes, P: Component> RouterView<'a> for Panels<T, P> {
    type Child = Panel<'a, T>;
    type Output = AnyElement;

    fn view(&'a self, mut content: impl FnMut(Panel<'a, T>) -> AnyElement) -> AnyElement {
        div()
            .flex()
            .gap_3()
            .size_full()
            .children(
                self.visible()
                    .map(|panel| div().flex_1().h_full().min_w(px(0.0)).child(content(panel))),
            )
            .into_any_element()
    }
}

/// A long list that reports the rows on screen to its owner.
pub struct ListView<C, F> {
    id: ElementId,
    len: usize,
    target: Handle<C>,
    to_msg: F,
    current: Range<usize>,
    overscan: usize,
}

impl<C, F> fmt::Debug for ListView<C, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListView")
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}

impl<C, F> ListView<C, F>
where
    C: Component,
    F: Fn(Range<usize>) -> C::Msg + Copy + 'static,
{
    /// Creates a list of `len` rows that tells `target` which are on screen.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, len: usize, target: Handle<C>, to_msg: F) -> Self {
        Self {
            id: id.into(),
            len,
            target,
            to_msg,
            current: 0..0,
            overscan: OVERSCAN,
        }
    }

    /// Sets the window the list's `Items` holds now, from `Items::window`.
    /// Without it, every draw reports a window.
    #[must_use]
    pub fn window(mut self, current: Range<usize>) -> Self {
        self.current = current;
        self
    }

    /// Sets how many spare rows stay alive on each side of the rows on screen.
    /// [`OVERSCAN`] by default.
    #[must_use]
    pub const fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    /// Builds the list. GPUI asks for rows by range while it draws.
    pub fn view<V: TreeView>(
        self,
        cx: &Context<'_, V>,
        row: impl Fn(&mut V, usize, &mut Context<'_, V>) -> AnyElement + 'static,
    ) -> UniformList {
        let Self {
            id,
            len,
            target,
            to_msg,
            current,
            overscan,
        } = self;
        let latest: Rc<Cell<Option<Range<usize>>>> = Rc::default();
        uniform_list(
            id,
            len,
            cx.processor(move |view, range: Range<usize>, window, cx| {
                // A draw also asks for one row to measure its height. The range
                // on screen comes last, so only that one is reported.
                if latest.replace(Some(range.clone())).is_none() {
                    let latest = Rc::clone(&latest);
                    let current = current.clone();
                    cx.defer_in(window, move |view, _window, cx| {
                        let Some(window) = latest
                            .take()
                            .and_then(|shown| next_window(shown, current.clone(), overscan, len))
                        else {
                            return;
                        };
                        update(view, cx, |tree| {
                            // The list outlives the component by at most one frame.
                            let _stale_is_fine = tree.send(target, to_msg(window));
                        });
                    });
                }
                range.map(|index| row(view, index, cx)).collect()
            }),
        )
    }
}

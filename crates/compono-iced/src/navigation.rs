//! Draws routers. Every UI adapter offers [`RouterView`] and [`ListView`].

use crate::{Action, send};
use compono::router::items::{Items, OVERSCAN, next_window, visible_rows};
use compono::router::pages::Pages;
use compono::router::panels::{Panel, PanelTypes, Panels};
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono::{Component, Handle};
use core::fmt;
use core::hash::Hash;
use core::ops::Range;
use iced::widget::{Column, Row, column, container, scrollable, space};
use iced::{Element, Fill};

type View<'a> = Element<'a, Action>;

/// Builds what a router currently shows.
pub trait RouterView<'a> {
    /// What the content receives. A router with several visible children calls
    /// the content once for each.
    type Child;
    /// The result: `Option` when the router can be empty.
    type Output;

    /// Builds the router's visible children.
    fn view(&'a self, content: impl FnMut(Self::Child) -> View<'a>) -> Self::Output;
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Stack<R, H, P> {
    type Child = &'a H;
    type Output = View<'a>;

    fn view(&'a self, mut content: impl FnMut(&'a H) -> View<'a>) -> View<'a> {
        content(self.active())
    }
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Slot<R, H, P> {
    type Child = &'a H;
    type Output = Option<View<'a>>;

    fn view(&'a self, content: impl FnMut(&'a H) -> View<'a>) -> Option<View<'a>> {
        self.active().map(content)
    }
}

impl<'a, R, H: 'static, P: Component> RouterView<'a> for Pages<R, H, P>
where
    R: Clone + Eq + Hash + 'static,
{
    type Child = &'a H;
    type Output = Option<View<'a>>;

    fn view(&'a self, content: impl FnMut(&'a H) -> View<'a>) -> Option<View<'a>> {
        self.active().map(content)
    }
}

impl<'a, T: PanelTypes, P: Component> RouterView<'a> for Panels<T, P> {
    type Child = Panel<'a, T>;
    type Output = View<'a>;

    fn view(&'a self, mut content: impl FnMut(Panel<'a, T>) -> View<'a>) -> View<'a> {
        self.visible()
            .fold(Row::new().spacing(12), |row, panel| {
                row.push(container(content(panel)).width(Fill).height(Fill))
            })
            .into()
    }
}

/// A long list that reports the rows on screen to its owner.
pub struct ListView<'a, K, H, P, C, F> {
    items: &'a Items<K, H, P>,
    target: Handle<C>,
    to_msg: F,
    row_height: f32,
    overscan: usize,
}

impl<K, H, P, C, F> fmt::Debug for ListView<'_, K, H, P, C, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListView")
            .field("row_height", &self.row_height)
            .finish_non_exhaustive()
    }
}

impl<'a, K, H, P, C, F> ListView<'a, K, H, P, C, F>
where
    K: Clone + Eq + Hash + 'static,
    H: 'static,
    P: Component,
    C: Component,
    C::Msg: Clone + Send + Sync,
    F: Fn(Range<usize>) -> C::Msg + 'a,
{
    /// Creates a list that tells `target` which rows are on screen.
    #[must_use]
    pub const fn new(items: &'a Items<K, H, P>, target: Handle<C>, to_msg: F) -> Self {
        Self {
            items,
            target,
            to_msg,
            row_height: 24.0,
            overscan: OVERSCAN,
        }
    }

    /// Sets how many spare rows stay alive on each side of the rows on screen.
    /// [`OVERSCAN`] by default.
    #[must_use]
    pub const fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    /// Sets the height of every row.
    #[must_use]
    pub const fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    /// Builds the list.
    ///
    /// A row is `None` until its component exists.
    pub fn view(self, row: impl Fn(usize, Option<&'a H>) -> View<'a>) -> View<'a> {
        let Self {
            items,
            target,
            to_msg,
            row_height,
            overscan,
        } = self;
        let window = items.window();
        let current = window.clone();
        let len = items.len();
        let before = window.start as f32 * row_height;
        let after = len.saturating_sub(window.end) as f32 * row_height;
        let rows = window.fold(Column::new(), |list, index| {
            list.push(row(index, items.child(index)))
        });
        let content = column![space().height(before), rows, space().height(after)];
        scrollable(content)
            .spacing(4)
            .width(Fill)
            .height(Fill)
            .on_scroll(move |viewport| {
                let offset = viewport.absolute_offset().y;
                let height = viewport.bounds().height;
                let visible = visible_rows(offset, height, row_height, len);
                match next_window(visible, current.clone(), overscan, len) {
                    Some(window) => send(target, to_msg(window)),
                    // Scrolling through spare rows changes nothing. An empty pump
                    // makes no commit.
                    None => Action::Pump,
                }
            })
            .into()
    }
}

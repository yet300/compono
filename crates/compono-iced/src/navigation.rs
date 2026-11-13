//! Draws routers. Every UI adapter offers [`RouterView`] and [`ListView`].

use crate::motion::{self, Animated, Rule};
use crate::{Action, send};
use compono::router::items::{Items, OVERSCAN, next_window, visible_rows};
use compono::router::pages::Pages;
use compono::router::panels::{Panel, PanelTypes, Panels, PanelsMode};
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

    /// Builds the active entry, moving in with the application's motion.
    fn view(&'a self, mut content: impl FnMut(&'a H) -> View<'a>) -> View<'a> {
        let keys = self.entries().map(|(key, _route)| key).collect();
        Animated::new(
            content(self.active()),
            Rule::Stack(keys),
            self.active_key(),
            motion::preferred(motion::app_motion()),
        )
        .into()
    }
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Slot<R, H, P> {
    type Child = &'a H;
    type Output = Option<View<'a>>;

    /// Builds the entry, fading in when it opens.
    fn view(&'a self, content: impl FnMut(&'a H) -> View<'a>) -> Option<View<'a>> {
        let key = self.active_key()?;
        let child = self.active().map(content)?;
        let motion = motion::preferred(motion::slot_motion(motion::app_motion()));
        Some(Animated::new(child, Rule::Slot, key, motion).into())
    }
}

impl<'a, R, H: 'static, P: Component> RouterView<'a> for Pages<R, H, P>
where
    R: Clone + Eq + Hash + 'static,
{
    type Child = &'a H;
    type Output = Option<View<'a>>;

    /// Builds the selected page, sliding in by page order.
    fn view(&'a self, content: impl FnMut(&'a H) -> View<'a>) -> Option<View<'a>> {
        let child = self.active().map(content)?;
        let motion = motion::preferred(motion::page_motion(motion::app_motion()));
        Some(Animated::new(child, Rule::Pages, self.selected() as u64, motion).into())
    }
}

impl<'a, T: PanelTypes, P: Component> RouterView<'a> for Panels<T, P> {
    type Child = Panel<'a, T>;
    type Output = View<'a>;

    /// Builds the visible panels side by side. When one fits, the deepest panel
    /// slides in over the one it covers.
    fn view(&'a self, mut content: impl FnMut(Panel<'a, T>) -> View<'a>) -> View<'a> {
        let row = self.visible().fold(Row::new().spacing(12), |row, panel| {
            row.push(container(content(panel)).width(Fill).height(Fill))
        });
        let single = self.mode() == PanelsMode::Single;
        let key = motion::panel_key(self.keys(), self.shown());
        let motion = motion::preferred(motion::panel_motion(motion::app_motion()));
        Animated::new(row, Rule::Panels { single }, key, motion).into()
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

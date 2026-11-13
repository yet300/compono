//! Draws routers. Every UI adapter offers [`RouterView`] and [`ListView`].

use crate::Outbox;
use compono::router::items::{Items, OVERSCAN, next_window};
use compono::router::pages::Pages;
use compono::router::panels::{Panel, PanelTypes, Panels};
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono::{Component, Handle};
use core::fmt;
use core::hash::Hash;
use core::ops::Range;
use egui::{ScrollArea, Ui};

/// Draws what a router currently shows.
pub trait RouterView<'a> {
    /// What the content receives. A router with several visible children calls
    /// the content once for each.
    type Child;

    /// Draws the router's visible children.
    fn view(&'a self, ui: &mut Ui, content: impl FnMut(&mut Ui, Self::Child));
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Stack<R, H, P> {
    type Child = &'a H;

    fn view(&'a self, ui: &mut Ui, mut content: impl FnMut(&mut Ui, &'a H)) {
        ui.push_id(self.active_key(), |ui| content(ui, self.active()));
    }
}

impl<'a, R: 'static, H: 'static, P: Component> RouterView<'a> for Slot<R, H, P> {
    type Child = &'a H;

    fn view(&'a self, ui: &mut Ui, mut content: impl FnMut(&mut Ui, &'a H)) {
        if let Some(child) = self.active() {
            content(ui, child);
        }
    }
}

impl<'a, R, H: 'static, P: Component> RouterView<'a> for Pages<R, H, P>
where
    R: Clone + Eq + Hash + 'static,
{
    type Child = &'a H;

    fn view(&'a self, ui: &mut Ui, mut content: impl FnMut(&mut Ui, &'a H)) {
        if let Some(child) = self.active() {
            ui.push_id(self.selected(), |ui| content(ui, child));
        }
    }
}

impl<'a, T: PanelTypes, P: Component> RouterView<'a> for Panels<T, P> {
    type Child = Panel<'a, T>;

    fn view(&'a self, ui: &mut Ui, mut content: impl FnMut(&mut Ui, Panel<'a, T>)) {
        ui.columns(self.visible().count().max(1), |columns| {
            for (ui, panel) in columns.iter_mut().zip(self.visible()) {
                content(ui, panel);
            }
        });
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
    F: FnOnce(Range<usize>) -> C::Msg,
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

    /// Draws the rows on screen.
    ///
    /// A row is `None` for the one frame before its component exists.
    pub fn view(
        self,
        ui: &mut Ui,
        out: &mut Outbox,
        mut row: impl FnMut(&mut Ui, &mut Outbox, usize, Option<&'a H>),
    ) {
        let Self {
            items,
            target,
            to_msg,
            row_height,
            overscan,
        } = self;
        ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            row_height,
            items.len(),
            |ui, range| {
                if let Some(window) =
                    next_window(range.clone(), items.window(), overscan, items.len())
                {
                    out.send(target, to_msg(window));
                }
                for index in range {
                    row(ui, out, index, items.child(index));
                }
            },
        );
    }
}

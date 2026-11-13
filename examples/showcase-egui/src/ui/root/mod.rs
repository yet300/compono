use super::{book, gallery, tabs};
use compono::{Handle, Tree};
use compono_egui::{Outbox, RouterView};
use egui::Ui;
use showcase::feature::root::{Child, Root};

/// Draws whichever screen is active.
pub fn root(ui: &mut Ui, tree: &Tree, root: Handle<Root>, out: &mut Outbox) {
    let Some(root) = tree.get(root) else {
        return;
    };
    root.stack.view(ui, |ui, child| match *child {
        Child::Tabs(tabs) => tabs::view(ui, tree, tabs, out),
        Child::Book(book) => book::view(ui, tree, book, out),
        Child::Gallery(gallery) => gallery::view(ui, tree, gallery, out),
    });
}

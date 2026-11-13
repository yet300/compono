use super::widgets::View;
use super::{book, gallery, tabs};
use compono::{Handle, Tree};
use compono_iced::RouterView;
use iced::widget::space;
use showcase::feature::root::{Child, Root};

/// Builds whichever screen is active.
#[must_use]
pub fn root(tree: &Tree, root: Handle<Root>) -> View<'_> {
    let Some(root) = tree.get(root) else {
        return space().into();
    };
    root.stack.view(|child| match *child {
        Child::Tabs(tabs) => tabs::view(tree, tabs),
        Child::Book(book) => book::view(tree, book),
        Child::Gallery(gallery) => gallery::view(tree, gallery),
    })
}

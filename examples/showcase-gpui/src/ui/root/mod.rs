use super::widgets::{Cx, fill};
use super::{book, gallery, tabs};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::RouterView;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::ActiveTheme;
use showcase::feature::root::{Child, Root};

/// Builds whichever screen is active, each as a view of its own.
pub fn root(owner: &mut ShowcaseView, root: Handle<Root>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(root) = tree.get(root) else {
        return div().into_any_element();
    };
    let body = root.stack.view(|child| {
        match *child {
            Child::Tabs(tabs) => views.view(tabs, tabs::view, cx),
            Child::Book(book) => views.view(book, book::view, cx),
            Child::Gallery(gallery) => views.view(gallery, gallery::view, cx),
        }
        .cached(fill())
        .into_any_element()
    });
    let theme = cx.theme();
    div()
        .size_full()
        .bg(theme.background)
        .text_color(theme.foreground)
        .child(body)
        .into_any_element()
}

mod dialog;

use super::widgets::{Cx, action, fill, label, primary};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::{RouterView, send};
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::v_flex;
use showcase::feature::menu::{Menu, MenuMsg};

/// Links to the other screens, and a dialog held in a slot.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Menu>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(menu) = tree.get(handle) else {
        return div().into_any_element();
    };
    let dialog = menu.dialog.view(|child| {
        views
            .view(*child, dialog::view, cx)
            .cached(fill())
            .into_any_element()
    });
    div()
        .relative()
        .size_full()
        .child(
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .child(label("Showcase").text_3xl())
                .child(primary("Open pages", send(handle, MenuMsg::OpenBook, cx)))
                .child(action(
                    "Open custom navigation",
                    send(handle, MenuMsg::OpenGallery, cx),
                ))
                .child(action("Show dialog", send(handle, MenuMsg::ShowDialog, cx))),
        )
        .children(dialog)
        .into_any_element()
}

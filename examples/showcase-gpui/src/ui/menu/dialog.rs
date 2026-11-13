use super::super::widgets::{Cx, label, modal, primary};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::h_flex;
use showcase::feature::menu::{Dialog, DialogMsg};

/// A dialog whose text and dismissal live in its component.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Dialog>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(dialog) = tree.get(handle) else {
        return div().into_any_element();
    };
    let text = dialog.text;
    modal(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(label("Slot").text_lg())
            .child(label(text))
            .child(
                h_flex()
                    .justify_end()
                    .child(primary("Dismiss", send(handle, DialogMsg::Dismiss, cx))),
            ),
        cx,
    )
    .into_any_element()
}

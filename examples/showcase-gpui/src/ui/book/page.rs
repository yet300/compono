use super::super::widgets::{Cx, card, label, primary};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::v_flex;
use showcase::feature::book::{Page, PageMsg};

/// One page, which keeps its tap count while it stays alive.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Page>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(page) = tree.get(handle) else {
        return div().into_any_element();
    };
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .child(
            card(label(format!("Page {}", page.number)).text_3xl()).child(
                v_flex()
                    .gap_3()
                    .child(label(format!("Tapped {} times", page.taps)))
                    .child(primary("Tap", send(handle, PageMsg::Tap, cx))),
            ),
        )
        .into_any_element()
}

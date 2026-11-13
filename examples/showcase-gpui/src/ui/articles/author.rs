use super::super::widgets::{Cx, action, card, label};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::v_flex;
use showcase::feature::articles::{AuthorMsg, AuthorPanel};

/// The author of the open article.
pub fn view(owner: &mut ShowcaseView, handle: Handle<AuthorPanel>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(author) = tree.get(handle) else {
        return div().into_any_element();
    };
    card(label("Author").text_sm())
        .child(
            v_flex()
                .gap_3()
                .child(label(author.author.name.clone()).text_xl())
                .child(action("Close author", send(handle, AuthorMsg::Close, cx))),
        )
        .into_any_element()
}

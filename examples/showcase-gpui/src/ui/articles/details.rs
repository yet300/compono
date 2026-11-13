use super::super::widgets::{Cx, action, card, label};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::{h_flex, v_flex};
use showcase::feature::articles::{ArticleDetails, DetailsMsg};

/// An article. Its author opens in the extra panel.
pub fn view(
    owner: &mut ShowcaseView,
    handle: Handle<ArticleDetails>,
    cx: &mut Cx<'_>,
) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(details) = tree.get(handle) else {
        return div().into_any_element();
    };
    let article = &details.article;
    v_flex()
        .id("details")
        .size_full()
        .overflow_y_scroll()
        .child(
            card(label(article.title.clone()).text_xl()).child(
                v_flex().gap_3().child(article.body.clone()).child(
                    h_flex()
                        .gap_2()
                        .child(action("Author", send(handle, DetailsMsg::ShowAuthor, cx)))
                        .child(action("Close", send(handle, DetailsMsg::Close, cx))),
                ),
            ),
        )
        .into_any_element()
}

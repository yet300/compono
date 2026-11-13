mod row;

use super::widgets::{Cx, label};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::{ListView, send};
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex};
use showcase::feature::feed::{Feed, FeedMsg};

/// 100 000 rows. Only the rows on screen have components.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Feed>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(feed) = tree.get(handle) else {
        return div().into_any_element();
    };
    let status = format!("{} rows, {} live", feed.items.len(), feed.items.live());
    let live = Switch::new("live")
        .label("Live updates")
        .checked(feed.live)
        .on_click(send(handle, FeedMsg::ToggleLive, cx));
    let list = ListView::new("feed", feed.items.len(), handle, FeedMsg::Window)
        .window(feed.items.window())
        .view(cx, move |owner: &mut ShowcaseView, index, cx| {
            row::view(owner, handle, index, cx)
        })
        .size_full();
    v_flex()
        .size_full()
        .gap_3()
        .child(
            h_flex()
                .justify_between()
                .child(label(status).text_2xl())
                .child(live),
        )
        .child(div().flex_1().min_h_0().child(list))
        .into_any_element()
}

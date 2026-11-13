use super::super::widgets::Cx;
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div, px};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, Sizable, h_flex};
use showcase::feature::feed::{Feed, RowMsg};

/// The height of every row.
pub const HEIGHT: f32 = 36.0;

/// One row. A changed row redraws the feed's view, which holds about 20 rows.
pub fn view(
    owner: &mut ShowcaseView,
    feed: Handle<Feed>,
    index: usize,
    cx: &mut Cx<'_>,
) -> AnyElement {
    let (tree, _views) = owner.parts();
    let row = tree
        .get(feed)
        .and_then(|feed| feed.items.child(index).copied());
    let Some((row, state)) = row.and_then(|row| tree.get(row).map(|state| (row, state))) else {
        return div()
            .w_full()
            .h(px(HEIGHT))
            .text_color(cx.theme().muted_foreground)
            .child(format!("Row {index}"))
            .into_any_element();
    };
    let like = Button::new(("like", state.id))
        .small()
        .label(if state.liked { "Liked" } else { "Like" })
        .on_click(send(row, RowMsg::Like, cx));
    h_flex()
        .w_full()
        .h(px(HEIGHT))
        .justify_between()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(format!("Row {}  value {}", state.id, state.value))
        .child(if state.liked {
            like.primary()
        } else {
            like.ghost()
        })
        .into_any_element()
}

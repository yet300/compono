use super::super::widgets::Cx;
use compono::{Handle, Tree};
use gpui::prelude::*;
use gpui::{AnyElement, px};
use gpui_component::tag::Tag;
use gpui_component::{ActiveTheme, v_flex};
use showcase::feature::gallery::Photo;

/// One photo, which develops while it is visible. A tag shows how far.
pub fn view(
    tree: &Tree,
    key: u64,
    handle: Handle<Photo>,
    selected: bool,
    cx: &Cx<'_>,
) -> AnyElement {
    let developed = tree.get(handle).map_or(0, |photo| photo.developed);
    let theme = cx.theme();
    let tag = if selected {
        Tag::primary()
    } else {
        Tag::secondary()
    };
    v_flex()
        .w(px(88.0))
        .h(px(72.0))
        .p_2()
        .gap_1()
        .rounded(theme.radius)
        .border_1()
        .border_color(if selected {
            theme.primary
        } else {
            theme.border
        })
        .bg(theme.secondary)
        .child(format!("#{key}"))
        .child(tag.child(developed.to_string()))
        .into_any_element()
}

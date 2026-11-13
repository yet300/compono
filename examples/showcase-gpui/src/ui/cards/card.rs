use super::super::widgets::Cx;
use compono::{Handle, Phase, Tree};
use gpui::AnyElement;
use gpui::prelude::*;
use gpui_component::tag::Tag;
use gpui_component::{ActiveTheme, h_flex};
use showcase::feature::cards::Card;

/// One card. A tag shows its phase, and the focused card is highlighted.
pub fn view(
    tree: &Tree,
    number: u32,
    handle: Handle<Card>,
    phase: Phase,
    cx: &Cx<'_>,
) -> AnyElement {
    let value = tree.get(handle).map_or(0, |card| card.value);
    let theme = cx.theme();
    let focused = phase == Phase::Focused;
    let tag = if focused {
        Tag::primary()
    } else {
        Tag::secondary()
    };
    h_flex()
        .w_full()
        .p_3()
        .gap_3()
        .justify_between()
        .rounded(theme.radius)
        .border_1()
        .border_color(if focused { theme.primary } else { theme.border })
        .bg(theme.secondary)
        .child(format!("Card {number}: {value}"))
        .child(tag.child(format!("{phase:?}")))
        .into_any_element()
}

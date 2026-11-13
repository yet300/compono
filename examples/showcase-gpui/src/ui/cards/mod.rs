mod card;

use super::widgets::{Cx, action, primary, title};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::v_flex;
use showcase::feature::cards::{Cards, CardsMsg};

/// A pile of cards kept in any order. Only the top card ticks.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Cards>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(cards) = tree.get(handle) else {
        return div().into_any_element();
    };
    let mut pile: Vec<AnyElement> = cards
        .cards
        .iter()
        .map(|(number, child, phase)| card::view(tree, *number, *child, phase, cx))
        .collect();
    pile.reverse();
    v_flex()
        .size_full()
        .gap_3()
        .child(title(
            "Cards",
            [
                action(
                    "Send top to back",
                    send(handle, CardsMsg::SendTopToBack, cx),
                ),
                action("Remove", send(handle, CardsMsg::Remove, cx)),
                primary("Add", send(handle, CardsMsg::Add, cx)),
            ],
        ))
        .child(
            v_flex()
                .id("cards")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .gap_2()
                .children(pile),
        )
        .into_any_element()
}

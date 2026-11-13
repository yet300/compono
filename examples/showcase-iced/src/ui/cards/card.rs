use super::super::widgets::View;
use compono::{Handle, Phase, Tree};
use iced::Fill;
use iced::widget::{container, text};
use showcase::feature::cards::Card;

/// One card. The focused card uses the primary colour.
pub fn view(tree: &Tree, number: u32, handle: Handle<Card>, phase: Phase) -> View<'_> {
    let value = tree.get(handle).map_or(0, |card| card.value);
    container(text(format!("Card {number}: {value} ({phase:?})")))
        .padding(14)
        .width(Fill)
        .style(if phase == Phase::Focused {
            container::primary
        } else {
            container::rounded_box
        })
        .into()
}

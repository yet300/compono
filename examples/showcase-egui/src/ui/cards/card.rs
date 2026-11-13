use super::super::widgets::card;
use compono::{Handle, Phase, Tree};
use egui::{RichText, Ui};
use showcase::feature::cards::Card;

/// One card. The focused card is highlighted.
pub fn view(ui: &mut Ui, tree: &Tree, number: u32, handle: Handle<Card>, phase: Phase) {
    let value = tree.get(handle).map_or(0, |card| card.value);
    card(ui, |ui| {
        ui.set_width(ui.available_width());
        let text = RichText::new(format!("Card {number}: {value} ({phase:?})"));
        let text = if phase == Phase::Focused {
            text.strong()
        } else {
            text
        };
        ui.label(text);
    });
}

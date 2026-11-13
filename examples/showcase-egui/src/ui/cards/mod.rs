mod card;

use super::widgets::title;
use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{ScrollArea, Ui};
use showcase::feature::cards::{Cards, CardsMsg};

/// A pile of cards kept in any order. Only the top card ticks.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Cards>, out: &mut Outbox) {
    let Some(cards) = tree.get(handle) else {
        return;
    };
    title(ui, "Cards", |ui| {
        for (label, msg) in [
            ("Send top to back", CardsMsg::SendTopToBack),
            ("Remove", CardsMsg::Remove),
            ("Add", CardsMsg::Add),
        ] {
            if ui.button(label).clicked() {
                out.send(handle, msg);
            }
        }
    });
    ScrollArea::vertical()
        .id_salt(module_path!())
        .auto_shrink(false)
        .show(ui, |ui| {
            for (number, child, phase) in cards.cards.iter().collect::<Vec<_>>().into_iter().rev() {
                card::view(ui, tree, *number, *child, phase);
                ui.add_space(6.0);
            }
        });
}

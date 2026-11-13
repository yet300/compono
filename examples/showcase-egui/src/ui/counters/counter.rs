use super::super::widgets::card;
use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{RichText, Ui};
use showcase::feature::counters::{Counter, CounterMsg};

/// One counter. It ticks only while it is the focused one.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Counter>, out: &mut Outbox) {
    let Some(counter) = tree.get(handle) else {
        return;
    };
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        card(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("Counter {}", counter.index)).heading());
                ui.label(RichText::new(format!("Value {}", counter.value)).size(32.0));
                ui.horizontal(|ui| {
                    if ui.button("Previous").clicked() {
                        out.send(handle, CounterMsg::Previous);
                    }
                    if ui.button("Next").clicked() {
                        out.send(handle, CounterMsg::Next);
                    }
                });
            });
        });
    });
}

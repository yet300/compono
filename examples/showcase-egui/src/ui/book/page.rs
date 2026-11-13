use super::super::widgets::card;
use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{RichText, Ui};
use showcase::feature::book::{Page, PageMsg};

/// One page, which keeps its tap count while it stays alive.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Page>, out: &mut Outbox) {
    let Some(page) = tree.get(handle) else {
        return;
    };
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        card(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("Page {}", page.number)).size(32.0));
                ui.label(format!("Tapped {} times", page.taps));
                if ui.button("Tap").clicked() {
                    out.send(handle, PageMsg::Tap);
                }
            });
        });
    });
}

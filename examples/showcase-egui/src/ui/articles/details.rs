use super::super::widgets::card;
use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{RichText, ScrollArea, Ui};
use showcase::feature::articles::{ArticleDetails, DetailsMsg};

/// An article. Its author opens in the extra panel.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<ArticleDetails>, out: &mut Outbox) {
    let Some(details) = tree.get(handle) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(&details.article.title).heading());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Close").clicked() {
                out.send(handle, DetailsMsg::Close);
            }
            if ui.button("Author").clicked() {
                out.send(handle, DetailsMsg::ShowAuthor);
            }
        });
    });
    ScrollArea::vertical()
        .id_salt("article details")
        .show(ui, |ui| {
            card(ui, |ui| ui.label(&details.article.body));
        });
}

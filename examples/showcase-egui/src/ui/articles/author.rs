use super::super::widgets::card;
use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{RichText, Ui};
use showcase::feature::articles::{AuthorMsg, AuthorPanel};

/// The author of the open article.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<AuthorPanel>, out: &mut Outbox) {
    let Some(author) = tree.get(handle) else {
        return;
    };
    card(ui, |ui| {
        ui.label(RichText::new(&author.author.name).heading());
        if ui.button("Close author").clicked() {
            out.send(handle, AuthorMsg::Close);
        }
    });
}

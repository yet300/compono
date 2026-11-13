use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{Modal, RichText, Ui};
use showcase::feature::menu::{Dialog, DialogMsg};

/// A modal whose text and dismissal live in its component.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Dialog>, out: &mut Outbox) {
    let Some(dialog) = tree.get(handle) else {
        return;
    };
    Modal::new(ui.scope_id().with("dialog")).show(ui.ctx(), |ui| {
        ui.set_width(280.0);
        ui.label(RichText::new("Slot").heading());
        ui.label(dialog.text);
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui.button("Dismiss").clicked() {
                out.send(handle, DialogMsg::Dismiss);
            }
        });
    });
}

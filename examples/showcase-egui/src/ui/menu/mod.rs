mod dialog;

use super::widgets::title;
use compono::{Handle, Tree};
use compono_egui::{Outbox, RouterView};
use egui::{Button, Ui, vec2};
use showcase::feature::menu::{Menu, MenuMsg};

/// Links to the other screens, and a dialog held in a slot.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Menu>, out: &mut Outbox) {
    let Some(menu) = tree.get(handle) else {
        return;
    };
    title(ui, "Showcase", |_ui| {});
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        for (label, msg) in [
            ("Open pages", MenuMsg::OpenBook),
            ("Open custom navigation", MenuMsg::OpenGallery),
            ("Show dialog", MenuMsg::ShowDialog),
        ] {
            if ui
                .add(Button::new(label).min_size(vec2(260.0, 36.0)))
                .clicked()
            {
                out.send(handle, msg);
            }
            ui.add_space(8.0);
        }
    });
    menu.dialog
        .view(ui, |ui, dialog| dialog::view(ui, tree, *dialog, out));
}

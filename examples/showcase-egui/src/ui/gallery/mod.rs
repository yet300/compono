mod photo;

use super::widgets::title;
use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{ScrollArea, Ui};
use showcase::feature::gallery::{Gallery, GalleryMsg, StripMode};

/// A filmstrip: a custom router written with only the public API.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Gallery>, out: &mut Outbox) {
    let Some(gallery) = tree.get(handle) else {
        return;
    };
    title(ui, "Filmstrip", |ui| {
        for (label, msg) in [
            ("Remove", GalleryMsg::RemoveSelected),
            ("Change mode", GalleryMsg::ToggleMode),
            ("Next", GalleryMsg::Next),
            ("Previous", GalleryMsg::Previous),
            ("Close", GalleryMsg::Close),
        ] {
            if ui.button(label).clicked() {
                out.send(handle, msg);
            }
        }
    });
    ui.label("A custom router: Filmstrip, written with only public API.");
    ScrollArea::vertical()
        .id_salt(module_path!())
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (key, child, selected) in gallery.strip.frames() {
                    if gallery.strip.mode() == StripMode::Single && !selected {
                        continue;
                    }
                    photo::view(ui, tree, key, *child, selected);
                }
            });
        });
}

use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{RichText, Ui};
use showcase::feature::feed::{Row, RowMsg};

/// One row. It has a component only while it is on screen.
pub fn view(
    ui: &mut Ui,
    tree: &Tree,
    index: usize,
    handle: Option<Handle<Row>>,
    height: f32,
    out: &mut Outbox,
) {
    ui.horizontal(|ui| {
        ui.set_height(height);
        let Some((handle, row)) =
            handle.and_then(|handle| tree.get(handle).map(|row| (handle, row)))
        else {
            ui.weak(format!("Row {index}"));
            return;
        };
        ui.label(format!("Row {}  value {}", row.id, row.value));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let heart = if row.liked { "Liked" } else { "Like" };
            if ui.small_button(RichText::new(heart)).clicked() {
                out.send(handle, RowMsg::Like);
            }
        });
    });
}

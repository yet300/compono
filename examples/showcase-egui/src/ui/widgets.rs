//! Small building blocks every screen shares, made of egui's own widgets.

use egui::{Frame, Margin, RichText, Ui};

/// A rounded card with the window's fill and a soft stroke.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::group(ui.style())
        .inner_margin(Margin::same(14))
        .corner_radius(12.0)
        .fill(ui.visuals().faint_bg_color)
        .show(ui, add)
        .inner
}

/// A screen title with room for actions on the right.
pub fn title(ui: &mut Ui, text: &str, actions: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).heading().strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
    });
    ui.separator();
}

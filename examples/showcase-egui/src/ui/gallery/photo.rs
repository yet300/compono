use compono::{Handle, Tree};
use egui::{Frame, Margin, RichText, Ui, vec2};
use showcase::feature::gallery::Photo;

/// One photo, which develops while it is visible.
pub fn view(ui: &mut Ui, tree: &Tree, key: u64, handle: Handle<Photo>, selected: bool) {
    let developed = tree.get(handle).map_or(0, |photo| photo.developed);
    let visuals = ui.visuals();
    let fill = if selected {
        visuals.selection.bg_fill
    } else {
        visuals.faint_bg_color
    };
    Frame::group(ui.style())
        .fill(fill)
        .corner_radius(10.0)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_size(vec2(72.0, 56.0));
            ui.label(RichText::new(format!("#{key}")).strong());
            ui.label(format!("{developed}"));
        });
}

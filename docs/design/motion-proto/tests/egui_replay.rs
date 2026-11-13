//! Proves an immediate-mode adapter can keep drawing an entry whose component is
//! gone: copy the entry's shapes from its own layer on the last frame it was drawn,
//! then repaint them, translated, on later frames.
use egui::{Area, Context, Id, LayerId, Order, Pos2, RawInput, Rect, Shape, Vec2, emath::TSTransform};

#[test]
fn last_frame_of_a_leaving_entry_can_be_replayed_with_a_transform() {
    let ctx = Context::default();
    let input = || RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))), ..Default::default() };
    let layer = LayerId::new(Order::Middle, Id::new("entry-7"));
    let mut saved = Vec::new();

    // Frame 1: the entry is alive and drawn into its own layer.
    let mut first = ctx.run_ui(input(), |ui| {
        Area::new(Id::new("entry-7")).order(Order::Middle).fixed_pos(Pos2::ZERO).show(ui.ctx(), |ui| {
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(50.0)), 0.0, egui::Color32::RED);
            ui.label("Entry 7");
        });
    });
    first.textures_delta.clear();
    // egui moves shapes out of GraphicLayers when the frame ends, so the adapter
    // copies them inside the frame, right after drawing. Run a frame that does so.
    let mut out = ctx.run_ui(input(), |ui| {
        Area::new(Id::new("entry-7")).order(Order::Middle).fixed_pos(Pos2::ZERO).show(ui.ctx(), |ui| {
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(50.0)), 0.0, egui::Color32::RED);
            ui.label("Entry 7");
        });
        ui.ctx().graphics(|g| {
            if let Some(list) = g.get(layer) { saved = list.all_entries().cloned().collect(); }
        });
    });
    out.textures_delta.clear();
    assert!(!saved.is_empty(), "the entry's shapes were captured");
    let before = out.shapes.len();
    assert!(before > 0);

    // Frame 3: the component is destroyed; nothing calls the entry's view. The
    // adapter repaints the saved shapes into a fresh layer, shifted 200 px right.
    let ghost = LayerId::new(Order::Middle, Id::new("leaving-7"));
    let mut out = ctx.run_ui(input(), |ui| {
        let ctx = ui.ctx().clone();
        ctx.graphics_mut(|g| {
            let list = g.entry(ghost);
            for s in &saved { list.add(s.clip_rect, s.shape.clone()); }
        });
        ctx.set_transform_layer(ghost, TSTransform::from_translation(Vec2::new(200.0, 0.0)));
    });
    out.textures_delta.clear();
    let moved = out.shapes.iter().any(|c| matches!(&c.shape, Shape::Rect(r) if (r.rect.min.x - 210.0).abs() < 0.5));
    assert!(moved, "the replayed rect is drawn 200 px to the right");
}

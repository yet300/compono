mod row;

use super::widgets::title;
use compono::{Handle, Tree};
use compono_egui::{ListView, Outbox};
use egui::Ui;
use showcase::feature::feed::{Feed, FeedMsg};

const ROW_HEIGHT: f32 = 28.0;

/// 100 000 rows. Only the rows on screen have components.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Feed>, out: &mut Outbox) {
    let Some(feed) = tree.get(handle) else {
        return;
    };
    let heading = format!("{} rows, {} live", feed.items.len(), feed.items.live());
    title(ui, &heading, |ui| {
        let mut live = feed.live;
        if ui.toggle_value(&mut live, "Live updates").changed() {
            out.send(handle, FeedMsg::ToggleLive);
        }
    });
    ListView::new(&feed.items, handle, FeedMsg::Window)
        .row_height(ROW_HEIGHT)
        .view(ui, out, |ui, out, index, child| {
            row::view(ui, tree, index, child.copied(), ROW_HEIGHT, out);
        });
}

mod counter;

use super::widgets::title;
use compono::{Handle, Tree};
use compono_egui::{Outbox, RouterView};
use egui::Ui;
use showcase::feature::counters::{Counters, CountersMsg};

/// A stack of counters that can be a thousand deep.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Counters>, out: &mut Outbox) {
    let Some(counters) = tree.get(handle) else {
        return;
    };
    title(
        ui,
        &format!("Stack depth {}", counters.stack.depth()),
        |ui| {
            if ui.button("Back to first").clicked() {
                out.send(handle, CountersMsg::PopToRoot);
            }
            if ui.button("Push 1000 screens").clicked() {
                out.send(handle, CountersMsg::GoDeep);
            }
        },
    );
    counters
        .stack
        .view(ui, |ui, child| counter::view(ui, tree, *child, out));
}

mod page;

use super::widgets::title;
use compono::{Handle, Tree};
use compono_egui::{Outbox, RouterView};
use egui::{ProgressBar, Ui};
use showcase::feature::book::{Book, BookMsg};

/// A hundred pages, of which only the selected one and its neighbours exist.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Book>, out: &mut Outbox) {
    let Some(pages) = tree.get(handle).and_then(|book| book.pages.as_ref()) else {
        return;
    };
    let count = pages.routes().len();
    title(ui, "Pages", |ui| {
        for (label, msg) in [
            ("Jump to 50", BookMsg::Select(49)),
            ("Next", BookMsg::Next),
            ("Previous", BookMsg::Previous),
            ("Close", BookMsg::Close),
        ] {
            if ui.button(label).clicked() {
                out.send(handle, msg);
            }
        }
    });
    #[expect(clippy::cast_precision_loss, reason = "a page count fits a float")]
    let progress = (pages.selected() + 1) as f32 / count.max(1) as f32;
    ui.add(ProgressBar::new(progress).text(format!(
        "{} of {} pages, {} alive",
        pages.selected() + 1,
        count,
        pages.live()
    )));
    pages.view(ui, |ui, child| page::view(ui, tree, *child, out));
}

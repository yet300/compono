use compono::{Handle, Tree};
use compono_egui::Outbox;
use egui::{ScrollArea, Ui};
use showcase::feature::articles::{ARTICLES, ArticleList, ListMsg, article};

const ROW_HEIGHT: f32 = 22.0;

/// A thousand articles, drawn only where they are on screen.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<ArticleList>, out: &mut Outbox) {
    let Some(list) = tree.get(handle) else {
        return;
    };
    let total = usize::try_from(ARTICLES).unwrap_or(0);
    ScrollArea::vertical()
        .id_salt("articles list")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, total, |ui, range| {
            for index in range {
                let id = u32::try_from(index).unwrap_or(0);
                let selected = list.selected == Some(id);
                let row = ui.add_sized(
                    [ui.available_width(), ROW_HEIGHT],
                    egui::Button::selectable(selected, article(id).title),
                );
                if row.clicked() {
                    out.send(handle, ListMsg::Select(id));
                }
            }
        });
}

mod author;
mod details;
mod list;

use super::widgets::title;
use compono::router::panels::{Panel, PanelsMode};
use compono::{Handle, Tree};
use compono_egui::{Outbox, RouterView};
use egui::Ui;
use showcase::feature::articles::{Articles, ArticlesMsg};

/// A list, its details and the author, in one, two or three panels.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Articles>, out: &mut Outbox) {
    let Some(articles) = tree.get(handle) else {
        return;
    };
    title(ui, "Articles", |ui| {
        for (label, mode) in [
            ("Three panels", PanelsMode::Triple),
            ("Two panels", PanelsMode::Dual),
            ("One panel", PanelsMode::Single),
        ] {
            if ui
                .selectable_label(articles.panels.mode() == mode, label)
                .clicked()
            {
                out.send(handle, ArticlesMsg::SetMode(mode));
            }
        }
    });
    articles.panels.view(ui, |ui, panel| match panel {
        Panel::Main(list) => list::view(ui, tree, *list, out),
        Panel::Details(details) => details::view(ui, tree, *details, out),
        Panel::Extra(author) => author::view(ui, tree, *author, out),
    });
}

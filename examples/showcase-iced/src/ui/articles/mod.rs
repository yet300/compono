mod author;
mod details;
mod list;

use super::widgets::{View, title};
use compono::router::panels::{Panel, PanelsMode};
use compono::{Handle, Tree};
use compono_iced::{RouterView, send};
use iced::widget::{button, column, space, text};
use showcase::feature::articles::{Articles, ArticlesMsg};

/// A list, its details and the author, in one, two or three panels.
pub fn view(tree: &Tree, handle: Handle<Articles>) -> View<'_> {
    let Some(articles) = tree.get(handle) else {
        return space().into();
    };
    let modes = [
        ("Three panels", PanelsMode::Triple),
        ("Two panels", PanelsMode::Dual),
        ("One panel", PanelsMode::Single),
    ]
    .into_iter()
    .map(|(label, mode)| {
        let style = if articles.panels.mode() == mode {
            button::primary
        } else {
            button::secondary
        };
        button(text(label))
            .style(style)
            .on_press(send(handle, ArticlesMsg::SetMode(mode)))
            .into()
    })
    .collect();
    column![
        title("Articles".to_owned(), modes),
        articles.panels.view(|panel| match panel {
            Panel::Main(list) => list::view(tree, *list),
            Panel::Details(details) => details::view(tree, *details),
            Panel::Extra(author) => author::view(tree, *author),
        }),
    ]
    .spacing(12)
    .into()
}

mod author;
mod details;
mod list;

use super::widgets::{Cx, choice, fill, title};
use crate::ShowcaseView;
use compono::Handle;
use compono::router::panels::{Panel, PanelsMode};
use compono_gpui::{RouterView, send};
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::v_flex;
use showcase::feature::articles::{Articles, ArticlesMsg};

/// A list, its details and the author, in one, two or three panels.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Articles>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(articles) = tree.get(handle) else {
        return div().into_any_element();
    };
    let mode = articles.panels.mode();
    let modes = [
        ("Three panels", PanelsMode::Triple),
        ("Two panels", PanelsMode::Dual),
        ("One panel", PanelsMode::Single),
    ]
    .map(|(name, value)| {
        choice(
            name,
            mode == value,
            send(handle, ArticlesMsg::SetMode(value), cx),
        )
    });
    let panels = articles.panels.view(|panel| {
        match panel {
            Panel::Main(list) => views.view(*list, list::view, cx),
            Panel::Details(details) => views.view(*details, details::view, cx),
            Panel::Extra(author) => views.view(*author, author::view, cx),
        }
        .cached(fill())
        .into_any_element()
    });
    v_flex()
        .size_full()
        .gap_3()
        .child(title("Articles", modes))
        .child(div().flex_1().min_h_0().child(panels))
        .into_any_element()
}

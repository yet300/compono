use super::super::widgets::Cx;
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, uniform_list};
use gpui_component::button::{Button, ButtonVariants};
use showcase::feature::articles::{ARTICLES, ArticleList, ListMsg, article};

/// A thousand articles, drawn only where they are on screen.
pub fn view(owner: &mut ShowcaseView, handle: Handle<ArticleList>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let selected = tree.get(handle).and_then(|list| list.selected);
    let total = usize::try_from(ARTICLES).unwrap_or(0);
    let rows = cx.processor(move |_owner, range: core::ops::Range<usize>, _window, cx| {
        range
            .map(|index| {
                let id = u32::try_from(index).unwrap_or(0);
                let row = Button::new(("article", id))
                    .label(article(id).title)
                    .w_full()
                    .justify_start()
                    .debug_selector(move || format!("article {id}"))
                    .on_click(send(handle, ListMsg::Select(id), cx));
                if selected == Some(id) {
                    row.primary()
                } else {
                    row.ghost()
                }
            })
            .collect::<Vec<_>>()
    });
    uniform_list("articles", total, rows)
        .size_full()
        .into_any_element()
}

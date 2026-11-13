mod page;

use super::widgets::{Cx, action, fill, label, title};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::{RouterView, send};
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::progress::Progress;
use gpui_component::v_flex;
use showcase::feature::book::{Book, BookMsg};

/// A hundred pages, of which only the selected one and its neighbours exist.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Book>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(pages) = tree.get(handle).and_then(|book| book.pages.as_ref()) else {
        return div().into_any_element();
    };
    let page = pages.view(|child| {
        views
            .view(*child, page::view, cx)
            .cached(fill())
            .into_any_element()
    });
    let count = pages.routes().len();
    let shown = pages.selected() + 1;
    #[expect(clippy::cast_precision_loss, reason = "a page count fits a float")]
    let percent = shown as f32 * 100.0 / count.max(1) as f32;
    let actions = [
        ("Jump to 50", BookMsg::Select(49)),
        ("Previous", BookMsg::Previous),
        ("Next", BookMsg::Next),
        ("Close", BookMsg::Close),
    ]
    .map(|(name, msg)| action(name, send(handle, msg, cx)));
    v_flex()
        .size_full()
        .p_4()
        .gap_3()
        .child(title("Pages", actions))
        .child(Progress::new("pages").value(percent))
        .child(label(format!(
            "{shown} of {count} pages, {} alive",
            pages.live()
        )))
        .child(div().flex_1().min_h_0().children(page))
        .into_any_element()
}

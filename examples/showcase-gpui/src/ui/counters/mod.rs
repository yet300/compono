mod counter;

use super::widgets::{Cx, action, fill, title};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::{RouterView, send};
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::v_flex;
use showcase::feature::counters::{Counters, CountersMsg};

/// A stack of counters that can be a thousand deep.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Counters>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(counters) = tree.get(handle) else {
        return div().into_any_element();
    };
    let screen = counters.stack.view(|child| {
        views
            .view(*child, counter::view, cx)
            .cached(fill())
            .into_any_element()
    });
    v_flex()
        .size_full()
        .gap_4()
        .child(title(
            format!("Stack depth {}", counters.stack.depth()),
            [
                action("Push 1000 screens", send(handle, CountersMsg::GoDeep, cx)),
                action("Back to first", send(handle, CountersMsg::PopToRoot, cx)),
            ],
        ))
        .child(div().flex_1().min_h_0().child(screen))
        .into_any_element()
}

use super::super::widgets::{Cx, action, card, label, primary};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::{h_flex, v_flex};
use showcase::feature::counters::{Counter, CounterMsg};

/// One counter. It ticks only while it is the focused one.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Counter>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(counter) = tree.get(handle) else {
        return div().into_any_element();
    };
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .child(
            card(label(format!("Counter {}", counter.index)).text_xl()).child(
                v_flex()
                    .gap_3()
                    .child(label(format!("Value {}", counter.value)).text_3xl())
                    .child(
                        h_flex()
                            .gap_2()
                            .child(action("Previous", send(handle, CounterMsg::Previous, cx)))
                            .child(primary("Next", send(handle, CounterMsg::Next, cx))),
                    ),
            ),
        )
        .into_any_element()
}

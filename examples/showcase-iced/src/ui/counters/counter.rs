use super::super::widgets::{View, action, card, subtle};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::widget::{column, row, space, text};
use showcase::feature::counters::{Counter, CounterMsg};

/// One counter. It ticks only while it is the focused one.
pub fn view(tree: &Tree, handle: Handle<Counter>) -> View<'_> {
    let Some(counter) = tree.get(handle) else {
        return space().into();
    };
    card(
        text(format!("Counter {}", counter.index)).size(20),
        column![
            text(format!("Value {}", counter.value)).size(40),
            row![
                subtle("Previous", send(handle, CounterMsg::Previous)),
                action("Next", send(handle, CounterMsg::Next)),
            ]
            .spacing(8),
        ]
        .spacing(12),
    )
}

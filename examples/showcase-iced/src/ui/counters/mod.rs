mod counter;

use super::widgets::{View, subtle, title};
use compono::{Handle, Tree};
use compono_iced::{RouterView, send};
use iced::widget::{center, column, space};
use showcase::feature::counters::{Counters, CountersMsg};

/// A stack of counters that can be a thousand deep.
pub fn view(tree: &Tree, handle: Handle<Counters>) -> View<'_> {
    let Some(counters) = tree.get(handle) else {
        return space().into();
    };
    column![
        title(
            format!("Stack depth {}", counters.stack.depth()),
            vec![
                subtle("Push 1000 screens", send(handle, CountersMsg::GoDeep)),
                subtle("Back to first", send(handle, CountersMsg::PopToRoot)),
            ],
        ),
        center(counters.stack.view(|child| counter::view(tree, *child))),
    ]
    .into()
}

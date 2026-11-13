mod card;

use super::widgets::{View, action, subtle, title};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::widget::{Column, column, scrollable, space};
use showcase::feature::cards::{Cards, CardsMsg};

/// A pile of cards kept in any order. Only the top card ticks.
pub fn view(tree: &Tree, handle: Handle<Cards>) -> View<'_> {
    let Some(cards) = tree.get(handle) else {
        return space().into();
    };
    let entries: Vec<_> = cards.cards.iter().collect();
    let pile = entries
        .into_iter()
        .rev()
        .fold(Column::new().spacing(8), |pile, (number, child, phase)| {
            pile.push(card::view(tree, *number, *child, phase))
        });
    column![
        title(
            "Cards".to_owned(),
            vec![
                subtle("Send top to back", send(handle, CardsMsg::SendTopToBack)),
                subtle("Remove", send(handle, CardsMsg::Remove)),
                action("Add", send(handle, CardsMsg::Add)),
            ],
        ),
        scrollable(pile),
    ]
    .spacing(12)
    .into()
}

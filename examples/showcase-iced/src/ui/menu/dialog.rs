use super::super::widgets::{View, action};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::widget::{space, text};
use iced_aw::Card;
use showcase::feature::menu::{Dialog, DialogMsg};

/// A card whose text and dismissal live in its component.
pub fn view(tree: &Tree, handle: Handle<Dialog>) -> View<'_> {
    let Some(dialog) = tree.get(handle) else {
        return space().into();
    };
    Card::new(text("Slot").size(20), text(dialog.text))
        .foot(action("Dismiss", send(handle, DialogMsg::Dismiss)))
        .width(320)
        .style(iced_aw::style::card::primary)
        .into()
}

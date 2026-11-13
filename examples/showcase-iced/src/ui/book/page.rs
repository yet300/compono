use super::super::widgets::{View, action, card};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::widget::{column, space, text};
use showcase::feature::book::{Page, PageMsg};

/// One page, which keeps its tap count while it stays alive.
pub fn view(tree: &Tree, handle: Handle<Page>) -> View<'_> {
    let Some(page) = tree.get(handle) else {
        return space().into();
    };
    card(
        text(format!("Page {}", page.number)).size(28),
        column![
            text(format!("Tapped {} times", page.taps)),
            action("Tap", send(handle, PageMsg::Tap)),
        ]
        .spacing(12),
    )
}

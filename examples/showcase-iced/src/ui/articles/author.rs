use super::super::widgets::{View, subtle};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::Fill;
use iced::widget::{space, text};
use iced_aw::Card;
use showcase::feature::articles::{AuthorMsg, AuthorPanel};

/// The author of the open article.
pub fn view(tree: &Tree, handle: Handle<AuthorPanel>) -> View<'_> {
    let Some(author) = tree.get(handle) else {
        return space().into();
    };
    Card::new(text("Author").size(14), text(&author.author.name).size(20))
        .foot(subtle("Close author", send(handle, AuthorMsg::Close)))
        .width(Fill)
        .style(iced_aw::style::card::secondary)
        .into()
}

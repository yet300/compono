use super::super::widgets::{View, subtle};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::Fill;
use iced::widget::{row, scrollable, space, text};
use iced_aw::Card;
use showcase::feature::articles::{ArticleDetails, DetailsMsg};

/// An article in an `iced_aw` card. Its author opens in the extra panel.
pub fn view(tree: &Tree, handle: Handle<ArticleDetails>) -> View<'_> {
    let Some(details) = tree.get(handle) else {
        return space().into();
    };
    let article = &details.article;
    scrollable(
        Card::new(text(&article.title).size(20), text(&article.body))
            .foot(
                row![
                    subtle("Author", send(handle, DetailsMsg::ShowAuthor)),
                    subtle("Close", send(handle, DetailsMsg::Close)),
                ]
                .spacing(8),
            )
            .width(Fill)
            .style(iced_aw::style::card::primary),
    )
    .into()
}

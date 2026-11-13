use super::super::widgets::View;
use compono::{Handle, Tree};
use compono_iced::send;
use iced::Fill;
use iced::widget::{Column, button, scrollable, space, text};
use showcase::feature::articles::{ARTICLES, ArticleList, ListMsg, article};

/// A thousand articles. The selected one uses the primary colour.
pub fn view(tree: &Tree, handle: Handle<ArticleList>) -> View<'_> {
    let Some(list) = tree.get(handle) else {
        return space().into();
    };
    let rows = (0..ARTICLES).fold(Column::new().spacing(2), |rows, id| {
        let style = if list.selected == Some(id) {
            button::primary
        } else {
            button::text
        };
        rows.push(
            button(text(article(id).title))
                .style(style)
                .width(Fill)
                .on_press(send(handle, ListMsg::Select(id))),
        )
    });
    scrollable(rows).height(Fill).into()
}

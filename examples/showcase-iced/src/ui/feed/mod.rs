mod row;

use super::widgets::{View, title};
use compono::{Handle, Tree};
use compono_iced::{ListView, send};
use iced::widget::{column, space, toggler};
use showcase::feature::feed::{Feed, FeedMsg};

const ROW_HEIGHT: f32 = 32.0;

/// 100 000 rows. Only the rows on screen have components.
pub fn view(tree: &Tree, handle: Handle<Feed>) -> View<'_> {
    let Some(feed) = tree.get(handle) else {
        return space().into();
    };
    let live = toggler(feed.live)
        .label("Live updates")
        .on_toggle(move |_on| send(handle, FeedMsg::ToggleLive))
        .into();
    column![
        title(
            format!("{} rows, {} live", feed.items.len(), feed.items.live()),
            vec![live],
        ),
        ListView::new(&feed.items, handle, FeedMsg::Window)
            .row_height(ROW_HEIGHT)
            .view(|index, child| row::view(tree, index, child.copied(), ROW_HEIGHT)),
    ]
    .spacing(12)
    .into()
}

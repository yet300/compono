mod page;

use super::widgets::{View, subtle, title};
use compono::{Handle, Tree};
use compono_iced::{RouterView, send};
use iced::widget::{center, column, progress_bar, space, text};
use showcase::feature::book::{Book, BookMsg};

/// A hundred pages, of which only the selected one and its neighbours exist.
pub fn view(tree: &Tree, handle: Handle<Book>) -> View<'_> {
    let Some(pages) = tree.get(handle).and_then(|book| book.pages.as_ref()) else {
        return space().into();
    };
    let count = pages.routes().len();
    let actions = [
        ("Jump to 50", BookMsg::Select(49)),
        ("Previous", BookMsg::Previous),
        ("Next", BookMsg::Next),
        ("Close", BookMsg::Close),
    ]
    .into_iter()
    .map(|(label, msg)| subtle(label, send(handle, msg)))
    .collect();
    #[expect(clippy::cast_precision_loss, reason = "a page count fits a float")]
    let shown = (pages.selected() + 1) as f32;
    #[expect(clippy::cast_precision_loss, reason = "a page count fits a float")]
    let range = 0.0..=count.max(1) as f32;
    let content = pages
        .view(|child| page::view(tree, *child))
        .unwrap_or_else(|| space().into());
    column![
        title("Pages".to_owned(), actions),
        progress_bar(range, shown).girth(6),
        text(format!(
            "{} of {} pages, {} alive",
            pages.selected() + 1,
            count,
            pages.live()
        )),
        center(content),
    ]
    .spacing(12)
    .into()
}

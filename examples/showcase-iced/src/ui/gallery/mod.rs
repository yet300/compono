mod photo;

use super::widgets::{View, subtle, title};
use compono::{Handle, Tree};
use compono_iced::send;
use iced::widget::{Row, column, scrollable, space, text};
use showcase::feature::gallery::{Gallery, GalleryMsg, StripMode};

/// A filmstrip: a custom router written with only the public API.
pub fn view(tree: &Tree, handle: Handle<Gallery>) -> View<'_> {
    let Some(gallery) = tree.get(handle) else {
        return space().into();
    };
    let actions = [
        ("Remove", GalleryMsg::RemoveSelected),
        ("Change mode", GalleryMsg::ToggleMode),
        ("Previous", GalleryMsg::Previous),
        ("Next", GalleryMsg::Next),
        ("Close", GalleryMsg::Close),
    ]
    .into_iter()
    .map(|(label, msg)| subtle(label, send(handle, msg)))
    .collect();
    let single = gallery.strip.mode() == StripMode::Single;
    let frames = gallery
        .strip
        .frames()
        .filter(|(_key, _child, selected)| !single || *selected)
        .fold(Row::new().spacing(8), |frames, (key, child, selected)| {
            frames.push(photo::view(tree, key, *child, selected))
        })
        .wrap();
    column![
        title("Filmstrip".to_owned(), actions),
        text("A custom router: Filmstrip, written with only public API."),
        scrollable(frames),
    ]
    .spacing(12)
    .into()
}

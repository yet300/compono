mod dialog;

use super::widgets::{View, action, modal, subtle, title};
use compono::{Handle, Tree};
use compono_iced::{RouterView, send};
use iced::widget::{center, column, space};
use showcase::feature::menu::{Menu, MenuMsg};

/// Links to the other screens, and a dialog held in a slot.
pub fn view(tree: &Tree, handle: Handle<Menu>) -> View<'_> {
    let Some(menu) = tree.get(handle) else {
        return space().into();
    };
    let links: View<'_> = column![
        title("Showcase".to_owned(), Vec::new()),
        center(
            column![
                action("Open pages", send(handle, MenuMsg::OpenBook)),
                subtle("Open custom navigation", send(handle, MenuMsg::OpenGallery)),
                subtle("Show dialog", send(handle, MenuMsg::ShowDialog)),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center)
        ),
    ]
    .into();
    match menu.dialog.view(|child| dialog::view(tree, *child)) {
        Some(dialog) => modal(links, dialog),
        None => links,
    }
}

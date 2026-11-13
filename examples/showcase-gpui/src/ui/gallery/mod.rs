mod photo;

use super::widgets::{Cx, action, label, title};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::send;
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::{h_flex, v_flex};
use showcase::feature::gallery::{Gallery, GalleryMsg, StripMode};

/// A filmstrip: a custom router written with only the public API.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Gallery>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, _views) = owner.parts();
    let Some(gallery) = tree.get(handle) else {
        return div().into_any_element();
    };
    let single = gallery.strip.mode() == StripMode::Single;
    let frames = gallery
        .strip
        .frames()
        .filter(|(_key, _child, selected)| !single || *selected)
        .map(|(key, child, selected)| photo::view(tree, key, *child, selected, cx));
    let actions = [
        ("Remove", GalleryMsg::RemoveSelected),
        ("Change mode", GalleryMsg::ToggleMode),
        ("Previous", GalleryMsg::Previous),
        ("Next", GalleryMsg::Next),
        ("Close", GalleryMsg::Close),
    ]
    .map(|(name, msg)| action(name, send(handle, msg, cx)));
    v_flex()
        .size_full()
        .p_4()
        .gap_3()
        .child(title("Filmstrip", actions))
        .child(label(
            "A custom router: Filmstrip, written with only public API.",
        ))
        .child(h_flex().flex_wrap().gap_2().children(frames))
        .into_any_element()
}

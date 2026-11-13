//! A custom navigation model, written with only the public router API.
//!
//! `Filmstrip` is what a Decompose project would build with `children(...)`.
//! It owns its navigation state, and `cx.scoped` creates a child for each entry,
//! `cx.assign_scope` grants phases and `cx.destroy_scope` removes it.

mod filmstrip;
mod photo;
pub use filmstrip::{Filmstrip, StripMode};
pub use photo::{Photo, PhotoMsg};

use crate::Services;
use compono::{Component, Cx, Handle};
use std::time::Duration;

const FRAMES: u64 = 24;
const TICK: Duration = Duration::from_millis(200);

/// Input to [`Gallery`].
#[derive(Clone, Copy, Debug)]
pub enum GalleryMsg {
    Next,
    Previous,
    ToggleMode,
    RemoveSelected,
    Close,
}

/// What [`Gallery`] asks of its owner.
#[derive(Clone, Copy, Debug)]
pub enum GalleryOutput {
    Close,
}

/// The gallery screen.
#[derive(Debug)]
pub struct Gallery {
    /// The custom router.
    pub strip: Filmstrip<Handle<Photo>, Self>,
}

impl Gallery {
    /// Creates a gallery with a strip of frames.
    pub fn new(cx: &mut Cx<'_, Self>, services: Services) -> Self {
        let strip = Filmstrip::new(1..=FRAMES, cx, |number, cx| {
            let services = services.clone();
            cx.child(
                move |_cx| Photo {
                    number,
                    developed: 0,
                    timer: None,
                    services,
                },
                |_gallery: &mut Self, (), _cx| {},
            )
        });
        Self { strip }
    }
}

impl Component for Gallery {
    type Msg = GalleryMsg;
    type Output = GalleryOutput;

    fn update(&mut self, msg: GalleryMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            GalleryMsg::Next => self.strip.advance(1, cx),
            GalleryMsg::Previous => self.strip.advance(-1, cx),
            GalleryMsg::ToggleMode => {
                let mode = match self.strip.mode() {
                    StripMode::Contact => StripMode::Single,
                    StripMode::Single => StripMode::Contact,
                };
                self.strip.set_mode(mode, cx);
            }
            GalleryMsg::RemoveSelected => self.strip.remove_selected(cx),
            GalleryMsg::Close => cx.emit(GalleryOutput::Close),
        }
    }
}

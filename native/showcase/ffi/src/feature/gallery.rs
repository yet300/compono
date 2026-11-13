//! A custom navigation model: a filmstrip of photos.

use crate::component::{ChildRef, Ctx, address};
use boltffi::{data, export};
use compono::NodeId;
use showcase::feature::gallery::{Gallery, GalleryMsg, Photo, StripMode};

/// One photo of the strip.
#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FrameChild {
    pub child: ChildRef,
    pub selected: bool,
}

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PhotoModel {
    pub number: i64,
    pub developed: i64,
}

#[export]
pub struct GalleryComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl GalleryComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    /// Whether every photo shows, rather than a window around the selected one.
    pub fn is_contact(&self) -> bool {
        self.ctx
            .read::<Gallery, _>(self.node, |gallery| {
                gallery.strip.mode() == StripMode::Contact
            })
            .unwrap_or(false)
    }

    pub fn frames(&self) -> Vec<FrameChild> {
        let frames: Vec<(u64, NodeId, bool)> = self
            .ctx
            .read::<Gallery, _>(self.node, |gallery| {
                gallery
                    .strip
                    .frames()
                    .map(|(key, photo, selected)| (key, photo.id(), selected))
                    .collect()
            })
            .unwrap_or_default();
        frames
            .into_iter()
            .filter_map(|(key, id, selected)| {
                Some(FrameChild {
                    child: self.ctx.child(key, id)?,
                    selected,
                })
            })
            .collect()
    }

    pub fn photo(&self, frame: FrameChild) -> PhotoComponent {
        PhotoComponent {
            ctx: self.ctx.clone(),
            node: address(frame.child),
        }
    }

    pub fn next(&self) {
        self.ctx.send::<Gallery>(self.node, GalleryMsg::Next);
    }

    pub fn previous(&self) {
        self.ctx.send::<Gallery>(self.node, GalleryMsg::Previous);
    }

    pub fn toggle_mode(&self) {
        self.ctx.send::<Gallery>(self.node, GalleryMsg::ToggleMode);
    }

    pub fn remove_selected(&self) {
        self.ctx
            .send::<Gallery>(self.node, GalleryMsg::RemoveSelected);
    }

    pub fn dismiss(&self) {
        self.ctx.send::<Gallery>(self.node, GalleryMsg::Close);
    }
}

#[export]
pub struct PhotoComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl PhotoComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<PhotoModel> {
        self.ctx.read::<Photo, _>(self.node, |photo| PhotoModel {
            number: photo.number.cast_signed(),
            developed: photo.developed.cast_signed(),
        })
    }
}

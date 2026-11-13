//! The root: a stack of whole screens.

use super::book::BookComponent;
use super::gallery::GalleryComponent;
use super::tabs::TabsComponent;
use crate::component::{ChildRef, Ctx, address};
use boltffi::{data, export};
use compono::Handle;
use compono_ffi::NO_NODE;
use showcase::feature::root::{Child, Root};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    Tabs,
    Book,
    Gallery,
}

/// A screen of the root stack.
#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ScreenChild {
    pub kind: ScreenKind,
    pub child: ChildRef,
}

/// The application: tabs, with the book or the gallery opened above them.
#[export]
pub struct RootComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

impl RootComponent {
    fn screen(&self, pick: impl FnOnce(&Root) -> Option<(u64, Child)>) -> Option<ScreenChild> {
        let (key, child) = self.ctx.read::<Root, _>(self.node, pick)??;
        let (kind, node) = match child {
            Child::Tabs(handle) => (ScreenKind::Tabs, handle.id()),
            Child::Book(handle) => (ScreenKind::Book, handle.id()),
            Child::Gallery(handle) => (ScreenKind::Gallery, handle.id()),
        };
        Some(ScreenChild {
            kind,
            child: self.ctx.child(key, node)?,
        })
    }
}

#[export]
impl RootComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    /// The screen on top.
    pub fn active(&self) -> Option<ScreenChild> {
        self.screen(|root| Some((root.stack.active_key(), *root.stack.active())))
    }

    /// The screen that Back would reveal.
    pub fn previous(&self) -> Option<ScreenChild> {
        self.screen(|root| root.stack.previous().map(|(key, child)| (key, *child)))
    }

    pub fn tabs(&self, child: ScreenChild) -> Option<TabsComponent> {
        (child.kind == ScreenKind::Tabs).then(|| TabsComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }

    pub fn book(&self, child: ScreenChild) -> Option<BookComponent> {
        (child.kind == ScreenKind::Book).then(|| BookComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }

    pub fn gallery(&self, child: ScreenChild) -> Option<GalleryComponent> {
        (child.kind == ScreenKind::Gallery).then(|| GalleryComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }
}

pub(crate) fn root_component(ctx: Ctx, root: Option<Handle<Root>>) -> RootComponent {
    RootComponent {
        ctx,
        node: root.map_or(NO_NODE.cast_unsigned(), |root| root.id().to_bits()),
    }
}

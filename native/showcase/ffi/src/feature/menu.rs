//! The first tab: links and a dialog in a slot.

use crate::component::{ChildRef, Ctx, address};
use boltffi::export;
use showcase::feature::menu::{Dialog, DialogMsg, Menu, MenuMsg};

#[export]
pub struct MenuComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl MenuComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    /// The open dialog, if any.
    pub fn dialog(&self) -> Option<ChildRef> {
        let dialog = self
            .ctx
            .read::<Menu, _>(self.node, |menu| menu.dialog.active().copied())??;
        self.ctx.child(0, dialog.id())
    }

    pub fn dialog_component(&self, child: ChildRef) -> DialogComponent {
        DialogComponent {
            ctx: self.ctx.clone(),
            node: address(child),
        }
    }

    pub fn open_book(&self) {
        self.ctx.send::<Menu>(self.node, MenuMsg::OpenBook);
    }

    pub fn open_gallery(&self) {
        self.ctx.send::<Menu>(self.node, MenuMsg::OpenGallery);
    }

    pub fn show_dialog(&self) {
        self.ctx.send::<Menu>(self.node, MenuMsg::ShowDialog);
    }
}

#[export]
pub struct DialogComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl DialogComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn text(&self) -> Option<String> {
        self.ctx
            .read::<Dialog, _>(self.node, |dialog| dialog.text.to_owned())
    }

    pub fn dismiss(&self) {
        self.ctx.send::<Dialog>(self.node, DialogMsg::Dismiss);
    }
}

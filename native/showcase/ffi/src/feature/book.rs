//! A hundred pages, of which only the selected one and its neighbours exist.

use crate::component::{ChildRef, Ctx, address, to_i32};
use boltffi::{data, export};
use showcase::feature::book::{Book, BookMsg, Page, PageMsg};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BookModel {
    pub selected: i32,
    pub pages: i32,
    pub alive: i32,
}

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PageModel {
    pub number: i32,
    pub taps: i32,
}

#[export]
pub struct BookComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl BookComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<BookModel> {
        self.ctx.read::<Book, _>(self.node, |book| {
            book.pages.as_ref().map(|pages| BookModel {
                selected: to_i32(pages.selected()),
                pages: to_i32(pages.routes().len()),
                alive: to_i32(pages.live()),
            })
        })?
    }

    /// The selected page.
    pub fn active(&self) -> Option<ChildRef> {
        let (selected, page) = self.ctx.read::<Book, _>(self.node, |book| {
            let pages = book.pages.as_ref()?;
            Some((pages.selected(), *pages.active()?))
        })??;
        self.ctx.child(selected as u64, page.id())
    }

    pub fn page(&self, child: ChildRef) -> PageComponent {
        PageComponent {
            ctx: self.ctx.clone(),
            node: address(child),
        }
    }

    pub fn next(&self) {
        self.ctx.send::<Book>(self.node, BookMsg::Next);
    }

    pub fn previous(&self) {
        self.ctx.send::<Book>(self.node, BookMsg::Previous);
    }

    pub fn dismiss(&self) {
        self.ctx.send::<Book>(self.node, BookMsg::Close);
    }

    pub fn select(&self, index: i32) {
        if let Ok(index) = usize::try_from(index) {
            self.ctx.send::<Book>(self.node, BookMsg::Select(index));
        }
    }
}

#[export]
pub struct PageComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl PageComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<PageModel> {
        self.ctx.read::<Page, _>(self.node, |page| PageModel {
            number: i32::try_from(page.number).unwrap_or(i32::MAX),
            taps: i32::try_from(page.taps).unwrap_or(i32::MAX),
        })
    }

    pub fn tap(&self) {
        self.ctx.send::<Page>(self.node, PageMsg::Tap);
    }
}

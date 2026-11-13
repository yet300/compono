//! 100 000 rows, with live components only on screen.

use crate::component::{Ctx, to_i32};
use boltffi::{data, export};
use compono::NodeId;
use showcase::feature::feed::{Feed, FeedMsg, Row, RowMsg};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FeedModel {
    pub rows: i32,
    pub live_rows: i32,
    pub live: bool,
}

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RowModel {
    /// The row's address, which stays while the row is live.
    pub node: i64,
    pub id: i64,
    pub value: i64,
    pub liked: bool,
}

#[export]
pub struct FeedComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl FeedComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<FeedModel> {
        self.ctx.read::<Feed, _>(self.node, |feed| FeedModel {
            rows: to_i32(feed.items.len()),
            live_rows: to_i32(feed.items.live()),
            live: feed.live,
        })
    }

    pub fn toggle_live(&self) {
        self.ctx.send::<Feed>(self.node, FeedMsg::ToggleLive);
    }

    /// Tells which rows are on screen, from `first` up to `last`.
    pub fn set_window(&self, first: i32, last: i32) {
        if let (Ok(first), Ok(last)) = (usize::try_from(first), usize::try_from(last)) {
            self.ctx
                .send::<Feed>(self.node, FeedMsg::Window(first..last));
        }
    }

    /// A row's state, or `None` while it is outside the live window.
    pub fn row_at(&self, index: i32) -> Option<RowModel> {
        let index = usize::try_from(index).ok()?;
        self.ctx.tree(|tree| {
            let feed = tree.handle_of::<Feed>(NodeId::from_bits(self.node))?;
            let row = *tree.get(feed)?.items.child(index)?;
            let state: &Row = tree.get(row)?;
            Some(RowModel {
                node: row.id().to_bits().cast_signed(),
                id: state.id.cast_signed(),
                value: state.value.cast_signed(),
                liked: state.liked,
            })
        })
    }

    pub fn row(&self, model: RowModel) -> RowComponent {
        RowComponent {
            ctx: self.ctx.clone(),
            node: model.node.cast_unsigned(),
        }
    }
}

#[export]
pub struct RowComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl RowComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn like(&self) {
        self.ctx.send::<Row>(self.node, RowMsg::Like);
    }
}

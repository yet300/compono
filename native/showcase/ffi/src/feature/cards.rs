//! A pile of cards kept in any order.

use crate::component::{ChildRef, Ctx, address};
use boltffi::{data, export};
use compono::NodeId;
use showcase::feature::cards::{Card, Cards, CardsMsg};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CardModel {
    pub number: i32,
    pub value: i64,
}

#[export]
pub struct CardsComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl CardsComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    /// The cards, bottom first.
    pub fn cards(&self) -> Vec<ChildRef> {
        let cards: Vec<(u64, NodeId)> = self
            .ctx
            .read::<Cards, _>(self.node, |cards| {
                cards
                    .cards
                    .iter()
                    .map(|(number, card, _phase)| (u64::from(*number), card.id()))
                    .collect()
            })
            .unwrap_or_default();
        cards
            .into_iter()
            .filter_map(|(key, id)| self.ctx.child(key, id))
            .collect()
    }

    pub fn card(&self, child: ChildRef) -> CardComponent {
        CardComponent {
            ctx: self.ctx.clone(),
            node: address(child),
        }
    }

    pub fn add(&self) {
        self.ctx.send::<Cards>(self.node, CardsMsg::Add);
    }

    pub fn remove(&self) {
        self.ctx.send::<Cards>(self.node, CardsMsg::Remove);
    }

    pub fn send_top_to_back(&self) {
        self.ctx.send::<Cards>(self.node, CardsMsg::SendTopToBack);
    }
}

#[export]
pub struct CardComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl CardComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<CardModel> {
        self.ctx.read::<Card, _>(self.node, |card| CardModel {
            number: i32::try_from(card.number).unwrap_or(i32::MAX),
            value: card.value.cast_signed(),
        })
    }
}

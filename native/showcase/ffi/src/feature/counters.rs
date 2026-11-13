//! A deep stack of counters.

use crate::component::{ChildRef, Ctx, address, to_i64};
use boltffi::{data, export};
use showcase::feature::counters::{Counter, CounterMsg, Counters, CountersMsg, DEEP_PUSH};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CountersModel {
    pub depth: i64,
    pub deep_push: i64,
}

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CounterModel {
    pub index: i64,
    pub value: i64,
}

#[export]
pub struct CountersComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl CountersComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<CountersModel> {
        self.ctx
            .read::<Counters, _>(self.node, |counters| CountersModel {
                depth: to_i64(counters.stack.depth()),
                deep_push: to_i64(DEEP_PUSH),
            })
    }

    /// The counter on top.
    pub fn active(&self) -> Option<ChildRef> {
        let (key, counter) = self.ctx.read::<Counters, _>(self.node, |counters| {
            (counters.stack.active_key(), *counters.stack.active())
        })?;
        self.ctx.child(key, counter.id())
    }

    /// The counter that Back would reveal.
    pub fn revealed(&self) -> Option<ChildRef> {
        let (key, counter) = self.ctx.read::<Counters, _>(self.node, |counters| {
            counters
                .stack
                .previous()
                .map(|(key, counter)| (key, *counter))
        })??;
        self.ctx.child(key, counter.id())
    }

    pub fn counter(&self, child: ChildRef) -> CounterComponent {
        CounterComponent {
            ctx: self.ctx.clone(),
            node: address(child),
        }
    }

    pub fn go_deep(&self) {
        self.ctx.send::<Counters>(self.node, CountersMsg::GoDeep);
    }

    pub fn pop_to_root(&self) {
        self.ctx.send::<Counters>(self.node, CountersMsg::PopToRoot);
    }

    pub fn back(&self) {
        self.ctx.send::<Counters>(self.node, CountersMsg::Back);
    }
}

#[export]
pub struct CounterComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl CounterComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn model(&self) -> Option<CounterModel> {
        self.ctx
            .read::<Counter, _>(self.node, |counter| CounterModel {
                index: to_i64(counter.index),
                value: counter.value.cast_signed(),
            })
    }

    pub fn next(&self) {
        self.ctx.send::<Counter>(self.node, CounterMsg::Next);
    }

    pub fn previous(&self) {
        self.ctx.send::<Counter>(self.node, CounterMsg::Previous);
    }
}

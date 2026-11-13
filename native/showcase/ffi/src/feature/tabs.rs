//! Bottom tabs, each keeping its own state.

use super::articles::ArticlesComponent;
use super::cards::CardsComponent;
use super::counters::CountersComponent;
use super::feed::FeedComponent;
use super::menu::MenuComponent;
use crate::component::{ChildRef, Ctx, address};
use boltffi::{data, export};
use showcase::feature::tabs::{Tab, TabChild, Tabs, TabsMsg};

#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Menu,
    Counters,
    Cards,
    Articles,
    Feed,
}

/// The selected tab and its child.
#[data]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TabsChild {
    pub tab: TabKind,
    pub child: ChildRef,
}

#[export]
pub struct TabsComponent {
    pub(crate) ctx: Ctx,
    pub(crate) node: u64,
}

#[export]
impl TabsComponent {
    pub fn node(&self) -> i64 {
        self.node.cast_signed()
    }

    pub fn active(&self) -> Option<TabsChild> {
        let (key, child) = self.ctx.read::<Tabs, _>(self.node, |tabs| {
            (tabs.stack.active_key(), *tabs.stack.active())
        })?;
        let (tab, node) = match child {
            TabChild::Menu(handle) => (TabKind::Menu, handle.id()),
            TabChild::Counters(handle) => (TabKind::Counters, handle.id()),
            TabChild::Cards(handle) => (TabKind::Cards, handle.id()),
            TabChild::Articles(handle) => (TabKind::Articles, handle.id()),
            TabChild::Feed(handle) => (TabKind::Feed, handle.id()),
        };
        Some(TabsChild {
            tab,
            child: self.ctx.child(key, node)?,
        })
    }

    pub fn select(&self, tab: TabKind) {
        let tab = match tab {
            TabKind::Menu => Tab::Menu,
            TabKind::Counters => Tab::Counters,
            TabKind::Cards => Tab::Cards,
            TabKind::Articles => Tab::Articles,
            TabKind::Feed => Tab::Feed,
        };
        self.ctx.send::<Tabs>(self.node, TabsMsg::Select(tab));
    }

    pub fn menu(&self, child: TabsChild) -> Option<MenuComponent> {
        (child.tab == TabKind::Menu).then(|| MenuComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }

    pub fn counters(&self, child: TabsChild) -> Option<CountersComponent> {
        (child.tab == TabKind::Counters).then(|| CountersComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }

    pub fn cards(&self, child: TabsChild) -> Option<CardsComponent> {
        (child.tab == TabKind::Cards).then(|| CardsComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }

    pub fn articles(&self, child: TabsChild) -> Option<ArticlesComponent> {
        (child.tab == TabKind::Articles).then(|| ArticlesComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }

    pub fn feed(&self, child: TabsChild) -> Option<FeedComponent> {
        (child.tab == TabKind::Feed).then(|| FeedComponent {
            ctx: self.ctx.clone(),
            node: address(child.child),
        })
    }
}

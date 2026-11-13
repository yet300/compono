//! Bottom tabs: a stack that brings a tab to the front instead of pushing.

use crate::Services;
use crate::feature::articles::Articles;
use crate::feature::cards::Cards;
use crate::feature::counters::Counters;
use crate::feature::feed::Feed;
use crate::feature::menu::{Menu, MenuOutput};
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist};
use serde::{Deserialize, Serialize};

/// A tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tab {
    Menu,
    Counters,
    Cards,
    Articles,
    Feed,
}

impl Tab {
    /// Every tab with its title.
    pub const ALL: [(&'static str, Self); 5] = [
        ("Menu", Self::Menu),
        ("Counters", Self::Counters),
        ("Cards", Self::Cards),
        ("Articles", Self::Articles),
        ("Feed", Self::Feed),
    ];
}

/// The component behind a tab.
#[derive(Clone, Copy, Debug)]
pub enum TabChild {
    Menu(Handle<Menu>),
    Counters(Handle<Counters>),
    Cards(Handle<Cards>),
    Articles(Handle<Articles>),
    Feed(Handle<Feed>),
}

/// Input to [`Tabs`].
#[derive(Clone, Copy, Debug)]
pub enum TabsMsg {
    Select(Tab),
}

/// What [`Tabs`] asks of its owner.
#[derive(Clone, Copy, Debug)]
pub enum TabsOutput {
    OpenBook,
    OpenGallery,
}

/// The tab bar and whichever tab is selected.
#[derive(Debug)]
pub struct Tabs {
    /// The tabs, the selected one on top.
    pub stack: Stack<Tab, TabChild, Self>,
}

impl Tabs {
    /// Creates the tabs, resuming the saved selection if there is any.
    pub fn new(cx: &mut Cx<'_, Self>, services: Services) -> Self {
        let saved = cx.restored();
        // A name keeps the saved stack its own if routers are added before it.
        let stack = cx.named_routers("tabs", |cx| {
            Stack::restore_or(saved, Tab::Menu, cx, move |tab, cx| {
                let services = services.clone();
                match tab {
                    Tab::Menu => TabChild::Menu(cx.child(Menu::new, Self::on_menu)),
                    Tab::Counters => TabChild::Counters(cx.persistent_child(
                        move |cx| Counters::new(cx, services),
                        |_tabs: &mut Self, (), _cx| {},
                    )),
                    Tab::Cards => TabChild::Cards(cx.child(
                        move |cx| Cards::new(cx, services),
                        |_tabs: &mut Self, (), _cx| {},
                    )),
                    Tab::Articles => {
                        TabChild::Articles(cx.child(Articles::new, |_tabs: &mut Self, (), _cx| {}))
                    }
                    Tab::Feed => TabChild::Feed(cx.child(
                        move |cx| Feed::new(cx, services),
                        |_tabs: &mut Self, (), _cx| {},
                    )),
                }
            })
        });
        Self { stack }
    }

    fn on_menu(&mut self, output: MenuOutput, cx: &mut Cx<'_, Self>) {
        cx.emit(match output {
            MenuOutput::OpenBook => TabsOutput::OpenBook,
            MenuOutput::OpenGallery => TabsOutput::OpenGallery,
        });
    }
}

impl Component for Tabs {
    type Msg = TabsMsg;
    type Output = TabsOutput;

    fn update(&mut self, msg: TabsMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            TabsMsg::Select(tab) => self.stack.bring_to_front(tab, cx),
        }
    }
}

impl Persist for Tabs {
    type Saved = SavedStack<Tab>;
    const KIND: &'static str = "showcase.tabs";

    fn save(&self) -> Self::Saved {
        self.stack.snapshot()
    }
}

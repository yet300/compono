use super::widgets::{Cx, fill};
use super::{articles, cards, counters, feed, menu};
use crate::ShowcaseView;
use compono::Handle;
use compono_gpui::{RouterView, send};
use gpui::prelude::*;
use gpui::{AnyElement, div};
use gpui_component::tab::{Tab as TabItem, TabBar};
use gpui_component::v_flex;
use showcase::feature::tabs::{Tab, TabChild, Tabs, TabsMsg};

/// The selected tab above a segmented tab bar. Each tab keeps its state.
pub fn view(owner: &mut ShowcaseView, handle: Handle<Tabs>, cx: &mut Cx<'_>) -> AnyElement {
    let (tree, views) = owner.parts();
    let Some(tabs) = tree.get(handle) else {
        return div().into_any_element();
    };
    let content = tabs.stack.view(|child| {
        match *child {
            TabChild::Menu(menu) => views.view(menu, menu::view, cx),
            TabChild::Counters(counters) => views.view(counters, counters::view, cx),
            TabChild::Cards(cards) => views.view(cards, cards::view, cx),
            TabChild::Articles(articles) => views.view(articles, articles::view, cx),
            TabChild::Feed(feed) => views.view(feed, feed::view, cx),
        }
        .cached(fill())
        .into_any_element()
    });
    let selected = Tab::ALL
        .iter()
        .position(|(_title, tab)| tabs.stack.route() == tab)
        .unwrap_or(0);
    let items = Tab::ALL.iter().map(|(title, tab)| {
        let title: &'static str = title;
        TabItem::new()
            .label(title)
            .debug_selector(move || title.to_owned())
            .on_click(send(handle, TabsMsg::Select(*tab), cx))
    });
    let bar = TabBar::new("tabs")
        .segmented()
        .selected_index(selected)
        .children(items);
    v_flex()
        .size_full()
        .child(div().flex_1().min_h_0().p_4().child(content))
        .child(div().p_2().flex().justify_center().child(bar))
        .into_any_element()
}

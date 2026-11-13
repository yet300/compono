use super::widgets::View;
use super::{articles, cards, counters, feed, menu};
use compono::{Handle, Tree};
use compono_iced::{RouterView, send};
use iced::Fill;
use iced::widget::{column, container, space};
use iced_aw::{TabBar, TabLabel};
use showcase::feature::tabs::{Tab, TabChild, Tabs, TabsMsg};

/// An `iced_aw` tab bar under the selected tab. Each tab keeps its state.
pub fn view(tree: &Tree, handle: Handle<Tabs>) -> View<'_> {
    let Some(tabs) = tree.get(handle) else {
        return space().into();
    };
    let content = tabs.stack.view(|child| match *child {
        TabChild::Menu(menu) => menu::view(tree, menu),
        TabChild::Counters(counters) => counters::view(tree, counters),
        TabChild::Cards(cards) => cards::view(tree, cards),
        TabChild::Articles(articles) => articles::view(tree, articles),
        TabChild::Feed(feed) => feed::view(tree, feed),
    });
    let bar = Tab::ALL
        .iter()
        .fold(
            TabBar::new(move |tab: Tab| send(handle, TabsMsg::Select(tab))),
            |bar, (title, tab)| bar.push(*tab, TabLabel::Text((*title).to_owned())),
        )
        .set_active_tab(tabs.stack.route())
        .style(iced_aw::style::tab_bar::primary)
        .height(48);
    column![container(content).padding(16).width(Fill).height(Fill), bar].into()
}

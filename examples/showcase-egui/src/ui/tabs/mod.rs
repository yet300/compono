use super::{articles, cards, counters, feed, menu};
use compono::{Handle, Tree};
use compono_egui::{Outbox, RouterView};
use egui::{Button, Panel, RichText, Ui};
use showcase::feature::tabs::{Tab, TabChild, Tabs, TabsMsg};

/// A bottom bar of tabs; each tab keeps its state.
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Tabs>, out: &mut Outbox) {
    let Some(tabs) = tree.get(handle) else {
        return;
    };
    Panel::bottom("tabs").show(ui, |ui| {
        ui.columns(Tab::ALL.len(), |columns| {
            for (column, (title, tab)) in columns.iter_mut().zip(Tab::ALL) {
                column.vertical_centered(|ui| {
                    let selected = *tabs.stack.route() == tab;
                    let label = RichText::new(title).size(14.0);
                    if ui
                        .add(Button::selectable(selected, label).frame(selected))
                        .clicked()
                    {
                        out.send(handle, TabsMsg::Select(tab));
                    }
                });
            }
        });
    });
    tabs.stack.view(ui, |ui, child| match *child {
        TabChild::Menu(menu) => menu::view(ui, tree, menu, out),
        TabChild::Counters(counters) => counters::view(ui, tree, counters, out),
        TabChild::Cards(cards) => cards::view(ui, tree, cards, out),
        TabChild::Articles(articles) => articles::view(ui, tree, articles, out),
        TabChild::Feed(feed) => feed::view(ui, tree, feed, out),
    });
}

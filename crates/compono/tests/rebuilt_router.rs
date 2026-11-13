//! A router that its owner rebuilds keeps the saved state of its entries.

#![cfg(feature = "serde")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::items::Items;
use compono::router::pages::Pages;
use compono::router::panels::{PanelTypes, Panels, PanelsMode};
use compono::router::set::ChildSet;
use compono::router::slot::{SavedSlot, Slot};
use compono::router::stack::Stack;
use compono::router::{Router, RouterId};
use compono::{Component, Cx, Handle, Persist, Tree};

struct Cell(u32);

impl Component for Cell {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, value: u32, _cx: &mut Cx<'_, Self>) {
        self.0 = value;
    }
}

impl Persist for Cell {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.0
    }
}

fn cell(_route: &u32, cx: &mut Cx<'_, Owner>) -> Handle<Cell> {
    cx.persistent_child(|cx| Cell(cx.restored().unwrap_or(0)), |_owner, (), _cx| {})
}

/// Holds one dialog, and closes it by building a fresh slot.
struct Owner {
    dialog: Option<Slot<u32, Handle<Cell>, Owner>>,
}

enum Msg {
    Rebuild,
    Show(u32),
}

impl Component for Owner {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Rebuild => {
                let old = self.dialog.take().unwrap();
                self.dialog = Some(cx.replace_router(old, |cx| Slot::new(cx, cell)));
            }
            Msg::Show(route) => {
                self.dialog.as_mut().unwrap().show(route, cx);
            }
        }
    }
}

impl Persist for Owner {
    type Saved = SavedSlot<u32>;
    fn save(&self) -> SavedSlot<u32> {
        self.dialog.as_ref().unwrap().snapshot()
    }
}

fn mount(tree: &mut Tree) -> Handle<Owner> {
    tree.mount_persistent(|cx| Owner {
        dialog: Some(Slot::restore(cx.restored(), cx, cell)),
    })
}

/// Rebuilds the slot `rebuilds` times, shows a dialog holding 222, saves, and
/// returns what the dialog holds after a relaunch.
fn relaunched_after(rebuilds: usize) -> Option<u32> {
    let mut tree = Tree::new(NullHost);
    let owner = mount(&mut tree);
    for _ in 0..rebuilds {
        tree.send(owner, Msg::Rebuild).unwrap();
    }
    tree.send(owner, Msg::Show(5)).unwrap();
    let dialog = *tree
        .get(owner)
        .unwrap()
        .dialog
        .as_ref()
        .unwrap()
        .active()
        .unwrap();
    tree.send(dialog, 222).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let owner = mount(&mut tree);
    let dialog = *tree.get(owner).unwrap().dialog.as_ref()?.active()?;
    tree.get(dialog).map(|cell| cell.0)
}

#[test]
fn a_slot_never_rebuilt_keeps_its_dialog_state() {
    assert_eq!(relaunched_after(0), Some(222));
}

#[test]
fn a_rebuilt_slot_keeps_its_dialog_state() {
    assert_eq!(relaunched_after(1), Some(222));
}

#[test]
fn a_slot_rebuilt_past_the_router_ids_keeps_its_dialog_state() {
    assert_eq!(relaunched_after(40_000), Some(222));
}

struct Types;
impl PanelTypes for Types {
    type Main = ();
    type MainChild = ();
    type Details = u32;
    type DetailsChild = ();
    type Extra = u32;
    type ExtraChild = ();
}

/// One of every router, rebuilt together.
struct Every {
    stack: Option<Stack<u32, (), Every>>,
    set: Option<ChildSet<u32, (), Every>>,
    pages: Option<Pages<u32, (), Every>>,
    items: Option<Items<u32, (), Every>>,
    panels: Option<Panels<Types, Every>>,
    /// The id a router made after the rebuild gets.
    next: Option<RouterId>,
}

fn ids(every: &Every) -> Vec<RouterId> {
    let mut ids: Vec<RouterId> = Vec::new();
    ids.extend(every.stack.as_ref().unwrap().router_ids());
    ids.extend(every.set.as_ref().unwrap().router_ids());
    ids.extend(every.pages.as_ref().unwrap().router_ids());
    ids.extend(every.items.as_ref().unwrap().router_ids());
    ids.extend(every.panels.as_ref().unwrap().router_ids());
    ids
}

fn nothing<K>(_key: &K, _cx: &mut Cx<'_, Every>) {}

impl Component for Every {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        let stack = self.stack.take().unwrap();
        self.stack = Some(cx.replace_router(stack, |cx| Stack::new(0, cx, nothing)));
        let set = self.set.take().unwrap();
        self.set = Some(cx.replace_router(set, |cx| ChildSet::new(cx, nothing)));
        let pages = self.pages.take().unwrap();
        self.pages = Some(cx.replace_router(pages, |cx| {
            Pages::new(vec![0, 1], 0, 1, cx, nothing).unwrap()
        }));
        let items = self.items.take().unwrap();
        self.items = Some(cx.replace_router(items, |cx| Items::new(vec![0, 1], cx, nothing)));
        let panels = self.panels.take().unwrap();
        self.panels = Some(cx.replace_router(panels, |cx| {
            Panels::new((), PanelsMode::Single, cx, nothing, nothing, nothing)
        }));
        self.next = Some(cx.router());
    }
}

#[test]
fn every_router_rebuilt_in_its_own_ids_keeps_them() {
    let mut tree = Tree::new(NullHost);
    let every = tree.mount(|cx| Every {
        stack: Some(Stack::new(0, cx, nothing)),
        set: Some(ChildSet::new(cx, nothing)),
        pages: Some(Pages::new(vec![0, 1], 0, 1, cx, nothing).unwrap()),
        items: Some(Items::new(vec![0, 1], cx, nothing)),
        panels: Some(Panels::new(
            (),
            PanelsMode::Single,
            cx,
            nothing,
            nothing,
            nothing,
        )),
        next: None,
    });
    let before = ids(tree.get(every).unwrap());
    assert_eq!(before.len(), 7, "panels hold three routers");

    tree.send(every, ()).unwrap();

    let after = tree.get(every).unwrap();
    assert_eq!(ids(after), before);
    assert_eq!(
        format!("{:?}", after.next.unwrap()),
        "RouterId(7)",
        "no id was used up"
    );
}

//! `Cx::replace_router` gives the new router the old router's ids, so the new
//! router's entries find their saved state after a restart.

#![cfg(feature = "serde")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::panels::{PanelTypes, Panels, PanelsMode, SavedPanels};
use compono::router::slot::{SavedSlot, Slot};
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist, Tree};
use serde::{Deserialize, Serialize};

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

fn cell<P: Component>(_route: &u32, cx: &mut Cx<'_, P>) -> Handle<Cell> {
    cx.persistent_child(|cx| Cell(cx.restored().unwrap_or(0)), |_owner, (), _cx| {})
}

fn value(tree: &Tree, cell: Handle<Cell>) -> u32 {
    tree.get(cell).unwrap().0
}

struct Types;
impl PanelTypes for Types {
    type Main = u32;
    type MainChild = Handle<Cell>;
    type Details = u32;
    type DetailsChild = Handle<Cell>;
    type Extra = u32;
    type ExtraChild = Handle<Cell>;
}

/// One router of each shape. Each is an `Option`, so the owner can move it
/// into `replace_router`.
struct Owner {
    stack: Option<Stack<u32, Handle<Cell>, Owner>>,
    dialog: Option<Slot<u32, Handle<Cell>, Owner>>,
    panels: Option<Panels<Types, Owner>>,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    stack: SavedStack<u32>,
    dialog: SavedSlot<u32>,
    panels: SavedPanels<u32, u32, u32>,
}

enum Msg {
    /// Replaces every router with a fresh one.
    Replace,
    /// Opens an entry in every router.
    Open,
}

impl Component for Owner {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Replace => {
                let stack = self.stack.take().unwrap();
                self.stack = Some(cx.replace_router(stack, |cx| Stack::new(0, cx, cell)));
                let dialog = self.dialog.take().unwrap();
                self.dialog = Some(cx.replace_router(dialog, |cx| Slot::new(cx, cell)));
                let panels = self.panels.take().unwrap();
                self.panels = Some(cx.replace_router(panels, |cx| {
                    Panels::new(0, PanelsMode::Triple, cx, cell, cell, cell)
                }));
            }
            Msg::Open => {
                self.stack.as_mut().unwrap().push(1, cx);
                self.dialog.as_mut().unwrap().show(2, cx);
                let panels = self.panels.as_mut().unwrap();
                panels.set_details(Some(3), cx);
                panels.set_extra(Some(4), cx);
            }
        }
    }
}

impl Persist for Owner {
    type Saved = Saved;
    fn save(&self) -> Saved {
        Saved {
            stack: self.stack.as_ref().unwrap().snapshot(),
            dialog: self.dialog.as_ref().unwrap().snapshot(),
            panels: self.panels.as_ref().unwrap().snapshot(),
        }
    }
}

fn mount(tree: &mut Tree) -> Handle<Owner> {
    tree.mount_persistent(|cx| {
        let saved: Option<Saved> = cx.restored();
        let (stack, dialog, panels) = match saved {
            Some(saved) => (Some(saved.stack), Some(saved.dialog), Some(saved.panels)),
            None => (None, None, None),
        };
        Owner {
            stack: Some(Stack::restore_or(stack, 0, cx, cell)),
            dialog: Some(Slot::restore(dialog, cx, cell)),
            panels: Some(Panels::restore_or(
                panels,
                0,
                PanelsMode::Triple,
                cx,
                cell,
                cell,
                cell,
            )),
        }
    })
}

/// What the replaced routers' entries hold after a restart.
struct Relaunched {
    stack: u32,
    dialog: u32,
    panels: [u32; 3],
}

/// Replaces every router, opens an entry in each, gives each entry its own
/// value, and returns what the entries hold after a restart.
fn relaunched_after_replacing() -> Relaunched {
    let mut tree = Tree::new(NullHost);
    let owner = mount(&mut tree);
    tree.send(owner, Msg::Replace).unwrap();
    tree.send(owner, Msg::Open).unwrap();
    let open = cells(&tree, owner);
    for (cell, saved) in open.into_iter().zip([11, 22, 30, 33, 44]) {
        tree.send(cell, saved).unwrap();
    }
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let owner = mount(&mut tree);
    let [stack, dialog, main, details, extra] = cells(&tree, owner).map(|cell| value(&tree, cell));
    Relaunched {
        stack,
        dialog,
        panels: [main, details, extra],
    }
}

/// The open entries: the stack's top, the dialog, and the three panels.
fn cells(tree: &Tree, owner: Handle<Owner>) -> [Handle<Cell>; 5] {
    let owner = tree.get(owner).unwrap();
    let panels = owner.panels.as_ref().unwrap();
    [
        *owner.stack.as_ref().unwrap().active(),
        *owner.dialog.as_ref().unwrap().active().unwrap(),
        *panels.main().unwrap(),
        *panels.details().unwrap(),
        *panels.extra().unwrap(),
    ]
}

#[test]
fn a_replaced_stack_keeps_the_saved_state_of_its_entries() {
    assert_eq!(relaunched_after_replacing().stack, 11);
}

#[test]
fn a_replaced_slot_keeps_the_saved_state_of_its_entry() {
    assert_eq!(relaunched_after_replacing().dialog, 22);
}

#[test]
fn replaced_panels_keep_the_saved_state_of_each_panel() {
    assert_eq!(relaunched_after_replacing().panels, [30, 33, 44]);
}

/// A stack its owner can replace with one restored from its own snapshot, or
/// close for good.
struct Screens {
    stack: Option<Stack<u32, Handle<Cell>, Screens>>,
}

enum ScreensMsg {
    Push(u32),
    Pop,
    Replace,
    ReplaceAndClose,
}

impl Component for Screens {
    type Msg = ScreensMsg;
    type Output = ();
    fn update(&mut self, msg: ScreensMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ScreensMsg::Push(route) => self.stack.as_mut().unwrap().push(route, cx),
            ScreensMsg::Pop => {
                self.stack.as_mut().unwrap().pop(cx).unwrap();
            }
            ScreensMsg::Replace => {
                let old = self.stack.take().unwrap();
                let saved = Some(old.snapshot());
                self.stack =
                    Some(cx.replace_router(old, |cx| Stack::restore_or(saved, 0, cx, cell)));
            }
            ScreensMsg::ReplaceAndClose => {
                let old = self.stack.take().unwrap();
                let saved = Some(old.snapshot());
                let fresh = cx.replace_router(old, |cx| Stack::restore_or(saved, 0, cx, cell));
                compono::router::Router::destroy(fresh, cx);
            }
        }
    }
}

impl Persist for Screens {
    type Saved = Option<SavedStack<u32>>;
    fn save(&self) -> Self::Saved {
        self.stack.as_ref().map(Stack::snapshot)
    }
}

fn mount_screens(tree: &mut Tree) -> Handle<Screens> {
    tree.mount_persistent(|cx| {
        let saved: Option<Option<SavedStack<u32>>> = cx.restored();
        Screens {
            stack: Some(Stack::restore_or(saved.flatten(), 0, cx, cell)),
        }
    })
}

fn active(tree: &Tree, screens: Handle<Screens>) -> Handle<Cell> {
    *tree.get(screens).unwrap().stack.as_ref().unwrap().active()
}

/// Leaves three screens holding 100, 101 and 102, and returns the saved bytes.
fn three_screens() -> Vec<u8> {
    let mut tree = Tree::new(NullHost);
    let screens = mount_screens(&mut tree);
    let root = active(&tree, screens);
    tree.send(root, 100).unwrap();
    for (route, saved) in [(1, 101), (2, 102)] {
        tree.send(screens, ScreensMsg::Push(route)).unwrap();
        let top = active(&tree, screens);
        tree.send(top, saved).unwrap();
    }
    tree.save(1).unwrap()
}

#[test]
fn a_restored_stack_replaced_before_its_parked_entries_are_shown_keeps_their_state() {
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &three_screens(), 1);
    let screens = mount_screens(&mut tree);
    assert_eq!(value(&tree, active(&tree, screens)), 102);

    tree.send(screens, ScreensMsg::Replace).unwrap();
    tree.send(screens, ScreensMsg::Pop).unwrap();
    assert_eq!(value(&tree, active(&tree, screens)), 101);
    tree.send(screens, ScreensMsg::Pop).unwrap();
    assert_eq!(value(&tree, active(&tree, screens)), 100);
}

#[test]
fn a_router_destroyed_without_a_replacement_leaves_no_saved_state() {
    // The replacement keeps the parked entries' state, and destroying it then
    // must discard that state as destroying any router does.
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &three_screens(), 1);
    let screens = mount_screens(&mut tree);
    tree.send(screens, ScreensMsg::ReplaceAndClose).unwrap();
    let closed = tree.save(1).unwrap();

    let mut fresh = Tree::new(NullHost);
    let empty = mount_screens(&mut fresh);
    fresh.send(empty, ScreensMsg::ReplaceAndClose).unwrap();
    assert_eq!(
        closed,
        fresh.save(1).unwrap(),
        "nothing of the old screens is carried"
    );
}

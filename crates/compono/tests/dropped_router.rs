//! A router or scope that is dropped without being destroyed still removes its
//! components, once the operation that dropped it ends.

#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::slot::Slot;
use compono::router::{Router, RouterId, Scope};
use compono::{Component, Cx, Handle, Phase, Tree};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<&'static str>>>;

/// A dialog that handles Back and records its last phase and its removal.
struct Dialog {
    log: Log,
}

impl Component for Dialog {
    type Msg = ();
    type Output = ();
    /// Asks its owner to close it.
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.emit(());
    }
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        if phase == Phase::Hidden {
            self.log.borrow_mut().push("hidden");
        }
    }
    fn back(&self) -> Option<()> {
        Some(())
    }
}

impl Drop for Dialog {
    fn drop(&mut self) {
        self.log.borrow_mut().push("dropped");
    }
}

fn dialog(log: &Log, cx: &mut Cx<'_, Menu>) -> Handle<Dialog> {
    let log = Rc::clone(log);
    cx.child(
        move |_cx| Dialog { log },
        |menu: &mut Menu, (), _cx| {
            menu.dialog = None;
        },
    )
}

enum MenuMsg {
    Show,
    /// Replaces the slot and drops the old one.
    Drop,
    /// Destroys the slot, then drops it.
    Destroy,
}

struct Menu {
    dialog: Option<Slot<u32, Handle<Dialog>, Menu>>,
    log: Log,
}

impl Menu {
    fn slot(log: &Log, cx: &mut Cx<'_, Self>) -> Slot<u32, Handle<Dialog>, Self> {
        let log = Rc::clone(log);
        Slot::new(cx, move |_route, cx| dialog(&log, cx))
    }
}

impl Component for Menu {
    type Msg = MenuMsg;
    type Output = ();
    fn update(&mut self, msg: MenuMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            MenuMsg::Show => {
                if let Some(slot) = &mut self.dialog {
                    slot.show(1, cx);
                }
            }
            MenuMsg::Drop => self.dialog = None,
            MenuMsg::Destroy => {
                if let Some(slot) = self.dialog.take() {
                    slot.destroy(cx);
                }
            }
        }
    }
}

fn open_menu() -> (Tree, Handle<Menu>, Handle<Dialog>, Log) {
    let log = Log::default();
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let menu = tree.mount(|cx| Menu {
        dialog: Some(Menu::slot(&log, cx)),
        log: Rc::clone(&log),
    });
    tree.send(menu, MenuMsg::Show).unwrap();
    let open = tree.get(menu).unwrap().dialog.as_ref().unwrap();
    let dialog = *open.active().unwrap();
    assert!(tree.get(menu).unwrap().log.borrow().is_empty());
    (tree, menu, dialog, log)
}

#[test]
fn a_dropped_slot_removes_its_dialog() {
    let (mut tree, menu, dialog, log) = open_menu();

    tree.send(menu, MenuMsg::Drop).unwrap();

    assert!(tree.get(dialog).is_none(), "the dialog is gone");
    assert_eq!(*log.borrow(), ["hidden", "dropped"]);
}

#[test]
fn a_dropped_slot_no_longer_handles_back() {
    let (mut tree, menu, _dialog, _log) = open_menu();

    tree.send(menu, MenuMsg::Drop).unwrap();

    assert!(!tree.back(), "nothing is left to handle Back");
}

#[test]
fn a_slot_dropped_by_queued_work_removes_its_dialog() {
    let (mut tree, _menu, dialog, log) = open_menu();

    // The dialog's output reaches the menu as queued work, which drops the slot.
    tree.send(dialog, ()).unwrap();

    assert!(tree.get(dialog).is_none());
    assert_eq!(*log.borrow(), ["hidden", "dropped"]);
}

#[test]
fn a_destroyed_slot_removes_its_dialog_once() {
    let (mut tree, menu, dialog, log) = open_menu();

    tree.send(menu, MenuMsg::Destroy).unwrap();

    assert!(tree.get(dialog).is_none());
    assert_eq!(*log.borrow(), ["hidden", "dropped"]);
}

/// A custom router that keeps one scope and drops it on any message.
struct Solo {
    router: RouterId,
    shown: Option<(Handle<Leaf>, Scope)>,
}

struct Leaf;
impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Component for Solo {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        self.shown = None;
    }
}

#[test]
fn a_dropped_scope_removes_its_components() {
    let mut tree = Tree::new(NullHost);
    let solo = tree.mount(|cx| {
        let router = cx.router();
        cx.forget_router(router);
        let shown = cx.scoped(router, 0, |cx| cx.child(|_cx| Leaf, |_solo, (), _cx| {}));
        Solo {
            router,
            shown: Some(shown),
        }
    });
    let leaf = tree.get(solo).unwrap().shown.as_ref().unwrap().0;
    assert!(tree.get(solo).unwrap().router.is_saved());

    tree.send(solo, ()).unwrap();

    assert!(tree.get(leaf).is_none());
}

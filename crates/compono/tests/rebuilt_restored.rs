//! A router rebuilt from a restored snapshot keeps the saved state of entries
//! that were restored but not yet shown, and a router destroyed without a
//! replacement leaves none behind.

#![cfg(feature = "serde")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::Router;
use compono::router::set::{ChildSet, SavedChildSet};
use compono::router::stack::{SavedStack, Stack};
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

fn cell<P: Component>(_route: &u32, cx: &mut Cx<'_, P>) -> Handle<Cell> {
    cx.persistent_child(|cx| Cell(cx.restored().unwrap_or(0)), |_owner, (), _cx| {})
}

fn value(tree: &Tree, cell: Handle<Cell>) -> u32 {
    tree.get(cell).unwrap().0
}

/// A stack its owner can rebuild from its own snapshot, or close for good.
struct Screens {
    stack: Option<Stack<u32, Handle<Cell>, Screens>>,
}

enum ScreensMsg {
    Push(u32),
    Pop,
    Rebuild,
    Close,
}

impl Component for Screens {
    type Msg = ScreensMsg;
    type Output = ();
    fn update(&mut self, msg: ScreensMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ScreensMsg::Push(route) => {
                if let Some(stack) = &mut self.stack {
                    stack.push(route, cx);
                }
            }
            ScreensMsg::Pop => {
                if let Some(stack) = &mut self.stack {
                    stack.pop(cx).unwrap();
                }
            }
            ScreensMsg::Rebuild => {
                let Some(old) = self.stack.take() else {
                    return;
                };
                let saved = Some(old.snapshot());
                self.stack =
                    Some(cx.replace_router(old, |cx| Stack::restore_or(saved, 0, cx, cell)));
            }
            ScreensMsg::Close => {
                if let Some(old) = self.stack.take() {
                    old.destroy(cx);
                }
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
fn a_rebuilt_stack_gives_a_restored_screen_its_state_when_shown() {
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &three_screens(), 1);
    let screens = mount_screens(&mut tree);
    assert_eq!(value(&tree, active(&tree, screens)), 102);

    tree.send(screens, ScreensMsg::Rebuild).unwrap();
    tree.send(screens, ScreensMsg::Pop).unwrap();
    assert_eq!(value(&tree, active(&tree, screens)), 101);
    tree.send(screens, ScreensMsg::Pop).unwrap();
    assert_eq!(value(&tree, active(&tree, screens)), 100);
}

#[test]
fn a_stack_destroyed_without_a_replacement_leaves_no_saved_state() {
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &three_screens(), 1);
    let screens = mount_screens(&mut tree);
    tree.send(screens, ScreensMsg::Close).unwrap();
    let closed = tree.save(1).unwrap();

    let mut fresh = Tree::new(NullHost);
    let empty = mount_screens(&mut fresh);
    fresh.send(empty, ScreensMsg::Close).unwrap();
    assert_eq!(
        closed,
        fresh.save(1).unwrap(),
        "nothing of the old screens is carried"
    );
}

/// A set its owner can rebuild from its own snapshot.
struct Cards {
    set: Option<ChildSet<u32, Handle<Cell>, Cards>>,
}

enum CardsMsg {
    Show(Vec<u32>),
    Rebuild,
}

impl Component for Cards {
    type Msg = CardsMsg;
    type Output = ();
    fn update(&mut self, msg: CardsMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            CardsMsg::Show(keys) => {
                let desired = keys.into_iter().map(|key| (key, compono::Phase::Focused));
                self.set.as_mut().unwrap().reconcile(desired, cx).unwrap();
            }
            CardsMsg::Rebuild => {
                let old = self.set.take().unwrap();
                let saved = Some(old.snapshot());
                self.set = Some(cx.replace_router(old, |cx| ChildSet::restore(saved, cx, cell)));
            }
        }
    }
}

impl Persist for Cards {
    type Saved = SavedChildSet<u32>;
    fn save(&self) -> Self::Saved {
        self.set.as_ref().unwrap().snapshot()
    }
}

fn mount_cards(tree: &mut Tree) -> Handle<Cards> {
    tree.mount_persistent(|cx| Cards {
        set: Some(ChildSet::restore(cx.restored(), cx, cell)),
    })
}

fn card(tree: &Tree, cards: Handle<Cards>, index: usize) -> Handle<Cell> {
    *tree
        .get(cards)
        .unwrap()
        .set
        .as_ref()
        .unwrap()
        .child_at(index)
        .unwrap()
}

#[test]
fn a_rebuilt_set_gives_a_restored_key_its_state_when_shown() {
    let mut tree = Tree::new(NullHost);
    let cards = mount_cards(&mut tree);
    tree.send(cards, CardsMsg::Show(vec![7])).unwrap();
    let seven = card(&tree, cards, 0);
    tree.send(seven, 77).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let cards = mount_cards(&mut tree);
    tree.send(cards, CardsMsg::Rebuild).unwrap();
    tree.send(cards, CardsMsg::Show(vec![7])).unwrap();
    assert_eq!(value(&tree, card(&tree, cards, 0)), 77);
}

//! Saved state finds its component by key, not by the order components were made.

#![cfg(feature = "serde")]
#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::stack::Stack;
use compono::{Component, Cx, Handle, Persist, Tree};

const SCHEMA: u32 = 1;

/// A number that survives a restart.
struct Cell {
    value: u32,
}

impl Cell {
    fn new(cx: &mut Cx<'_, Self>) -> Self {
        Self {
            value: cx.restored().unwrap_or(0),
        }
    }
}

impl Component for Cell {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, value: u32, _cx: &mut Cx<'_, Self>) {
        self.value = value;
    }
}

impl Persist for Cell {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.value
    }
}

/// Plain children of no saved state, as a later version might add.
struct Banner;
impl Component for Banner {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

struct Screen {
    draft: Handle<Cell>,
    other: Option<Handle<Cell>>,
}

impl Component for Screen {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Persist for Screen {
    type Saved = ();
    fn save(&self) {}
}

fn ignore(_screen: &mut Screen, (): (), _cx: &mut Cx<'_, Screen>) {}

fn value(tree: &Tree, cell: Handle<Cell>) -> u32 {
    tree.get(cell).unwrap().value
}

#[test]
fn a_keyed_child_keeps_its_state_when_children_are_added_before_it() {
    let saved = {
        let mut tree = Tree::new(NullHost);
        let root = tree.mount_persistent(|cx| Screen {
            draft: cx.persistent_child_keyed("draft", Cell::new, ignore),
            other: None,
        });
        let draft = tree.get(root).unwrap().draft;
        tree.send(draft, 42).unwrap();
        tree.save(SCHEMA).unwrap()
    };
    // The next version adds two children before the draft.
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &saved, SCHEMA);
    let root = tree.mount_persistent(|cx| {
        let _banner = cx.child(|_cx| Banner, ignore);
        let other = cx.persistent_child_keyed("other", Cell::new, ignore);
        Screen {
            draft: cx.persistent_child_keyed("draft", Cell::new, ignore),
            other: Some(other),
        }
    });
    let screen = tree.get(root).unwrap();
    assert_eq!(value(&tree, screen.draft), 42);
    assert_eq!(
        value(&tree, screen.other.unwrap()),
        0,
        "a new key starts clean"
    );
}

#[test]
fn a_repeated_key_never_shares_state_with_the_first_child() {
    let saved = {
        let mut tree = Tree::new(NullHost);
        let root = tree.mount_persistent(|cx| Screen {
            draft: cx.persistent_child_keyed("draft", Cell::new, ignore),
            other: Some(cx.persistent_child_keyed("draft", Cell::new, ignore)),
        });
        let screen = tree.get(root).unwrap();
        let (draft, other) = (screen.draft, screen.other.unwrap());
        tree.send(draft, 7).unwrap();
        tree.send(other, 9).unwrap();
        tree.save(SCHEMA).unwrap()
    };
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &saved, SCHEMA);
    let root = tree.mount_persistent(|cx| Screen {
        draft: cx.persistent_child_keyed("draft", Cell::new, ignore),
        other: None,
    });
    let draft = tree.get(root).unwrap().draft;
    assert_eq!(
        value(&tree, draft),
        7,
        "the repeat did not overwrite the first"
    );
}

/// A stack, then many routers, then another stack.
struct Owner {
    first: Stack<u32, Handle<Cell>, Self>,
    last: Stack<u32, Handle<Cell>, Self>,
}

impl Component for Owner {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Persist for Owner {
    type Saved = Option<compono::router::stack::SavedStack<u32>>;
    fn save(&self) -> Self::Saved {
        Some(self.first.snapshot())
    }
}

fn cell(_route: &u32, cx: &mut Cx<'_, Owner>) -> Handle<Cell> {
    cx.persistent_child(Cell::new, |_owner: &mut Owner, (), _cx| {})
}

fn mount_owner(tree: &mut Tree) -> Handle<Owner> {
    tree.mount_persistent(|cx| {
        let saved = cx.restored().flatten();
        let first = Stack::restore_or(saved, 0, cx, cell);
        // Takes every other id, so the next router is the 32 769th.
        for _ in 1..(1 << 15) {
            let _router = cx.router();
        }
        let last = Stack::new(0, cx, cell);
        Owner { first, last }
    })
}

#[test]
fn routers_past_the_limit_never_touch_the_state_of_earlier_routers() {
    let saved = {
        let mut tree = Tree::new(NullHost);
        let root = mount_owner(&mut tree);
        let owner = tree.get(root).unwrap();
        let (first, last) = (*owner.first.active(), *owner.last.active());
        tree.send(first, 7).unwrap();
        tree.send(last, 9).unwrap();
        tree.save(SCHEMA).unwrap()
    };
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &saved, SCHEMA);
    let root = mount_owner(&mut tree);
    let owner = tree.get(root).unwrap();
    assert_eq!(value(&tree, *owner.first.active()), 7);
    assert_eq!(
        value(&tree, *owner.last.active()),
        0,
        "a router past the limit saves nothing"
    );
}

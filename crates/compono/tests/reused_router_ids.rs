//! A router that `Cx::replace_router` replaces several times in one operation
//! keeps its id.

#![cfg(feature = "serde")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::slot::{SavedSlot, Slot};
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

fn cell(_route: &u32, cx: &mut Cx<'_, Pair>) -> Handle<Cell> {
    cx.persistent_child(|cx| Cell(cx.restored().unwrap_or(0)), |_pair, (), _cx| {})
}

/// Two dialogs, constructed in a fixed order.
struct Pair {
    first: Slot<u32, Handle<Cell>, Pair>,
    second: Option<Slot<u32, Handle<Cell>, Pair>>,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    first: SavedSlot<u32>,
    second: SavedSlot<u32>,
}

enum Msg {
    /// Rebuilds the second dialog twice in one operation.
    RebuildSecondTwice,
    ShowSecond(u32),
}

impl Component for Pair {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::RebuildSecondTwice => {
                for _ in 0..2 {
                    let old = self.second.take().unwrap();
                    self.second = Some(cx.replace_router(old, |cx| Slot::new(cx, cell)));
                }
            }
            Msg::ShowSecond(route) => {
                self.second.as_mut().unwrap().show(route, cx);
            }
        }
    }
}

impl Persist for Pair {
    type Saved = Saved;
    fn save(&self) -> Saved {
        Saved {
            first: self.first.snapshot(),
            second: self.second.as_ref().unwrap().snapshot(),
        }
    }
}

fn mount(tree: &mut Tree) -> Handle<Pair> {
    tree.mount_persistent(|cx| {
        let saved: Option<Saved> = cx.restored();
        let (first, second) = match saved {
            Some(saved) => (Some(saved.first), Some(saved.second)),
            None => (None, None),
        };
        Pair {
            first: Slot::restore(first, cx, cell),
            second: Some(Slot::restore(second, cx, cell)),
        }
    })
}

fn set_second(tree: &mut Tree, pair: Handle<Pair>, value: u32) {
    let dialog = *tree
        .get(pair)
        .unwrap()
        .second
        .as_ref()
        .unwrap()
        .active()
        .unwrap();
    tree.send(dialog, value).unwrap();
}

/// Saves, relaunches and returns what the two dialogs hold.
fn relaunch(tree: &Tree) -> (Option<u32>, Option<u32>) {
    let bytes = tree.save(1).unwrap();
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let pair = mount(&mut tree);
    let pair_ref = tree.get(pair).unwrap();
    let first = pair_ref.first.active().and_then(|cell| tree.get(*cell));
    let second = pair_ref
        .second
        .as_ref()
        .and_then(Slot::active)
        .and_then(|cell| tree.get(*cell));
    (first.map(|cell| cell.0), second.map(|cell| cell.0))
}

#[test]
fn a_router_rebuilt_twice_in_one_operation_keeps_its_id() {
    let mut tree = Tree::new(NullHost);
    let pair = mount(&mut tree);
    tree.send(pair, Msg::ShowSecond(2)).unwrap();
    tree.send(pair, Msg::RebuildSecondTwice).unwrap();
    tree.send(pair, Msg::ShowSecond(2)).unwrap();
    set_second(&mut tree, pair, 222);

    let (_first, second) = relaunch(&tree);
    assert_eq!(second, Some(222));
}

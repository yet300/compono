//! Routers of one parent keep their saved state apart, on every path.

#![cfg(feature = "serde")]
#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::items::{Items, SavedItems};
use compono::router::slot::{SavedSlot, Slot};
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist, Tree};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

struct Cell {
    value: u32,
}

impl Component for Cell {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.value = msg;
    }
}

impl Persist for Cell {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.value
    }
}

fn cell(cx: &mut Cx<'_, Owner>) -> Handle<Cell> {
    cx.persistent_child(
        |cx| Cell {
            value: cx.restored().unwrap_or(0),
        },
        |_owner: &mut Owner, (), _cx| {},
    )
}

/// Three routers, always constructed in the same order: items, stack, slot.
struct Owner {
    items: Items<u32, Handle<Cell>, Owner>,
    stack: Stack<u32, Handle<Cell>, Owner>,
    slot: Slot<u32, Handle<Cell>, Owner>,
    fell_back: bool,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    items: SavedItems<u32>,
    stack: SavedStack<u32>,
    slot: SavedSlot<u32>,
}

enum Msg {
    Window(core::ops::Range<usize>),
    Show(u32),
}

impl Component for Owner {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Window(window) => {
                self.items.set_window(window, cx).unwrap();
            }
            Msg::Show(route) => {
                self.slot.show(route, cx);
            }
        }
    }
}

impl Persist for Owner {
    type Saved = Saved;
    fn save(&self) -> Saved {
        Saved {
            items: self.items.snapshot(),
            stack: self.stack.snapshot(),
            slot: self.slot.snapshot(),
        }
    }
}

fn mount(tree: &mut Tree, keys: Vec<u32>) -> Handle<Owner> {
    tree.mount_persistent(move |cx| {
        let saved: Option<Saved> = cx.restored();
        let (items_saved, stack_saved, slot_saved) = match saved {
            Some(saved) => (Some(saved.items), Some(saved.stack), Some(saved.slot)),
            None => (None, None, None),
        };
        let build = |_key: &u32, cx: &mut Cx<'_, Owner>| cell(cx);
        let (items, fell_back) = match Items::restore_or(items_saved, keys.clone(), cx, build) {
            Ok(items) => (items, false),
            Err(_repeated) => {
                // The owner builds the list again from keys that have no repeat.
                let mut unique = keys.clone();
                unique.dedup();
                (Items::new(unique, cx, build), true)
            }
        };
        Owner {
            items,
            stack: Stack::restore_or(stack_saved, 0, cx, build),
            slot: Slot::restore(slot_saved, cx, build),
            fell_back,
        }
    })
}

/// One launch that leaves 111 in the stack's screen and 222 in the dialog.
fn saved_launch() -> Vec<u8> {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree, (0..10).collect());
    tree.send(root, Msg::Window(0..3)).unwrap();
    tree.send(root, Msg::Show(5)).unwrap();
    let screen = *tree.get(root).unwrap().stack.active();
    let dialog = *tree.get(root).unwrap().slot.active().unwrap();
    tree.send(screen, 111).unwrap();
    tree.send(dialog, 222).unwrap();
    tree.save(1).unwrap()
}

/// Returns whether the list fell back, and what the screen and the dialog hold.
fn relaunch(keys: Vec<u32>) -> (bool, u32, Option<u32>) {
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &saved_launch(), 1);
    let root = mount(&mut tree, keys);
    let owner = tree.get(root).unwrap();
    let screen = tree.get(*owner.stack.active()).unwrap().value;
    let dialog = owner.slot.active();
    let dialog = dialog.map(|dialog| tree.get(*dialog).unwrap().value);
    (owner.fell_back, screen, dialog)
}

#[test]
fn every_router_of_one_parent_gets_back_its_own_state() {
    assert_eq!(relaunch((0..10).collect()), (false, 111, Some(222)));
}

#[test]
fn a_list_that_fails_to_restore_leaves_the_later_routers_their_own_state() {
    // Keys 1 and 2 are equal, inside the saved window 0..3.
    let keys = vec![0, 1, 1, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(relaunch(keys), (true, 111, Some(222)));
}

struct Many {
    ids: Vec<String>,
}

impl Component for Many {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

#[test]
fn a_component_has_32767_saved_router_ids_and_then_ids_that_save_nothing() {
    const SAVED: usize = (1 << 15) - 1;
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| Many {
        ids: (0..SAVED + 3)
            .map(|_| {
                let id = cx.router();
                format!("{id:?} {}", id.is_saved())
            })
            .collect(),
    });
    let ids = &tree.get(root).unwrap().ids;
    let distinct: HashSet<&String> = ids.iter().take(SAVED).collect();
    assert_eq!(distinct.len(), SAVED, "every saved id is its own");
    assert!(ids.iter().take(SAVED).all(|id| id.ends_with("true")));
    let past: HashSet<&String> = ids.iter().skip(SAVED).collect();
    assert_eq!(
        past.len(),
        1,
        "every later router shares the id that saves nothing"
    );
    assert!(ids.last().unwrap().ends_with("false"));
    assert!(
        !distinct.contains(ids.last().unwrap()),
        "no saved id repeats"
    );
}

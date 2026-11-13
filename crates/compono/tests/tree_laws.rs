//! Laws of plain children, Back order and saved state after any sequence of operations.

#![cfg(all(feature = "std", feature = "serde"))]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

mod common;

use common::NullHost;
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist, Phase, Tree};
use proptest::prelude::*;
use std::collections::HashSet;

fn phase() -> impl Strategy<Value = Phase> {
    prop_oneof![
        Just(Phase::Hidden),
        Just(Phase::Visible),
        Just(Phase::Focused)
    ]
}

// ---- Plain children and Back ----

struct Kid {
    handles_back: bool,
    presses: u32,
}

enum KidMsg {
    Toggle,
    Back,
}

impl Component for Kid {
    type Msg = KidMsg;
    type Output = ();
    fn update(&mut self, msg: KidMsg, _cx: &mut Cx<'_, Self>) {
        match msg {
            KidMsg::Toggle => self.handles_back = !self.handles_back,
            KidMsg::Back => self.presses += 1,
        }
    }
    fn back(&self) -> Option<KidMsg> {
        self.handles_back.then_some(KidMsg::Back)
    }
}

#[derive(Default)]
struct Parent {
    kids: Vec<Handle<Kid>>,
}

#[derive(Clone, Debug)]
enum Op {
    Add(bool),
    Remove(usize),
    Grant(usize, Phase),
    Toggle(usize),
    Root(Phase),
    Back,
}

enum ParentMsg {
    Add(bool),
    Remove(usize),
    Grant(usize, Phase),
}

impl Component for Parent {
    type Msg = ParentMsg;
    type Output = ();
    fn update(&mut self, msg: ParentMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ParentMsg::Add(handles_back) => {
                let kid = Kid {
                    handles_back,
                    presses: 0,
                };
                self.kids.push(cx.child(|_cx| kid, |_parent, (), _cx| {}));
            }
            ParentMsg::Remove(index) => cx.destroy(self.kids.remove(index)).unwrap(),
            ParentMsg::Grant(index, phase) => cx.set_child_phase(self.kids[index], phase).unwrap(),
        }
    }
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        any::<bool>().prop_map(Op::Add),
        (0usize..8).prop_map(Op::Remove),
        (0usize..8, phase()).prop_map(|(index, phase)| Op::Grant(index, phase)),
        (0usize..8).prop_map(Op::Toggle),
        phase().prop_map(Op::Root),
        Just(Op::Back)
    ]
}

/// What the law expects of one child.
struct Expected {
    handle: Handle<Kid>,
    granted: Phase,
    handles_back: bool,
    /// When it was last shown, which is the order Back follows.
    shown: u32,
    presses: u32,
}

proptest! {
    #[test]
    fn a_child_never_exceeds_its_parent_and_back_goes_to_the_last_shown_handler(
        ops in prop::collection::vec(op(), 0..60),
    ) {
        let mut tree = Tree::new(NullHost);
        let root = tree.mount(|_cx| Parent::default());
        let mut root_phase = Phase::Hidden;
        let mut kids: Vec<Expected> = Vec::new();
        let mut clock = 0;
        for op in ops {
            match op {
                Op::Add(handles_back) => {
                    tree.send(root, ParentMsg::Add(handles_back)).unwrap();
                    let handle = *tree.get(root).unwrap().kids.last().unwrap();
                    clock += 1;
                    kids.push(Expected {
                        handle,
                        granted: Phase::Focused,
                        handles_back,
                        shown: clock,
                        presses: 0,
                    });
                }
                Op::Remove(index) if index < kids.len() => {
                    tree.send(root, ParentMsg::Remove(index)).unwrap();
                    let gone = kids.remove(index);
                    prop_assert!(tree.get(gone.handle).is_none());
                }
                Op::Grant(index, phase) if index < kids.len() => {
                    tree.send(root, ParentMsg::Grant(index, phase)).unwrap();
                    let kid = &mut kids[index];
                    if kid.granted < Phase::Visible && phase >= Phase::Visible {
                        clock += 1;
                        kid.shown = clock;
                    }
                    kid.granted = phase;
                }
                Op::Toggle(index) if index < kids.len() => {
                    tree.send(kids[index].handle, KidMsg::Toggle).unwrap();
                    kids[index].handles_back = !kids[index].handles_back;
                }
                Op::Root(phase) => {
                    tree.set_phase(phase);
                    root_phase = phase;
                }
                Op::Back => {
                    let target = kids
                        .iter_mut()
                        .filter(|kid| kid.handles_back && kid.granted >= Phase::Visible)
                        .max_by_key(|kid| kid.shown)
                        .filter(|_| root_phase >= Phase::Visible);
                    let expected = target.is_some();
                    if let Some(kid) = target {
                        kid.presses += 1;
                    }
                    prop_assert_eq!(tree.back(), expected);
                }
                Op::Remove(_) | Op::Grant(..) | Op::Toggle(_) => {}
            }
            for kid in &kids {
                prop_assert_eq!(tree.phase(kid.handle), Some(kid.granted.min(root_phase)));
                prop_assert_eq!(tree.get(kid.handle).unwrap().presses, kid.presses);
            }
        }
    }
}

// ---- Saved state ----

const MARK: [u8; 32] = [0xA5; 32];

/// Holds a payload that can be counted in saved bytes.
struct Cell;

impl Component for Cell {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Persist for Cell {
    type Saved = [u8; 32];
    fn save(&self) -> [u8; 32] {
        MARK
    }
}

struct Screens {
    stack: Stack<u8, Handle<Cell>, Self>,
}

#[derive(Clone, Debug)]
enum Nav {
    Push(u8),
    Pop,
    PopTo(usize),
    Replace(u8),
    Reset(u8),
    BringToFront(u8),
    PushToFront(u8),
    ReplaceAll(Vec<u8>),
    Restart,
}

impl Component for Screens {
    type Msg = Nav;
    type Output = ();
    fn update(&mut self, nav: Nav, cx: &mut Cx<'_, Self>) {
        match nav {
            Nav::Push(route) => self.stack.push(route, cx),
            Nav::Pop => {
                let _at_root = self.stack.pop(cx);
            }
            Nav::PopTo(depth) => {
                self.stack.pop_to(depth, cx);
            }
            Nav::Replace(route) => {
                self.stack.replace(route, cx);
            }
            Nav::Reset(route) => self.stack.reset(route, cx),
            Nav::BringToFront(route) => self.stack.bring_to_front(route, cx),
            Nav::PushToFront(route) => self.stack.push_to_front(route, cx),
            Nav::ReplaceAll(routes) => {
                let _empty = self.stack.replace_all(routes, cx);
            }
            Nav::Restart => {}
        }
    }
}

impl Persist for Screens {
    type Saved = SavedStack<u8>;
    fn save(&self) -> SavedStack<u8> {
        self.stack.snapshot()
    }
}

fn cell(cx: &mut Cx<'_, Screens>) -> Handle<Cell> {
    cx.persistent_child(|_cx| Cell, |_parent, (), _cx| {})
}

fn screens(
    tree: &mut Tree,
    tamper: impl FnOnce(SavedStack<u8>) -> SavedStack<u8>,
) -> Handle<Screens> {
    tree.mount_persistent(|cx| {
        let saved: Option<SavedStack<u8>> = cx.restored();
        Screens {
            stack: Stack::restore_or(saved.map(tamper), 0, cx, |_route, cx| cell(cx)),
        }
    })
}

fn nav() -> impl Strategy<Value = Nav> {
    prop_oneof![
        3 => (0u8..4).prop_map(Nav::Push),
        2 => Just(Nav::Pop),
        1 => (0usize..5).prop_map(Nav::PopTo),
        1 => (0u8..4).prop_map(Nav::Replace),
        1 => (0u8..4).prop_map(Nav::Reset),
        1 => (0u8..4).prop_map(Nav::BringToFront),
        1 => (0u8..4).prop_map(Nav::PushToFront),
        1 => prop::collection::vec(0u8..4, 0..4).prop_map(Nav::ReplaceAll),
        2 => Just(Nav::Restart)
    ]
}

fn marks(bytes: &[u8]) -> usize {
    bytes
        .windows(MARK.len())
        .filter(|window| *window == MARK)
        .count()
}

fn unique_keys(tree: &Tree, root: Handle<Screens>) -> bool {
    let stack = &tree.get(root).unwrap().stack;
    let keys: HashSet<u64> = stack.entries().map(|(key, _route)| key).collect();
    keys.len() == stack.depth()
}

proptest! {
    /// Saved bytes hold the state of entries that exist, built or waiting, and
    /// of nothing else. A waiting entry is counted by its key, which says
    /// whether it was ever built.
    #[test]
    fn saved_state_never_names_a_removed_entry(ops in prop::collection::vec(nav(), 0..40)) {
        let mut tree = Tree::new(NullHost);
        let mut root = screens(&mut tree, |saved| saved);
        // The keys of entries that have been built at some time.
        let mut built: HashSet<u64> = HashSet::from([0]);
        for op in ops {
            if matches!(op, Nav::Restart) {
                let bytes = tree.save(1).unwrap();
                let (restored, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
                tree = restored;
                root = screens(&mut tree, |saved| saved);
            } else {
                tree.send(root, op).unwrap();
            }
            let stack = &tree.get(root).unwrap().stack;
            built.insert(stack.active_key());
            let present: HashSet<u64> = stack.entries().map(|(key, _route)| key).collect();
            built.retain(|key| present.contains(key));
            prop_assert!(unique_keys(&tree, root));
            prop_assert_eq!(marks(&tree.save(1).unwrap()), built.len());
        }
    }

    /// Whatever keys and counter a saved stack holds, each entry has its own key
    /// afterwards, so no entry is given the state of another.
    #[test]
    fn damaged_saved_state_never_gives_two_entries_one_key(
        keys in prop::collection::vec(prop_oneof![0u64..6, Just(u64::MAX), Just(1 << 48)], 1..6),
        counter in prop_oneof![0u64..8, any::<u64>()],
        pushes in 0usize..4,
    ) {
        let mut tree = Tree::new(NullHost);
        let root = screens(&mut tree, |saved| saved);
        tree.send(root, Nav::Push(1)).unwrap();
        let bytes = tree.save(1).unwrap();
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let root = screens(&mut tree, move |_saved| {
            let entries: Vec<(u64, u8)> = keys.into_iter().map(|key| (key, 0)).collect();
            let bytes = postcard::to_allocvec(&(entries, counter)).unwrap();
            postcard::from_bytes(&bytes).unwrap()
        });
        prop_assert!(unique_keys(&tree, root));
        for _ in 0..pushes {
            tree.send(root, Nav::Push(2)).unwrap();
            prop_assert!(unique_keys(&tree, root));
        }
        tree.send(root, Nav::PopTo(1)).unwrap();
        prop_assert!(unique_keys(&tree, root));
    }

    /// Bytes changed anywhere, or cut short, give a tree that still works.
    #[test]
    fn changed_or_shortened_bytes_never_break_a_restore(
        ops in prop::collection::vec(nav(), 0..12),
        flips in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 0..4),
        cut in prop::option::of(any::<prop::sample::Index>()),
    ) {
        let mut tree = Tree::new(NullHost);
        let root = screens(&mut tree, |saved| saved);
        for op in ops {
            tree.send(root, op).unwrap();
        }
        let mut bytes = tree.save(1).unwrap();
        for (at, value) in flips {
            let at = at.index(bytes.len());
            bytes[at] = value;
        }
        if let Some(cut) = cut {
            bytes.truncate(cut.index(bytes.len()));
        }
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let root = screens(&mut tree, |saved| saved);
        prop_assert!(unique_keys(&tree, root));
        tree.send(root, Nav::Push(3)).unwrap();
        tree.send(root, Nav::PopTo(1)).unwrap();
        prop_assert!(unique_keys(&tree, root));
        prop_assert!(tree.save(1).is_ok());
    }
}

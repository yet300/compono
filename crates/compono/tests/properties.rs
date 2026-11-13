//! Laws that must hold after any sequence of operations, checked on generated sequences.

#![cfg(all(feature = "std", feature = "serde"))]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::set::ChildSet;
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist, Phase, Tree};
use proptest::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts live instances, so a leaked or doubly dropped child shows up.
struct Leaf {
    live: Arc<AtomicUsize>,
}

impl Leaf {
    fn new(live: &Arc<AtomicUsize>) -> Self {
        live.fetch_add(1, Ordering::Relaxed);
        Self {
            live: Arc::clone(live),
        }
    }
}

impl Drop for Leaf {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

struct Set {
    set: ChildSet<u8, Handle<Leaf>, Self>,
}

impl Component for Set {
    type Msg = Vec<(u8, Phase)>;
    type Output = ();
    fn update(&mut self, desired: Vec<(u8, Phase)>, cx: &mut Cx<'_, Self>) {
        self.set.reconcile(desired, cx).unwrap();
    }
}

fn phase() -> impl Strategy<Value = Phase> {
    prop_oneof![
        Just(Phase::Hidden),
        Just(Phase::Visible),
        Just(Phase::Focused)
    ]
}

/// A desired list: distinct keys from a small range, so lists overlap often.
fn desired() -> impl Strategy<Value = Vec<(u8, Phase)>> {
    prop::collection::btree_map(0..12_u8, phase(), 0..10)
        .prop_map(|map| map.into_iter().collect::<Vec<_>>())
        .prop_shuffle()
}

proptest! {
    #[test]
    fn a_set_matches_every_desired_list_and_keeps_survivors(
        steps in prop::collection::vec(desired(), 1..12),
    ) {
        let live = Arc::new(AtomicUsize::new(0));
        let mut tree = Tree::new(NullHost);
        tree.set_phase(Phase::Focused);
        let counter = Arc::clone(&live);
        let root = tree.mount(move |cx| Set {
            set: ChildSet::new(cx, move |_key, cx| {
                let counter = Arc::clone(&counter);
                cx.child(move |_cx| Leaf::new(&counter), |_set, (), _cx| {})
            }),
        });
        let mut previous: HashMap<u8, Handle<Leaf>> = HashMap::new();
        for wanted in steps {
            tree.send(root, wanted.clone()).unwrap();
            let set = &tree.get(root).unwrap().set;
            let shown: Vec<(u8, Phase)> =
                set.iter().map(|(key, _, phase)| (*key, phase)).collect();
            prop_assert_eq!(&shown, &wanted, "order and phases follow the desired list");
            prop_assert_eq!(live.load(Ordering::Relaxed), wanted.len(), "one live child per key");
            let now: HashMap<u8, Handle<Leaf>> =
                set.iter().map(|(key, handle, _)| (*key, *handle)).collect();
            for (key, handle) in &now {
                prop_assert!(tree.get(*handle).is_some());
                if let Some(before) = previous.get(key) {
                    prop_assert_eq!(before, handle, "a surviving key keeps its child");
                }
            }
            for (key, handle) in &previous {
                if !now.contains_key(key) {
                    prop_assert!(tree.get(*handle).is_none(), "a removed key's child is gone");
                }
            }
            previous = now;
        }
        drop(tree);
        prop_assert_eq!(live.load(Ordering::Relaxed), 0, "dropping the tree drops every child");
    }
}

#[derive(Clone, Debug)]
enum Op {
    Push(u8),
    Pop,
    Replace(u8),
    PopTo(usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => any::<u8>().prop_map(Op::Push),
        2 => Just(Op::Pop),
        1 => any::<u8>().prop_map(Op::Replace),
        1 => (0..6_usize).prop_map(Op::PopTo),
    ]
}

struct Screens {
    stack: Stack<u8, (), Self>,
}

impl Screens {
    fn new(cx: &mut Cx<'_, Self>) -> Self {
        let saved = cx.restored();
        Self {
            stack: Stack::restore_or(saved, 0, cx, |_route, _cx| ()),
        }
    }
}

impl Component for Screens {
    type Msg = Op;
    type Output = ();
    fn update(&mut self, op: Op, cx: &mut Cx<'_, Self>) {
        match op {
            Op::Push(route) => self.stack.push(route, cx),
            Op::Pop => {
                let _at_root = self.stack.pop(cx);
            }
            Op::Replace(route) => {
                let _old = self.stack.replace(route, cx);
            }
            Op::PopTo(depth) => {
                self.stack.pop_to(depth, cx);
            }
        }
    }
}

impl Persist for Screens {
    type Saved = SavedStack<u8>;
    fn save(&self) -> SavedStack<u8> {
        self.stack.snapshot()
    }
}

fn entries(tree: &Tree, root: Handle<Screens>) -> Vec<(u64, u8)> {
    let stack = &tree.get(root).unwrap().stack;
    stack.entries().map(|(key, route)| (key, *route)).collect()
}

proptest! {
    #[test]
    fn a_stack_follows_a_vec_and_survives_a_restore(ops in prop::collection::vec(op(), 0..40)) {
        let mut tree = Tree::new(NullHost);
        let root = tree.mount_persistent(Screens::new);
        let mut model: Vec<u8> = vec![0];
        for op in ops {
            match op {
                Op::Push(route) => model.push(route),
                Op::Pop if model.len() > 1 => {
                    model.pop();
                }
                Op::Pop => {}
                Op::Replace(route) => {
                    if let Some(top) = model.last_mut() {
                        *top = route;
                    }
                }
                Op::PopTo(depth) => model.truncate(depth.max(1)),
            }
            tree.send(root, op).unwrap();
            let routes: Vec<u8> = entries(&tree, root).into_iter().map(|(_, r)| r).collect();
            prop_assert_eq!(&routes, &model);
        }
        let before = entries(&tree, root);
        let mut keys: Vec<u64> = before.iter().map(|(key, _)| *key).collect();
        keys.sort_unstable();
        keys.dedup();
        prop_assert_eq!(keys.len(), before.len(), "every visit has its own key");

        let bytes = tree.save(1).unwrap();
        let (mut again, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let restored = again.mount_persistent(Screens::new);
        prop_assert_eq!(&entries(&again, restored), &before, "keys and routes survive a restore");
        again.send(restored, Op::Push(7)).unwrap();
        let after = entries(&again, restored);
        let fresh = after.last().map(|(key, _)| *key);
        prop_assert!(before.iter().all(|(key, _)| Some(*key) != fresh), "a new visit reuses no key");
    }
}

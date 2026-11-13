//! Behaviour of `Slot`, `ChildSet` and the wider `Stack` operations.
//!
//! Only the saved-state tests need a feature, so the rest also run with
//! `--no-default-features`.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

mod common;

use common::NullHost;
#[cfg(feature = "serde")]
use compono::Persist;
use compono::router::set::{ChildSet, DuplicateKey};
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono::{Component, Cx, Handle, Phase, Tree};
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<String>>>;

struct Leaf {
    name: u32,
    log: Log,
}

impl Leaf {
    fn new(name: u32, log: &Log) -> Self {
        log.lock().unwrap().push(format!("create {name}"));
        Self {
            name,
            log: Arc::clone(log),
        }
    }
}

impl Drop for Leaf {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(format!("drop {}", self.name));
    }
}

impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{} {phase:?}", self.name));
    }
}

#[cfg(feature = "serde")]
impl Persist for Leaf {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.name * 100
    }
}

enum HostMsg {
    Show(u32),
    Close,
    Want(Vec<(u32, Phase)>),
    Push(u32),
    Replace(u32),
    Reset(u32),
    PopTo(usize),
}

struct Parent {
    slot: Slot<u32, Handle<Leaf>, Parent>,
    set: ChildSet<u32, Handle<Leaf>, Parent>,
    stack: Stack<u32, Handle<Leaf>, Parent>,
    rejected: bool,
}

impl Parent {
    fn new(cx: &mut Cx<'_, Self>, log: &Log) -> Self {
        let (a, b, c) = (Arc::clone(log), Arc::clone(log), Arc::clone(log));
        Self {
            slot: Slot::new(cx, move |name, cx| leaf(*name, &a, cx)),
            set: ChildSet::new(cx, move |name, cx| leaf(*name, &b, cx)),
            stack: Stack::new(0, cx, move |name, cx| leaf(*name, &c, cx)),
            rejected: false,
        }
    }
}

fn leaf(name: u32, log: &Log, cx: &mut Cx<'_, Parent>) -> Handle<Leaf> {
    let log = Arc::clone(log);
    cx.child(move |_cx| Leaf::new(name, &log), |_parent, (), _cx| {})
}

impl Component for Parent {
    type Msg = HostMsg;
    type Output = ();
    fn update(&mut self, msg: HostMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            HostMsg::Show(name) => {
                self.slot.show(name, cx);
            }
            HostMsg::Close => {
                self.slot.close(cx);
            }
            HostMsg::Want(desired) => {
                self.rejected = self.set.reconcile(desired, cx) == Err(DuplicateKey);
            }
            HostMsg::Push(name) => self.stack.push(name, cx),
            HostMsg::Replace(name) => {
                self.stack.replace(name, cx);
            }
            HostMsg::Reset(name) => self.stack.reset(name, cx),
            HostMsg::PopTo(depth) => {
                self.stack.pop_to(depth, cx);
            }
        }
    }
}

fn setup() -> (Tree, Handle<Parent>, Log) {
    let log: Log = Arc::default();
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let log_for_root = Arc::clone(&log);
    let root = tree.mount(move |cx| Parent::new(cx, &log_for_root));
    log.lock().unwrap().clear();
    (tree, root, log)
}

fn take(log: &Log) -> Vec<String> {
    core::mem::take(&mut *log.lock().unwrap())
}

#[test]
fn slot_shows_replaces_and_closes() {
    let (mut tree, root, log) = setup();
    tree.send(root, HostMsg::Show(1)).unwrap();
    assert_eq!(take(&log), ["create 1", "1 Focused"]);
    tree.send(root, HostMsg::Show(2)).unwrap();
    assert_eq!(
        take(&log),
        ["1 Hidden", "drop 1", "create 2", "2 Focused"],
        "the old entry is gone before the new one appears",
    );
    assert_eq!(tree.get(root).unwrap().slot.route(), Some(&2));
    tree.send(root, HostMsg::Close).unwrap();
    assert_eq!(take(&log), ["2 Hidden", "drop 2"]);
    assert!(!tree.get(root).unwrap().slot.is_open());
}

#[test]
fn set_keeps_surviving_keys_across_a_reorder() {
    let (mut tree, root, log) = setup();
    let want = |keys: &[u32]| HostMsg::Want(keys.iter().map(|k| (*k, Phase::Visible)).collect());
    tree.send(root, want(&[1, 2, 3])).unwrap();
    let first = tree.get(root).unwrap().set.iter().next().map(|e| *e.1);
    take(&log);
    tree.send(root, want(&[3, 1])).unwrap();
    assert_eq!(take(&log), ["2 Hidden", "drop 2"]);
    let set = &tree.get(root).unwrap().set;
    let keys: Vec<u32> = set.iter().map(|entry| *entry.0).collect();
    assert_eq!(keys, [3, 1]);
    assert_eq!(set.iter().nth(1).map(|e| *e.1), first);
}

#[test]
fn set_lowers_phases_before_raising_them() {
    let (mut tree, root, log) = setup();
    let msg = |a, b| HostMsg::Want(vec![(1, a), (2, b)]);
    tree.send(root, msg(Phase::Focused, Phase::Hidden)).unwrap();
    take(&log);
    tree.send(root, msg(Phase::Hidden, Phase::Focused)).unwrap();
    assert_eq!(take(&log), ["1 Hidden", "2 Focused"]);
}

#[test]
fn set_rejects_duplicates_without_changing_anything() {
    let (mut tree, root, log) = setup();
    tree.send(root, HostMsg::Want(vec![(1, Phase::Visible)]))
        .unwrap();
    take(&log);
    let duplicate = vec![(2, Phase::Visible), (2, Phase::Focused)];
    tree.send(root, HostMsg::Want(duplicate)).unwrap();
    assert!(tree.get(root).unwrap().rejected);
    assert_eq!(tree.get(root).unwrap().set.len(), 1);
    assert!(take(&log).is_empty());
}

#[test]
fn large_set_churn_is_linear_in_child_count() {
    // 20 000 children removed from the front: quadratic bookkeeping would take minutes.
    let (mut tree, root, _log) = setup();
    let all: Vec<_> = (1..=20_000).map(|k| (k, Phase::Visible)).collect();
    tree.send(root, HostMsg::Want(all.clone())).unwrap();
    tree.send(root, HostMsg::Want(all[10_000..].to_vec()))
        .unwrap();
    assert_eq!(tree.get(root).unwrap().set.len(), 10_000);
    tree.send(root, HostMsg::Want(Vec::new())).unwrap();
    assert!(tree.get(root).unwrap().set.is_empty());
}

#[test]
fn stack_replace_reset_and_pop_to() {
    let (mut tree, root, log) = setup();
    tree.send(root, HostMsg::Replace(1)).unwrap();
    assert_eq!(
        take(&log),
        ["create 1", "0 Hidden", "drop 0", "1 Focused"],
        "replace removes the old screen before focusing the new one",
    );
    tree.send(root, HostMsg::Push(2)).unwrap();
    tree.send(root, HostMsg::Push(3)).unwrap();
    tree.send(root, HostMsg::Push(4)).unwrap();
    take(&log);
    tree.send(root, HostMsg::PopTo(2)).unwrap();
    assert_eq!(
        take(&log),
        ["drop 3", "4 Hidden", "drop 4", "2 Focused"],
        "covered entries go without a phase change, already being hidden",
    );
    assert_eq!(tree.get(root).unwrap().stack.depth(), 2);
    tree.send(root, HostMsg::PopTo(0)).unwrap();
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
    tree.send(root, HostMsg::Reset(9)).unwrap();
    assert_eq!(tree.get(root).unwrap().stack.route(), &9);
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
}

#[cfg(feature = "serde")]
mod persistence {
    use super::*;
    use compono::RestoreOutcome;
    use compono::router::set::SavedChildSet;
    use compono::router::slot::SavedSlot;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Saved {
        slot: SavedSlot<u32>,
        set: SavedChildSet<u32>,
    }

    struct Durable {
        slot: Slot<u32, Handle<Stored>, Durable>,
        set: ChildSet<u32, Handle<Stored>, Durable>,
    }

    struct Stored {
        value: u32,
    }

    impl Component for Stored {
        type Msg = u32;
        type Output = ();
        fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
            self.value = msg;
        }
    }

    impl Persist for Stored {
        type Saved = u32;
        fn save(&self) -> u32 {
            self.value
        }
    }

    fn stored(cx: &mut Cx<'_, Durable>) -> Handle<Stored> {
        cx.persistent_child(
            |cx| Stored {
                value: cx.restored().unwrap_or(0),
            },
            |_parent, (), _cx| {},
        )
    }

    enum Msg {
        Open(u32),
        Want(Vec<u32>),
    }

    impl Component for Durable {
        type Msg = Msg;
        type Output = ();
        fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
            match msg {
                Msg::Open(route) => {
                    self.slot.show(route, cx);
                }
                Msg::Want(keys) => {
                    let desired = keys.into_iter().map(|key| (key, Phase::Visible));
                    assert_eq!(self.set.reconcile(desired, cx), Ok(()));
                }
            }
        }
    }

    impl Persist for Durable {
        type Saved = Saved;
        fn save(&self) -> Saved {
            Saved {
                slot: self.slot.snapshot(),
                set: self.set.snapshot(),
            }
        }
    }

    fn mount(tree: &mut Tree) -> Handle<Durable> {
        tree.mount_persistent(|cx| {
            let saved = cx.restored();
            let (slot, set) = saved.map_or((None, None), |s: Saved| (Some(s.slot), Some(s.set)));
            Durable {
                slot: Slot::restore(slot, cx, |_route, cx| stored(cx)),
                set: ChildSet::restore(set, cx, |_key, cx| stored(cx)),
            }
        })
    }

    #[test]
    fn slot_and_set_children_keep_their_state() {
        let mut tree = Tree::new(NullHost);
        let root = mount(&mut tree);
        tree.send(root, Msg::Open(5)).unwrap();
        tree.send(root, Msg::Want(vec![7, 8])).unwrap();
        let sheet = *tree.get(root).unwrap().slot.active().unwrap();
        let eight = tree.get(root).unwrap().set.iter().nth(1).map(|e| *e.1);
        tree.send(sheet, 11).unwrap();
        tree.send(eight.unwrap(), 22).unwrap();
        let bytes = tree.save(1).unwrap();

        let (mut tree, outcome) = Tree::with_saved(NullHost, &bytes, 1);
        assert_eq!(outcome, RestoreOutcome::Restored);
        let root = mount(&mut tree);
        let sheet = *tree.get(root).unwrap().slot.active().unwrap();
        assert_eq!(tree.get(sheet).unwrap().value, 11);
        assert!(tree.get(root).unwrap().set.is_empty(), "built on demand");

        // A second save before the key is desired must not lose it.
        let bytes = tree.save(1).unwrap();
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let root = mount(&mut tree);
        tree.send(root, Msg::Want(vec![8])).unwrap();
        let eight = tree.get(root).unwrap().set.iter().next().map(|e| *e.1);
        assert_eq!(tree.get(eight.unwrap()).unwrap().value, 22);
    }
}

struct Revealing {
    stack: Stack<u32, u32, Self>,
}

impl Component for Revealing {
    type Msg = u32;
    type Output = ();

    fn update(&mut self, msg: u32, cx: &mut Cx<'_, Self>) {
        if msg == 0 {
            self.stack.pop(cx).unwrap();
        } else {
            self.stack.push(msg, cx);
        }
    }
}

#[test]
fn previous_is_the_entry_back_would_reveal() {
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| Revealing {
        stack: Stack::new(10, cx, |route: &u32, _cx| *route),
    });
    let read = |tree: &Tree| tree.get(root).unwrap().stack.previous().map(|(_key, v)| *v);
    assert_eq!(read(&tree), None, "the root reveals nothing");
    tree.send(root, 20).unwrap();
    assert_eq!(read(&tree), Some(10));
    tree.send(root, 30).unwrap();
    assert_eq!(read(&tree), Some(20));
    let (key, _value) = tree.get(root).unwrap().stack.previous().unwrap();
    tree.send(root, 0).unwrap();
    assert_eq!(tree.get(root).unwrap().stack.active_key(), key);
}

#[test]
fn entries_list_every_visit_oldest_first() {
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| Revealing {
        stack: Stack::new(10, cx, |route: &u32, _cx| *route),
    });
    let read = |tree: &Tree| -> Vec<(u64, u32)> {
        let stack = &tree.get(root).unwrap().stack;
        stack.entries().map(|(key, route)| (key, *route)).collect()
    };
    tree.send(root, 20).unwrap();
    tree.send(root, 20).unwrap();
    assert_eq!(read(&tree), [(0, 10), (1, 20), (2, 20)]);
    tree.send(root, 0).unwrap();
    tree.send(root, 30).unwrap();
    assert_eq!(
        read(&tree),
        [(0, 10), (1, 20), (3, 30)],
        "a new visit gets a new key"
    );
}

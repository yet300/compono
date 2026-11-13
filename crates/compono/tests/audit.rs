//! Defects found by the audit before 1.0. Each test failed before its fix.

#![cfg(all(feature = "std", feature = "serde"))]
#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::items::visible_rows;
use compono::router::set::{ChildSet, SavedChildSet};
use compono::router::stack::{SavedStack, Stack};
use compono::{Component, Cx, Handle, Persist, Phase, SendError, Sender, Tree};

#[test]
fn a_viewport_too_large_to_count_shows_the_whole_list() {
    assert_eq!(visible_rows(0.0, f32::INFINITY, 1.0, 10), 0..10);
    assert_eq!(visible_rows(0.0, f32::MAX, 0.001, 10), 0..10);
    assert_eq!(visible_rows(f32::MAX, 100.0, 0.001, 10), 10..10);
}

struct Cell {
    value: Vec<u8>,
}

impl Component for Cell {
    type Msg = Vec<u8>;
    type Output = ();
    fn update(&mut self, msg: Vec<u8>, _cx: &mut Cx<'_, Self>) {
        self.value = msg;
    }
}

impl Persist for Cell {
    type Saved = Vec<u8>;
    fn save(&self) -> Vec<u8> {
        self.value.clone()
    }
}

fn cell<P: Component>(cx: &mut Cx<'_, P>) -> Handle<Cell> {
    cx.persistent_child(
        |cx| Cell {
            value: cx.restored().unwrap_or_default(),
        },
        |_parent, (), _cx| {},
    )
}

/// Rewrites the counter of a saved router, as damaged or hostile bytes could.
fn with_counter<S>(saved: &S, counter: u64) -> S
where
    S: serde::Serialize + serde::de::DeserializeOwned,
{
    let bytes = postcard::to_allocvec(saved).unwrap();
    let (entries, _next): (Vec<(u64, u32)>, u64) = postcard::from_bytes(&bytes).unwrap();
    let bytes = postcard::to_allocvec(&(entries, counter)).unwrap();
    postcard::from_bytes(&bytes).unwrap()
}

struct Screens {
    stack: Stack<u32, Handle<Cell>, Self>,
}

impl Component for Screens {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, route: u32, cx: &mut Cx<'_, Self>) {
        self.stack.push(route, cx);
    }
}

impl Persist for Screens {
    type Saved = SavedStack<u32>;
    fn save(&self) -> SavedStack<u32> {
        self.stack.snapshot()
    }
}

fn screens(tree: &mut Tree, counter: Option<u64>) -> Handle<Screens> {
    tree.mount_persistent(|cx| {
        let saved: Option<SavedStack<u32>> = cx.restored();
        let saved = match (saved, counter) {
            (Some(saved), Some(counter)) => Some(with_counter(&saved, counter)),
            (saved, _) => saved,
        };
        Screens {
            stack: Stack::restore_or(saved, 0, cx, |_route, cx| cell(cx)),
        }
    })
}

#[test]
fn a_saved_stack_with_a_low_counter_does_not_hand_old_state_to_a_new_entry() {
    let mut tree = Tree::new(NullHost);
    let root = screens(&mut tree, None);
    let first = *tree.get(root).unwrap().stack.active();
    tree.send(first, vec![1, 2, 3]).unwrap();
    tree.send(root, 1).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = screens(&mut tree, Some(0));
    tree.send(root, 2).unwrap();
    let stack = &tree.get(root).unwrap().stack;
    let newest = *stack.active();
    let mut keys: Vec<u64> = stack.entries().map(|(key, _route)| key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 3, "every entry has its own key");
    assert!(tree.get(newest).unwrap().value.is_empty());
}

struct Shelf {
    set: ChildSet<u32, Handle<Cell>, Self>,
}

impl Component for Shelf {
    type Msg = Vec<u32>;
    type Output = ();
    fn update(&mut self, keys: Vec<u32>, cx: &mut Cx<'_, Self>) {
        let desired = keys.into_iter().map(|key| (key, Phase::Visible));
        self.set.reconcile(desired, cx).unwrap();
    }
}

impl Persist for Shelf {
    type Saved = SavedChildSet<u32>;
    fn save(&self) -> SavedChildSet<u32> {
        self.set.snapshot()
    }
}

fn shelf(tree: &mut Tree, counter: Option<u64>) -> Handle<Shelf> {
    tree.mount_persistent(|cx| {
        let saved: Option<SavedChildSet<u32>> = cx.restored();
        let saved = match (saved, counter) {
            (Some(saved), Some(counter)) => Some(with_counter(&saved, counter)),
            (saved, _) => saved,
        };
        Shelf {
            set: ChildSet::restore(saved, cx, |_key, cx| cell(cx)),
        }
    })
}

#[test]
fn a_saved_set_with_a_low_counter_does_not_hand_old_state_to_a_new_key() {
    let mut tree = Tree::new(NullHost);
    let root = shelf(&mut tree, None);
    tree.send(root, vec![7]).unwrap();
    let seven = *tree.get(root).unwrap().set.child_at(0).unwrap();
    tree.send(seven, vec![9, 9]).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = shelf(&mut tree, Some(0));
    tree.send(root, vec![8]).unwrap();
    let eight = *tree.get(root).unwrap().set.child_at(0).unwrap();
    assert!(tree.get(eight).unwrap().value.is_empty());
}

struct Listener {
    sender: Sender<Self>,
}

impl Component for Listener {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, _msg: u32, _cx: &mut Cx<'_, Self>) {}
}

#[test]
fn a_sender_that_outlives_its_tree_is_refused() {
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| Listener {
        sender: cx.sender(),
    });
    let sender = tree.get(root).unwrap().sender.clone();
    assert_eq!(sender.send(1), Ok(()));
    drop(tree);
    assert_eq!(sender.send(2), Err(SendError::Closed));
}

struct Link {
    child: Option<Handle<Link>>,
}

impl Component for Link {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.child = Some(cx.child(|_cx| Self { child: None }, |_parent, (), _cx| {}));
    }
    fn back(&self) -> Option<()> {
        None
    }
}

/// A chain built one link at a time, so that building it does not recurse.
fn chain(depth: usize) -> Tree {
    let mut tree = Tree::new(NullHost);
    let mut tip = tree.mount(|_cx| Link { child: None });
    for _ in 0..depth {
        tree.send(tip, ()).unwrap();
        tip = tree.get(tip).unwrap().child.unwrap();
    }
    tree
}

#[test]
fn a_deep_chain_changes_phase_saves_and_is_destroyed_without_overflowing() {
    let mut tree = chain(200_000);
    tree.set_phase(Phase::Focused);
    assert!(!tree.back());
    tree.set_phase(Phase::Hidden);
    assert!(tree.save(1).is_ok());
    drop(tree);
}

mod removal {
    use super::NullHost;
    use compono::router::Router;
    use compono::router::items::Items;
    use compono::router::pages::Pages;
    use compono::router::panels::{PanelTypes, Panels, PanelsMode};
    use compono::router::set::ChildSet;
    use compono::router::slot::Slot;
    use compono::router::stack::Stack;
    use compono::{Component, Cx, Handle, Phase, Tree};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Leaf(Arc<AtomicUsize>);

    impl Component for Leaf {
        type Msg = ();
        type Output = ();
        fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    }

    impl Drop for Leaf {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    struct Three;
    impl PanelTypes for Three {
        type Main = u32;
        type MainChild = Handle<Leaf>;
        type Details = u32;
        type DetailsChild = Handle<Leaf>;
        type Extra = u32;
        type ExtraChild = Handle<Leaf>;
    }

    struct Routers {
        stack: Stack<u32, Handle<Leaf>, Owner>,
        slot: Slot<u32, Handle<Leaf>, Owner>,
        set: ChildSet<u32, Handle<Leaf>, Owner>,
        pages: Pages<u32, Handle<Leaf>, Owner>,
        items: Items<u32, Handle<Leaf>, Owner>,
        panels: Panels<Three, Owner>,
    }

    struct Owner {
        routers: Option<Routers>,
    }

    impl Component for Owner {
        type Msg = ();
        type Output = ();
        fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
            if let Some(routers) = self.routers.take() {
                routers.stack.destroy(cx);
                routers.slot.destroy(cx);
                routers.set.destroy(cx);
                routers.pages.destroy(cx);
                routers.items.destroy(cx);
                routers.panels.destroy(cx);
            }
        }
    }

    fn leaf(live: &Arc<AtomicUsize>, cx: &mut Cx<'_, Owner>) -> Handle<Leaf> {
        live.fetch_add(1, Ordering::SeqCst);
        let live = Arc::clone(live);
        cx.child(move |_cx| Leaf(live), |_owner, (), _cx| {})
    }

    #[test]
    fn a_router_removed_before_its_owner_takes_its_components_with_it() {
        let live = Arc::new(AtomicUsize::new(0));
        let mut tree = Tree::new(NullHost);
        let count = Arc::clone(&live);
        let root = tree.mount(move |cx| {
            let build = |live: &Arc<AtomicUsize>| {
                let live = Arc::clone(live);
                move |_route: &u32, cx: &mut Cx<'_, Owner>| leaf(&live, cx)
            };
            let mut stack = Stack::new(0, cx, build(&count));
            stack.push(1, cx);
            let mut slot = Slot::new(cx, build(&count));
            slot.show(0, cx);
            let mut set = ChildSet::new(cx, build(&count));
            set.reconcile([(0, Phase::Visible), (1, Phase::Hidden)], cx)
                .unwrap();
            let pages = Pages::new(vec![0, 1, 2], 1, 1, cx, build(&count)).unwrap();
            let mut items = Items::new(vec![0, 1, 2, 3], cx, build(&count));
            items.set_window(0..2, cx).unwrap();
            let mut panels = Panels::new(
                0,
                PanelsMode::Triple,
                cx,
                build(&count),
                build(&count),
                build(&count),
            );
            panels.set_details(Some(1), cx);
            Owner {
                routers: Some(Routers {
                    stack,
                    slot,
                    set,
                    pages,
                    items,
                    panels,
                }),
            }
        });
        assert_eq!(live.load(Ordering::SeqCst), 12);
        tree.send(root, ()).unwrap();
        assert_eq!(live.load(Ordering::SeqCst), 0);
    }
}

struct Fresh {
    stack: Stack<u32, Handle<Cell>, Self>,
}

impl Component for Fresh {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, route: u32, cx: &mut Cx<'_, Self>) {
        self.stack.push(route, cx);
    }
}

/// Starts from its first screen on every launch, so it saves no navigation.
fn fresh(tree: &mut Tree) -> Handle<Fresh> {
    tree.mount(|cx| Fresh {
        stack: Stack::new(0, cx, |_route, cx| cell(cx)),
    })
}

#[test]
fn a_router_that_starts_fresh_gives_no_entry_the_state_of_an_earlier_launch() {
    let mut tree = Tree::new(NullHost);
    let root = fresh(&mut tree);
    tree.send(root, 1).unwrap();
    let detail = *tree.get(root).unwrap().stack.active();
    tree.send(detail, vec![4, 2]).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = fresh(&mut tree);
    assert!(
        tree.save(1).unwrap().len() < bytes.len(),
        "state that nothing can reach is not saved again"
    );
    tree.send(root, 9).unwrap();
    let other = *tree.get(root).unwrap().stack.active();
    assert!(tree.get(other).unwrap().value.is_empty());
}

/// Rewrites the entries of a saved stack, as damaged or older bytes could.
fn with_entries(saved: &SavedStack<u32>, entries: &[(u64, u32)], counter: u64) -> SavedStack<u32> {
    let _shape: (Vec<(u64, u32)>, u64) =
        postcard::from_bytes(&postcard::to_allocvec(saved).unwrap()).unwrap();
    let bytes = postcard::to_allocvec(&(entries.to_vec(), counter)).unwrap();
    postcard::from_bytes(&bytes).unwrap()
}

fn screens_from(tree: &mut Tree, entries: &'static [(u64, u32)], counter: u64) -> Handle<Screens> {
    tree.mount_persistent(move |cx| {
        let saved: Option<SavedStack<u32>> = cx.restored();
        let saved = saved.map(|saved| with_entries(&saved, entries, counter));
        Screens {
            stack: Stack::restore_or(saved, 0, cx, |_route, cx| cell(cx)),
        }
    })
}

#[test]
fn saved_state_of_an_entry_the_snapshot_does_not_name_is_not_carried() {
    let mut tree = Tree::new(NullHost);
    let root = screens(&mut tree, None);
    for (route, value) in [(1, 11), (2, 22)] {
        tree.send(root, route).unwrap();
        let active = *tree.get(root).unwrap().stack.active();
        tree.send(active, vec![value]).unwrap();
    }
    let full = tree.save(1).unwrap();

    // Bytes whose snapshot names only the root, while state for keys 1 and 2 remains.
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &full, 1);
    screens_from(&mut tree, &[(0, 0)], 3);
    let trimmed = tree.save(1).unwrap();

    // If that state were carried, naming the keys again would bring it back.
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &trimmed, 1);
    let root = screens_from(&mut tree, &[(0, 0), (1, 1), (2, 2)], 3);
    let active = *tree.get(root).unwrap().stack.active();
    assert!(
        tree.get(active).unwrap().value.is_empty(),
        "state of an entry the snapshot dropped is gone after the next save",
    );
}

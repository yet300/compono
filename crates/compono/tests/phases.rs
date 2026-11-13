//! The phase a component has and reads, and the order siblings change in.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::stack::Stack;
use compono::{Component, Cx, Handle, Phase, Tree};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<(u32, Phase)>>>;

struct Kid {
    name: u32,
    log: Log,
    presses: u32,
    /// What `cx.phase()` returned the last time this component ran.
    read: Option<Phase>,
}

enum KidMsg {
    Read,
    Back,
}

impl Component for Kid {
    type Msg = KidMsg;
    type Output = ();
    fn update(&mut self, msg: KidMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            KidMsg::Read => self.read = Some(cx.phase()),
            KidMsg::Back => self.presses += 1,
        }
    }
    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        self.log.borrow_mut().push((self.name, phase));
        self.read = Some(cx.phase());
    }
    fn back(&self) -> Option<KidMsg> {
        Some(KidMsg::Back)
    }
}

fn kid<P: Component>(name: u32, log: &Log, cx: &mut Cx<'_, P>) -> Handle<Kid> {
    let log = Rc::clone(log);
    cx.child(
        move |_cx| Kid {
            name,
            log,
            presses: 0,
            read: None,
        },
        |_parent: &mut P, (), _cx| {},
    )
}

struct Parent {
    kids: Vec<Handle<Kid>>,
}

impl Component for Parent {
    /// The child to grant a phase, and the phase.
    type Msg = (usize, Phase);
    type Output = ();
    fn update(&mut self, (index, phase): (usize, Phase), cx: &mut Cx<'_, Self>) {
        cx.set_child_phase(*self.kids.get(index).unwrap(), phase)
            .unwrap();
    }
}

/// Children 1, 2 and 3, of which 1 was hidden and shown again, so it was shown last.
fn family() -> (Tree, Handle<Parent>, Log) {
    let log = Log::default();
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let names = Rc::clone(&log);
    let root = tree.mount(move |cx| Parent {
        kids: (1..=3).map(|name| kid(name, &names, cx)).collect(),
    });
    tree.send(root, (0, Phase::Hidden)).unwrap();
    tree.send(root, (0, Phase::Focused)).unwrap();
    log.borrow_mut().clear();
    (tree, root, log)
}

fn names(log: &Log) -> Vec<u32> {
    log.borrow_mut()
        .drain(..)
        .map(|(name, _phase)| name)
        .collect()
}

#[test]
fn siblings_fall_most_recently_shown_first() {
    let (mut tree, _root, log) = family();
    tree.set_phase(Phase::Hidden);
    assert_eq!(names(&log), [1, 3, 2]);
}

#[test]
fn siblings_rise_in_the_order_they_were_shown() {
    let (mut tree, _root, log) = family();
    tree.set_phase(Phase::Hidden);
    log.borrow_mut().clear();
    tree.set_phase(Phase::Focused);
    assert_eq!(names(&log), [2, 3, 1]);
}

#[test]
fn a_child_granted_no_phase_stays_hidden_when_its_parent_rises() {
    let (mut tree, root, log) = family();
    tree.send(root, (1, Phase::Hidden)).unwrap();
    tree.set_phase(Phase::Hidden);
    log.borrow_mut().clear();
    tree.set_phase(Phase::Focused);
    assert_eq!(names(&log), [3, 1]);
    let second = *tree.get(root).unwrap().kids.get(1).unwrap();
    assert_eq!(tree.phase(second), Some(Phase::Hidden));
}

#[test]
fn a_component_reads_its_own_phase_through_cx() {
    let (mut tree, root, _log) = family();
    let first = *tree.get(root).unwrap().kids.first().unwrap();
    for phase in [Phase::Hidden, Phase::Visible, Phase::Focused] {
        tree.set_phase(phase);
        assert_eq!(
            tree.get(first).unwrap().read,
            Some(phase),
            "in phase_changed"
        );
        tree.send(first, KidMsg::Read).unwrap();
        assert_eq!(tree.get(first).unwrap().read, Some(phase), "in update");
    }
    // The parent lowers it, so its own phase is no longer the application's.
    tree.send(root, (0, Phase::Visible)).unwrap();
    tree.send(first, KidMsg::Read).unwrap();
    assert_eq!(tree.get(first).unwrap().read, Some(Phase::Visible));
}

/// A component with two children at every level, which remembers the phase it
/// was last told.
struct Branch {
    kids: Vec<Handle<Branch>>,
    heard: Phase,
}

impl Component for Branch {
    type Msg = (usize, Phase);
    type Output = ();
    fn update(&mut self, (index, phase): (usize, Phase), cx: &mut Cx<'_, Self>) {
        cx.set_child_phase(*self.kids.get(index).unwrap(), phase)
            .unwrap();
    }
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        self.heard = phase;
    }
}

fn branch(levels: u32, cx: &mut Cx<'_, Branch>) -> Branch {
    let kids = (0..2)
        .filter(|_| levels > 0)
        .map(|_| cx.child(move |cx| branch(levels - 1, cx), |_parent, (), _cx| {}))
        .collect();
    Branch {
        kids,
        heard: Phase::Hidden,
    }
}

/// One component of the tree under test.
struct Place {
    handle: Handle<Branch>,
    /// Where its parent is in the list, and which child of that parent it is.
    parent: Option<(usize, usize)>,
    granted: Phase,
}

#[test]
fn every_component_has_and_hears_the_lowest_phase_granted_on_its_path() {
    let mut tree = Tree::new(NullHost);
    let root = tree.mount(|cx| branch(3, cx));
    // Parents come before their children.
    let mut places = vec![Place {
        handle: root,
        parent: None,
        granted: Phase::Focused,
    }];
    let mut next = 0;
    while let Some(place) = places.get(next) {
        let kids = tree.get(place.handle).unwrap().kids.clone();
        for (index, handle) in kids.into_iter().enumerate() {
            places.push(Place {
                handle,
                parent: Some((next, index)),
                granted: Phase::Focused,
            });
        }
        next += 1;
    }
    assert_eq!(places.len(), 15);

    let mut application = Phase::Hidden;
    let mut seed = 1_u32;
    for step in 0..4_000 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let pick = usize::try_from(seed >> 16).unwrap();
        let phase = *[Phase::Hidden, Phase::Visible, Phase::Focused]
            .get(pick % 3)
            .unwrap();
        let chosen = (pick / 3) % places.len();
        match places.get(chosen).unwrap().parent {
            None => {
                tree.set_phase(phase);
                application = phase;
            }
            Some((parent, index)) => {
                let parent = places.get(parent).unwrap().handle;
                tree.send(parent, (index, phase)).unwrap();
                places.get_mut(chosen).unwrap().granted = phase;
            }
        }
        let mut expected: Vec<Phase> = Vec::with_capacity(places.len());
        for place in &places {
            let ceiling = match place.parent {
                None => application,
                Some((parent, _index)) => *expected.get(parent).unwrap(),
            };
            let phase = ceiling.min(place.granted);
            assert_eq!(tree.phase(place.handle), Some(phase), "step {step}");
            assert_eq!(tree.get(place.handle).unwrap().heard, phase, "step {step}");
            expected.push(phase);
        }
    }
}

struct Owner {
    stack: Option<Stack<u32, Handle<Kid>, Owner>>,
}

impl Component for Owner {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        // Dropped, not destroyed.
        self.stack = None;
    }
}

/// An owner that dropped its stack, and the component that stack was showing.
fn dropped_router() -> (Tree, Handle<Owner>, Handle<Kid>) {
    let log = Log::default();
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(move |cx| Owner {
        stack: Some(Stack::new(0, cx, move |route, cx| kid(*route, &log, cx))),
    });
    let shown = *tree.get(root).unwrap().stack.as_ref().unwrap().active();
    tree.send(root, ()).unwrap();
    (tree, root, shown)
}

#[test]
fn a_router_that_is_only_dropped_removes_its_component() {
    let (mut tree, _root, shown) = dropped_router();
    assert_eq!(tree.phase(shown), None);
    assert!(tree.get(shown).is_none());
    assert!(!tree.back(), "nothing is left to handle Back");
}

#[test]
fn a_component_of_a_dropped_router_goes_with_its_parent() {
    let (mut tree, root, shown) = dropped_router();
    let replacement = tree.mount(|_cx| Owner { stack: None });
    assert!(tree.get(root).is_none());
    assert!(tree.get(shown).is_none());
    assert!(tree.get(replacement).is_some());
}

//! Navigation a component asks for while navigation runs waits in the queue.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::router::stack::Stack;
use compono::{Commit, Component, Cx, Handle, Host, Phase, Tree};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Log = Rc<RefCell<Vec<String>>>;

#[derive(Clone, Default)]
struct Commits(Rc<Cell<usize>>);

impl Host for Commits {
    fn commit(&mut self, _commit: &Commit<'_>) {
        self.0.set(self.0.get() + 1);
    }
}

/// Asks for the next route from its constructor.
const FROM_CONSTRUCTOR: u32 = 10;
/// Asks for the next route when it is told it is focused.
const FROM_PHASE: u32 = 20;

struct Screen {
    route: u32,
    log: Log,
}

impl Component for Screen {
    type Msg = ();
    /// The route this screen wants opened above it.
    type Output = u32;
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    fn phase_changed(&mut self, phase: Phase, cx: &mut Cx<'_, Self>) {
        self.log
            .borrow_mut()
            .push(format!("{} {phase:?}", self.route));
        if self.route == FROM_PHASE && phase == Phase::Focused {
            cx.emit(self.route + 1);
        }
    }
}

struct Root {
    stack: Stack<u32, Handle<Screen>, Root>,
    log: Log,
}

impl Component for Root {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, route: u32, cx: &mut Cx<'_, Self>) {
        self.log.borrow_mut().push(format!("push {route} begins"));
        self.stack.push(route, cx);
        self.log.borrow_mut().push(format!("push {route} ends"));
    }
}

fn screen(route: u32, log: &Log, cx: &mut Cx<'_, Root>) -> Handle<Screen> {
    let log = Rc::clone(log);
    cx.child(
        move |cx| {
            log.borrow_mut().push(format!("new {route}"));
            if route == FROM_CONSTRUCTOR {
                cx.emit(route + 1);
            }
            Screen { route, log }
        },
        |root: &mut Root, next, cx| {
            let depth = root.stack.depth();
            root.log
                .borrow_mut()
                .push(format!("asked for {next} at depth {depth}"));
            root.stack.push(next, cx);
        },
    )
}

fn app() -> (Tree, Handle<Root>, Log, Commits) {
    let log = Log::default();
    let commits = Commits::default();
    let mut tree = Tree::new(commits.clone());
    tree.set_phase(Phase::Focused);
    let (for_root, for_screens) = (Rc::clone(&log), Rc::clone(&log));
    let root = tree.mount(move |cx| Root {
        stack: Stack::new(0, cx, move |route, cx| screen(*route, &for_screens, cx)),
        log: for_root,
    });
    log.borrow_mut().clear();
    commits.0.set(0);
    (tree, root, log, commits)
}

#[test]
fn navigation_asked_for_in_a_constructor_starts_after_the_push_ends() {
    let (mut tree, root, log, _commits) = app();
    tree.send(root, FROM_CONSTRUCTOR).unwrap();
    assert_eq!(
        *log.borrow(),
        [
            "push 10 begins",
            "new 10",
            "0 Hidden",
            "10 Focused",
            "push 10 ends",
            "asked for 11 at depth 2",
            "new 11",
            "10 Hidden",
            "11 Focused",
        ]
    );
}

#[test]
fn navigation_asked_for_in_phase_changed_starts_after_the_push_ends() {
    let (mut tree, root, log, _commits) = app();
    tree.send(root, FROM_PHASE).unwrap();
    assert_eq!(
        *log.borrow(),
        [
            "push 20 begins",
            "new 20",
            "0 Hidden",
            "20 Focused",
            "push 20 ends",
            "asked for 21 at depth 2",
            "new 21",
            "20 Hidden",
            "21 Focused",
        ]
    );
}

#[test]
fn navigation_and_the_navigation_it_causes_are_one_commit() {
    let (mut tree, root, _log, commits) = app();
    tree.send(root, FROM_CONSTRUCTOR).unwrap();
    assert_eq!(tree.get(root).unwrap().stack.depth(), 3);
    assert_eq!(commits.0.get(), 1);
}

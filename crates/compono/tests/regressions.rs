//! Each test here failed against an earlier version of the core.

#![cfg(all(feature = "std", feature = "serde"))]
#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::set::ChildSet;
use compono::router::stack::{EmptyRoutes, SavedStack, Stack};
use compono::{
    Commit, Component, Cx, Handle, Host, NodeId, Persist, Phase, Spawner, TaskGuard, TaskId, Tree,
};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Waker};

type Record = (Vec<NodeId>, bool);

#[derive(Clone, Default)]
struct Log(Rc<RefCell<Vec<Record>>>);

struct LogHost(Log);
impl Host for LogHost {
    fn commit(&mut self, commit: &Commit<'_>) {
        (self.0)
            .0
            .borrow_mut()
            .push((commit.dirty.to_vec(), commit.can_back));
    }
}

fn logged_tree() -> (Tree, Log) {
    let log = Log::default();
    (Tree::new(LogHost(log.clone())), log)
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

fn cell(cx: &mut Cx<'_, Screens>) -> Handle<Cell> {
    cx.persistent_child(
        |cx| Cell {
            value: cx.restored().unwrap_or_default(),
        },
        |_parent, (), _cx| {},
    )
}

type Pair = (Handle<Cell>, Handle<Cell>);

struct Screens {
    stack: Stack<u32, Pair, Screens>,
    rejected: bool,
}

enum Nav {
    Push(u32),
    PushNew(u32),
    PushToFront(u32),
    BringToFront(u32),
    PopTo(usize),
    PopWhileAbove(u32),
    ReplaceAll(Vec<u32>),
    Reset(u32),
}

impl Component for Screens {
    type Msg = Nav;
    type Output = ();
    fn update(&mut self, msg: Nav, cx: &mut Cx<'_, Self>) {
        match msg {
            Nav::Push(route) => self.stack.push(route, cx),
            Nav::PushNew(route) => {
                self.stack.push_new(route, cx);
            }
            Nav::PushToFront(route) => self.stack.push_to_front(route, cx),
            Nav::BringToFront(route) => self.stack.bring_to_front(route, cx),
            Nav::PopTo(depth) => {
                self.stack.pop_to(depth, cx);
            }
            Nav::PopWhileAbove(limit) => {
                self.stack.pop_while(|route| *route > limit, cx);
            }
            Nav::ReplaceAll(routes) => {
                self.rejected = self.stack.replace_all(routes, cx) == Err(EmptyRoutes);
            }
            Nav::Reset(route) => self.stack.reset(route, cx),
        }
    }
}

impl Persist for Screens {
    type Saved = SavedStack<u32>;
    fn save(&self) -> SavedStack<u32> {
        self.stack.snapshot()
    }
}

fn mount(tree: &mut Tree) -> Handle<Screens> {
    tree.mount_persistent(|cx| {
        let saved = cx.restored();
        Screens {
            stack: Stack::restore_or(saved, 0, cx, |_route, cx| (cell(cx), cell(cx))),
            rejected: false,
        }
    })
}

fn routes(tree: &Tree, root: Handle<Screens>) -> Vec<u32> {
    let stack = &tree.get(root).unwrap().stack;
    let mut all: Vec<u32> = stack.parked().map(|(route, _)| *route).collect();
    all.push(*stack.route());
    all
}

fn active(tree: &Tree, root: Handle<Screens>) -> Pair {
    *tree.get(root).unwrap().stack.active()
}

#[test]
fn every_child_of_an_entry_gets_back_its_own_state() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);
    tree.send(active(&tree, root).1, vec![10]).unwrap();
    tree.send(root, Nav::Push(1)).unwrap();
    tree.send(active(&tree, root).1, vec![20]).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = mount(&mut tree);
    assert_eq!(tree.get(active(&tree, root).1).unwrap().value, [20]);
    tree.send(root, Nav::PopTo(1)).unwrap();
    assert_eq!(tree.get(active(&tree, root).1).unwrap().value, [10]);
}

fn save_with_large_middle_entry() -> Vec<u8> {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);
    tree.send(root, Nav::Push(1)).unwrap();
    tree.send(active(&tree, root).0, vec![7; 5000]).unwrap();
    tree.send(root, Nav::Push(2)).unwrap();
    tree.save(1).unwrap()
}

#[test]
fn removing_an_unbuilt_entry_removes_its_saved_state() {
    for msg in [Nav::PopTo(1), Nav::Reset(9), Nav::ReplaceAll(vec![9])] {
        let bytes = save_with_large_middle_entry();
        let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let root = mount(&mut tree);
        tree.send(root, msg).unwrap();
        assert!(tree.save(1).unwrap().len() < 200);
    }
}

#[test]
fn a_later_save_keeps_unbuilt_state_that_was_not_removed() {
    let bytes = save_with_large_middle_entry();
    let (tree, _outcome) = {
        let (mut tree, outcome) = Tree::with_saved(NullHost, &bytes, 1);
        let _root = mount(&mut tree);
        (tree, outcome)
    };
    assert!(tree.save(1).unwrap().len() > 5000);
}

#[test]
fn a_new_entry_is_not_given_state_of_a_removed_one() {
    let bytes = save_with_large_middle_entry();
    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = mount(&mut tree);
    tree.send(root, Nav::PopTo(1)).unwrap();
    tree.send(root, Nav::Push(5)).unwrap();
    assert!(tree.get(active(&tree, root).0).unwrap().value.is_empty());
}

#[test]
fn stack_operations_on_equal_routes() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);
    for route in [1, 2, 3] {
        tree.send(root, Nav::Push(route)).unwrap();
    }
    tree.send(root, Nav::PushNew(3)).unwrap();
    assert_eq!(routes(&tree, root), [0, 1, 2, 3], "already on top");

    let one = tree
        .get(root)
        .unwrap()
        .stack
        .parked()
        .nth(1)
        .unwrap()
        .1
        .copied();
    tree.send(root, Nav::BringToFront(1)).unwrap();
    assert_eq!(routes(&tree, root), [0, 2, 3, 1]);
    assert_eq!(Some(active(&tree, root)), one, "the same components moved");

    tree.send(root, Nav::PushToFront(2)).unwrap();
    assert_eq!(routes(&tree, root), [0, 3, 1, 2]);

    tree.send(root, Nav::BringToFront(8)).unwrap();
    assert_eq!(
        routes(&tree, root),
        [0, 3, 1, 2, 8],
        "absent routes are pushed"
    );

    tree.send(root, Nav::PopWhileAbove(1)).unwrap();
    assert_eq!(routes(&tree, root), [0, 3, 1]);
    tree.send(root, Nav::PopWhileAbove(0)).unwrap();
    tree.send(root, Nav::PopWhileAbove(0)).unwrap();
    assert_eq!(routes(&tree, root), [0], "the root always stays");
}

#[test]
fn replace_all_creates_only_the_top_entry() {
    let mut tree = Tree::new(NullHost);
    let root = mount(&mut tree);
    tree.send(root, Nav::ReplaceAll(vec![4, 5, 6])).unwrap();
    assert_eq!(routes(&tree, root), [4, 5, 6]);
    let stack = &tree.get(root).unwrap().stack;
    assert!(stack.parked().all(|(_, live)| live.is_none()));

    tree.send(root, Nav::ReplaceAll(Vec::new())).unwrap();
    assert!(tree.get(root).unwrap().rejected);
    assert_eq!(routes(&tree, root), [4, 5, 6]);

    tree.send(root, Nav::PopTo(2)).unwrap();
    assert_eq!(routes(&tree, root), [4, 5]);
}

struct Quiet {
    child: Option<Handle<Self>>,
    handles_back: bool,
}

enum QuietMsg {
    Grow,
    Back(bool),
    Remove,
}

impl Component for Quiet {
    type Msg = QuietMsg;
    type Output = ();
    fn update(&mut self, msg: QuietMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            QuietMsg::Grow => {
                self.child = Some(cx.child(quiet, |_parent, (), _cx| {}));
            }
            QuietMsg::Back(on) => self.handles_back = on,
            QuietMsg::Remove => {
                if let Some(child) = self.child.take() {
                    cx.destroy(child).unwrap();
                }
            }
        }
    }
    fn back(&self) -> Option<QuietMsg> {
        self.handles_back.then_some(QuietMsg::Back(false))
    }
}

fn quiet(_cx: &mut Cx<'_, Quiet>) -> Quiet {
    Quiet {
        child: None,
        handles_back: false,
    }
}

#[test]
fn a_plain_child_follows_its_parent_and_can_be_lowered() {
    let (mut tree, _log) = logged_tree();
    tree.set_phase(Phase::Focused);
    let root = tree.mount(quiet);
    tree.send(root, QuietMsg::Grow).unwrap();
    let child = tree.get(root).unwrap().child.unwrap();
    assert_eq!(tree.phase(child), Some(Phase::Focused));
    tree.set_phase(Phase::Visible);
    assert_eq!(tree.phase(child), Some(Phase::Visible));
}

#[test]
fn a_phase_change_nobody_handles_commits_nothing() {
    let (mut tree, log) = logged_tree();
    let _root = tree.mount(quiet);
    log.0.borrow_mut().clear();
    tree.set_phase(Phase::Focused);
    assert!(log.0.borrow().is_empty());
}

#[test]
fn a_commit_never_names_a_removed_component() {
    struct Busy;
    impl Component for Busy {
        type Msg = ();
        type Output = ();
        fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
        fn phase_changed(&mut self, _phase: Phase, _cx: &mut Cx<'_, Self>) {}
    }
    struct Owner(Handle<Busy>);
    impl Component for Owner {
        type Msg = ();
        type Output = ();
        fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
            cx.destroy(self.0).unwrap();
        }
    }
    let (mut tree, log) = logged_tree();
    tree.set_phase(Phase::Focused);
    let owner = tree.mount(|cx| Owner(cx.child(|_cx| Busy, |_owner, (), _cx| {})));
    let busy = tree.get(owner).unwrap().0;
    tree.send(owner, ()).unwrap();
    let last = log.0.borrow().last().cloned().unwrap();
    assert_eq!(last.0, [owner.id()]);
    assert!(tree.get(busy).is_none());
}

#[test]
fn can_back_tracks_handlers_at_any_depth() {
    let (mut tree, log) = logged_tree();
    let can_back = || log.0.borrow().last().is_some_and(|commit| commit.1);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(quiet);
    tree.send(root, QuietMsg::Grow).unwrap();
    let child = tree.get(root).unwrap().child.unwrap();
    tree.send(child, QuietMsg::Grow).unwrap();
    let leaf = tree.get(child).unwrap().child.unwrap();
    assert!(!can_back());

    tree.send(leaf, QuietMsg::Back(true)).unwrap();
    assert!(can_back(), "gained below");
    tree.set_phase(Phase::Hidden);
    assert!(!can_back(), "a hidden tree handles nothing");
    tree.set_phase(Phase::Focused);
    assert!(can_back(), "regained on return");

    tree.send(root, QuietMsg::Back(true)).unwrap();
    tree.send(leaf, QuietMsg::Back(false)).unwrap();
    assert!(can_back(), "the root still handles it");
    tree.send(root, QuietMsg::Back(false)).unwrap();
    assert!(!can_back());

    tree.send(leaf, QuietMsg::Back(true)).unwrap();
    assert!(can_back());
    tree.send(child, QuietMsg::Remove).unwrap();
    assert!(!can_back(), "removing the handler clears it");
    assert!(!tree.back());
}

/// Runs a task to its first suspension during `spawn`.
struct Inline;
impl Spawner for Inline {
    fn spawn(&self, mut task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard {
        let mut context = Context::from_waker(Waker::noop());
        let _first_poll = task.as_mut().poll(&mut context);
        TaskGuard::new(Arc::new(()))
    }
}

struct Fetcher {
    received: Vec<u32>,
    pending: Option<TaskId>,
    was_outstanding: bool,
}

enum Fetch {
    Start(u32),
    StartAndCancel(u32),
    Got(u32),
}

impl Component for Fetcher {
    type Msg = Fetch;
    type Output = ();
    fn update(&mut self, msg: Fetch, cx: &mut Cx<'_, Self>) {
        match msg {
            Fetch::Start(value) => {
                self.pending = cx.perform(async move { value }, Fetch::Got).ok();
            }
            Fetch::StartAndCancel(value) => {
                let task = cx.perform(async move { value }, Fetch::Got).unwrap();
                self.was_outstanding = cx.cancel(task);
            }
            Fetch::Got(value) => self.received.push(value),
        }
    }
}

#[test]
fn a_cancelled_task_never_delivers_even_if_it_already_finished() {
    let mut tree = Tree::new(NullHost);
    tree.set_spawner(Inline);
    let fetcher = tree.mount(|_cx| Fetcher {
        received: Vec::new(),
        pending: None,
        was_outstanding: false,
    });
    tree.send(fetcher, Fetch::StartAndCancel(1)).unwrap();
    tree.send(fetcher, Fetch::Start(2)).unwrap();
    tree.pump().unwrap();
    let state = tree.get(fetcher).unwrap();
    assert!(state.was_outstanding);
    assert_eq!(state.received, [2]);
}

struct Rows {
    rows: ChildSet<u32, Handle<Cell2>, Rows>,
}

struct Cell2;
impl Component for Cell2 {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        ORDER.with(|order| order.borrow_mut().push(phase));
    }
}

std::thread_local! {
    static ORDER: RefCell<Vec<Phase>> = const { RefCell::new(Vec::new()) };
    static BUILT: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

impl Component for Rows {
    type Msg = Vec<(u32, Phase)>;
    type Output = ();
    fn update(&mut self, desired: Vec<(u32, Phase)>, cx: &mut Cx<'_, Self>) {
        assert_eq!(self.rows.reconcile(desired, cx), Ok(()));
    }
}

#[test]
fn all_new_children_exist_before_any_phase_is_raised() {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let list = tree.mount(|cx| Rows {
        rows: ChildSet::new(cx, |key, cx| {
            let key = *key;
            cx.child(
                move |_cx| {
                    assert!(ORDER.with(|order| order.borrow().is_empty()), "key {key}");
                    BUILT.with(|built| built.borrow_mut().push(key));
                    Cell2
                },
                |_list, (), _cx| {},
            )
        }),
    });
    tree.send(list, vec![(1, Phase::Focused), (2, Phase::Focused)])
        .unwrap();
    assert_eq!(BUILT.with(|built| built.borrow().clone()), [1, 2]);
    assert_eq!(ORDER.with(|order| order.borrow().len()), 2);
}

struct Row {
    name: u32,
    log: Rc<RefCell<Vec<String>>>,
}

impl Component for Row {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        self.log.borrow_mut().push(format!("back {}", self.name));
    }
    fn back(&self) -> Option<()> {
        Some(())
    }
}

impl Drop for Row {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("drop {}", self.name));
    }
}

struct Table {
    rows: ChildSet<u32, Handle<Row>, Table>,
    log: Rc<RefCell<Vec<String>>>,
}

impl Component for Table {
    type Msg = Vec<(u32, Phase)>;
    type Output = ();
    fn update(&mut self, desired: Vec<(u32, Phase)>, cx: &mut Cx<'_, Self>) {
        assert_eq!(self.rows.reconcile(desired, cx), Ok(()));
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        self.log.borrow_mut().push("drop table".to_owned());
    }
}

fn table(tree: &mut Tree, log: &Rc<RefCell<Vec<String>>>) -> Handle<Table> {
    let (for_rows, for_table) = (Rc::clone(log), Rc::clone(log));
    tree.mount(move |cx| Table {
        log: for_table,
        rows: ChildSet::new(cx, move |name, cx| {
            let (name, log) = (*name, Rc::clone(&for_rows));
            cx.child(move |_cx| Row { name, log }, |_table, (), _cx| {})
        }),
    })
}

#[test]
fn back_keeps_following_the_last_shown_after_the_app_returns() {
    let log = Rc::default();
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = table(&mut tree, &log);
    let shown = |phase| vec![(1, phase), (2, Phase::Visible)];
    tree.send(root, shown(Phase::Visible)).unwrap();
    tree.send(root, shown(Phase::Hidden)).unwrap();
    tree.send(root, shown(Phase::Visible)).unwrap();
    assert!(tree.back());
    assert_eq!(log.borrow().last().unwrap(), "back 1");

    tree.set_phase(Phase::Hidden);
    tree.set_phase(Phase::Focused);
    assert!(tree.back());
    assert_eq!(log.borrow().last().unwrap(), "back 1");
}

#[test]
fn dropping_the_tree_removes_children_before_parents() {
    let log: Rc<RefCell<Vec<String>>> = Rc::default();
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = table(&mut tree, &log);
    tree.send(root, vec![(1, Phase::Visible), (2, Phase::Visible)])
        .unwrap();
    drop(tree);
    assert_eq!(*log.borrow(), ["drop 2", "drop 1", "drop table"]);
}

struct Unmoved;
impl Component for Unmoved {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.unchanged();
    }
    fn back(&self) -> Option<()> {
        Some(())
    }
}

#[test]
fn the_ui_always_learns_that_a_gesture_ended() {
    let (mut tree, log) = logged_tree();
    tree.set_phase(Phase::Focused);
    let _root = tree.mount(|_cx| Unmoved);
    assert!(tree.back_start());
    log.0.borrow_mut().clear();
    assert!(tree.back_commit());
    assert_eq!(log.0.borrow().len(), 1);
    assert_eq!(tree.gesture_target(), None);

    assert!(tree.back_start());
    log.0.borrow_mut().clear();
    tree.set_phase(Phase::Hidden);
    assert_eq!(
        tree.gesture_target(),
        None,
        "a hidden target cannot finish a gesture"
    );
    assert_eq!(log.0.borrow().len(), 1);
}

struct Numbered(u32);
impl Component for Numbered {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 = msg;
    }
}
impl Persist for Numbered {
    type Saved = u32;
    fn save(&self) -> u32 {
        self.0
    }
}

struct Crowd {
    stack: Stack<u32, Vec<Handle<Numbered>>, Crowd>,
}
impl Component for Crowd {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}
impl Persist for Crowd {
    type Saved = SavedStack<u32>;
    fn save(&self) -> SavedStack<u32> {
        self.stack.snapshot()
    }
}

const CROWD: usize = 5000;

fn crowd(tree: &mut Tree) -> Handle<Crowd> {
    tree.mount_persistent(|cx| {
        let saved = cx.restored();
        Crowd {
            stack: Stack::restore_or(saved, 0, cx, |_route, cx| {
                (0..CROWD)
                    .map(|_| {
                        cx.persistent_child(
                            |cx| Numbered(cx.restored().unwrap_or(0)),
                            |_crowd: &mut Crowd, (), _cx| {},
                        )
                    })
                    .collect()
            }),
        }
    })
}

#[test]
fn an_entry_can_hold_any_number_of_children() {
    let mut tree = Tree::new(NullHost);
    let root = crowd(&mut tree);
    let children = tree.get(root).unwrap().stack.active().clone();
    tree.send(*children.first().unwrap(), 11).unwrap();
    tree.send(*children.last().unwrap(), 99).unwrap();
    let bytes = tree.save(1).unwrap();

    let (mut tree, _outcome) = Tree::with_saved(NullHost, &bytes, 1);
    let root = crowd(&mut tree);
    let children = tree.get(root).unwrap().stack.active().clone();
    assert_eq!(tree.get(*children.first().unwrap()).unwrap().0, 11);
    assert_eq!(tree.get(*children.last().unwrap()).unwrap().0, 99);
}

struct Parent {
    child: Option<Handle<Kid>>,
    received: u32,
}

struct Kid {
    parent: Handle<Parent>,
}

impl Component for Kid {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        assert_eq!(cx.send(self.parent, 1), Err(compono::NotOwnChild));
    }
}

impl Component for Parent {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, cx: &mut Cx<'_, Self>) {
        self.received += msg;
        if msg == 0 {
            let parent = self.me();
            self.child = Some(cx.child(move |_cx| Kid { parent }, |_p, (), _cx| {}));
        }
    }
}

impl Parent {
    fn me(&self) -> Handle<Self> {
        ME.with(|me| me.get()).unwrap()
    }
}

std::thread_local! {
    static ME: std::cell::Cell<Option<Handle<Parent>>> = const { std::cell::Cell::new(None) };
}

#[test]
fn a_child_given_its_parents_handle_still_cannot_message_it() {
    let mut tree = Tree::new(NullHost);
    let parent = tree.mount(|_cx| Parent {
        child: None,
        received: 0,
    });
    ME.with(|me| me.set(Some(parent)));
    tree.send(parent, 0).unwrap();
    let kid = tree.get(parent).unwrap().child.unwrap();
    tree.send(kid, ()).unwrap();
    assert_eq!(tree.get(parent).unwrap().received, 0);
}

#[test]
fn a_full_queue_refuses_senders_but_never_task_results() {
    struct Sink {
        sender: compono::Sender<Self>,
        got: u32,
    }
    impl Component for Sink {
        type Msg = u32;
        type Output = ();
        fn update(&mut self, msg: u32, cx: &mut Cx<'_, Self>) {
            self.got += 1;
            if msg == 7 {
                cx.perform(async { 0 }, |value| value).unwrap();
            }
        }
    }
    let mut tree = Tree::new(NullHost);
    tree.set_spawner(Inline);
    tree.set_queue_limit(2);
    let sink = tree.mount(|cx| Sink {
        sender: cx.sender(),
        got: 0,
    });
    let sender = tree.get(sink).unwrap().sender.clone();
    assert_eq!(sender.send(1), Ok(()));
    assert_eq!(sender.send(7), Ok(()));
    assert_eq!(sender.send(3), Err(compono::SendError::Full));

    // Message 7 starts a task while senders are refused, and its result still arrives.
    tree.set_queue_limit(0);
    tree.pump().unwrap();
    assert_eq!(tree.get(sink).unwrap().got, 2);
    assert_eq!(sender.send(4), Err(compono::SendError::Full));
    tree.pump().unwrap();
    assert_eq!(tree.get(sink).unwrap().got, 3);
}

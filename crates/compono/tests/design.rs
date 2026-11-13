//! Executable checks of the core contracts, on an app shaped like TetrisLite.
#![cfg(all(feature = "std", feature = "serde"))]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "assertions in tests"
)]

use compono::router::stack::{AtRoot, SavedStack, Stack};
use compono::{
    Commit, Component, Cx, Handle, Host, NodeId, Persist, Phase, RestoreOutcome, Spawner,
    TaskGuard, Tree,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake, Waker};

type Log = Rc<RefCell<Vec<String>>>;

type CommitRecord = (Vec<NodeId>, bool);

#[derive(Default, Clone)]
struct TestHost {
    commits: Rc<RefCell<Vec<CommitRecord>>>,
    wakes: Arc<WakeCount>,
}
#[derive(Default)]
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
impl Host for TestHost {
    fn commit(&mut self, commit: &Commit<'_>) {
        self.commits
            .borrow_mut()
            .push((commit.dirty.to_vec(), commit.can_back));
    }
    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.wakes))
    }
}

struct Settings {
    log: Log,
}
impl Component for Settings {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        self.log.borrow_mut().push(format!("settings {phase:?}"));
    }
}
impl Drop for Settings {
    fn drop(&mut self) {
        self.log.borrow_mut().push("settings drop".into());
    }
}

struct Home {
    log: Log,
    starts: u32,
}
enum HomeMsg {
    Play,
    Hover,
}
enum HomeOutput {
    Play { level: u32 },
}
#[derive(Serialize, Deserialize)]
struct HomeSaved {
    starts: u32,
}
impl Component for Home {
    type Msg = HomeMsg;
    type Output = HomeOutput;
    fn update(&mut self, msg: HomeMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            HomeMsg::Play => {
                self.starts += 1;
                cx.emit(HomeOutput::Play { level: self.starts });
            }
            HomeMsg::Hover => cx.unchanged(),
        }
    }
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        self.log.borrow_mut().push(format!("home {phase:?}"));
    }
}
impl Persist for Home {
    type Saved = HomeSaved;
    fn save(&self) -> HomeSaved {
        HomeSaved {
            starts: self.starts,
        }
    }
}

struct Game {
    log: Log,
    score: u64,
    sheet: Option<Handle<Settings>>,
}
enum GameMsg {
    Exit,
    OpenSheet,
    CloseSheet,
}
enum GameOutput {
    Exit { score: u64 },
}
#[derive(Serialize, Deserialize)]
struct GameSaved {
    score: u64,
}
impl Component for Game {
    type Msg = GameMsg;
    type Output = GameOutput;
    fn update(&mut self, msg: GameMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            GameMsg::Exit => cx.emit(GameOutput::Exit { score: self.score }),
            GameMsg::OpenSheet => {
                let log = Rc::clone(&self.log);
                let sheet = cx.child(move |_cx| Settings { log }, |_: &mut Self, (), _cx| {});
                cx.set_child_phase(sheet, Phase::Focused).unwrap();
                self.sheet = Some(sheet);
            }
            GameMsg::CloseSheet => {
                if let Some(sheet) = self.sheet.take() {
                    cx.destroy(sheet).unwrap();
                }
            }
        }
    }
    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        self.log.borrow_mut().push(format!("game {phase:?}"));
    }
    fn back(&self) -> Option<GameMsg> {
        self.sheet.is_some().then_some(GameMsg::CloseSheet)
    }
}
impl Persist for Game {
    type Saved = GameSaved;
    fn save(&self) -> GameSaved {
        GameSaved { score: self.score }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
enum Route {
    Home,
    Game { seed: u64 },
}

#[derive(Clone, Copy)]
enum Child {
    Home(Handle<Home>),
    Game(Handle<Game>),
}

struct Root {
    stack: Stack<Route, Child, Root>,
    best: u64,
}
enum RootMsg {
    Pop,
}
#[derive(Serialize, Deserialize)]
struct RootSaved {
    stack: SavedStack<Route>,
}

impl Root {
    fn build(log: Log) -> impl Fn(&Route, &mut Cx<'_, Root>) -> Child + 'static {
        move |route, cx| match route {
            Route::Home => {
                let log = Rc::clone(&log);
                Child::Home(cx.persistent_child(
                    move |cx| {
                        let starts = cx.restored().map_or(0, |saved: HomeSaved| saved.starts);
                        log.borrow_mut().push(format!("home new {starts}"));
                        Home { log, starts }
                    },
                    |root: &mut Root, HomeOutput::Play { level }, cx| {
                        root.stack.push(
                            Route::Game {
                                seed: u64::from(level),
                            },
                            cx,
                        );
                    },
                ))
            }
            Route::Game { seed } => {
                let log = Rc::clone(&log);
                let seed = *seed;
                Child::Game(cx.persistent_child(
                    move |cx| {
                        let score = cx.restored().map_or(seed, |saved: GameSaved| saved.score);
                        log.borrow_mut().push(format!("game new {score}"));
                        Game {
                            log,
                            score,
                            sheet: None,
                        }
                    },
                    |root: &mut Root, GameOutput::Exit { score }, cx| {
                        root.best = root.best.max(score);
                        assert_eq!(root.stack.pop(cx).map(|_| ()), Ok(()));
                    },
                ))
            }
        }
    }

    fn new(log: Log, cx: &mut Cx<'_, Self>) -> Self {
        let saved = cx.restored().map(|saved: RootSaved| saved.stack);
        Self {
            stack: Stack::restore_or(saved, Route::Home, cx, Self::build(log)),
            best: 0,
        }
    }
}
impl Component for Root {
    type Msg = RootMsg;
    type Output = ();
    fn update(&mut self, RootMsg::Pop: RootMsg, cx: &mut Cx<'_, Self>) {
        let _popped: Result<Route, AtRoot> = self.stack.pop(cx);
    }
    fn back(&self) -> Option<RootMsg> {
        self.stack.can_pop().then_some(RootMsg::Pop)
    }
}
impl Persist for Root {
    type Saved = RootSaved;
    fn save(&self) -> RootSaved {
        RootSaved {
            stack: self.stack.snapshot(),
        }
    }
}

fn app() -> (Tree, Handle<Root>, Log, TestHost) {
    let log = Log::default();
    let host = TestHost::default();
    let mut tree = Tree::new(host.clone());
    let root_log = Rc::clone(&log);
    let root = tree.mount_persistent(move |cx| Root::new(root_log, cx));
    tree.set_phase(Phase::Focused);
    (tree, root, log, host)
}

fn home(tree: &Tree, root: Handle<Root>) -> Handle<Home> {
    match tree.get(root).unwrap().stack.active() {
        Child::Home(home) => *home,
        Child::Game(_) => panic!("home is not active"),
    }
}

fn game(tree: &Tree, root: Handle<Root>) -> Handle<Game> {
    match tree.get(root).unwrap().stack.active() {
        Child::Game(game) => *game,
        Child::Home(_) => panic!("game is not active"),
    }
}

#[test]
fn child_reports_up_without_knowing_its_parent() {
    let (mut tree, root, _log, _host) = app();
    let first = home(&tree, root);
    tree.send(first, HomeMsg::Play).unwrap();
    let played = game(&tree, root);
    assert_eq!(tree.get(root).unwrap().stack.depth(), 2);
    tree.send(played, GameMsg::Exit).unwrap();
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
    assert_eq!(tree.get(root).unwrap().best, 1);
    assert!(tree.get(played).is_none());
    assert!(tree.send(played, GameMsg::Exit).is_err());
}

#[test]
fn phases_step_down_before_the_next_entry_steps_up() {
    let (mut tree, root, log, _host) = app();
    log.borrow_mut().clear();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    assert_eq!(*log.borrow(), ["game new 1", "home Hidden", "game Focused"]);
}

#[test]
fn a_child_never_exceeds_its_parent() {
    let (mut tree, root, log, _host) = app();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    tree.send(game(&tree, root), GameMsg::OpenSheet).unwrap();
    log.borrow_mut().clear();
    tree.set_phase(Phase::Visible);
    assert_eq!(*log.borrow(), ["settings Visible", "game Visible"]);
    log.borrow_mut().clear();
    tree.set_phase(Phase::Focused);
    assert_eq!(*log.borrow(), ["game Focused", "settings Focused"]);
}

#[test]
fn back_prefers_the_deepest_handler_and_reports_availability() {
    let (mut tree, root, log, host) = app();
    assert!(!tree.back());
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    assert!(host.commits.borrow().last().unwrap().1);
    tree.send(game(&tree, root), GameMsg::OpenSheet).unwrap();
    log.borrow_mut().clear();
    assert!(tree.back());
    assert_eq!(*log.borrow(), ["settings Hidden", "settings drop"]);
    assert_eq!(tree.get(root).unwrap().stack.depth(), 2);
    assert!(tree.back());
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
    assert!(!host.commits.borrow().last().unwrap().1);
}

#[test]
fn predictive_back_changes_nothing_until_committed() {
    let (mut tree, root, _log, _host) = app();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    assert!(tree.back_start());
    assert_eq!(tree.get(root).unwrap().stack.depth(), 2);
    tree.back_cancel();
    assert!(tree.gesture_target().is_none());
    assert_eq!(tree.get(root).unwrap().stack.depth(), 2);
    assert!(tree.back_start());
    assert!(tree.back_commit());
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
}

#[test]
fn one_operation_is_one_commit() {
    let (mut tree, root, _log, host) = app();
    host.commits.borrow_mut().clear();
    tree.send(home(&tree, root), HomeMsg::Hover).unwrap();
    assert!(
        host.commits.borrow().is_empty(),
        "unchanged run must not commit"
    );
    let first = home(&tree, root);
    tree.send(first, HomeMsg::Play).unwrap();
    let commits = host.commits.borrow();
    assert_eq!(commits.len(), 1, "push causes one commit");
    assert!(commits[0].0.contains(&first.id()));
    assert!(commits[0].0.contains(&root.id()));
}

fn saved_two_deep() -> Vec<u8> {
    let (mut tree, root, _log, _host) = app();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    tree.save(7).unwrap()
}

fn restore(bytes: &[u8], schema: u32) -> (Tree, Handle<Root>, Log, RestoreOutcome) {
    let log = Log::default();
    let (mut tree, outcome) = Tree::with_saved(TestHost::default(), bytes, schema);
    let root_log = Rc::clone(&log);
    let root = tree.mount_persistent(move |cx| Root::new(root_log, cx));
    tree.set_phase(Phase::Focused);
    (tree, root, log, outcome)
}

#[test]
fn restore_builds_only_what_is_on_screen() {
    let (mut tree, root, log, outcome) = restore(&saved_two_deep(), 7);
    assert_eq!(outcome, RestoreOutcome::Restored);
    assert_eq!(tree.get(root).unwrap().stack.depth(), 2);
    assert_eq!(*log.borrow(), ["game new 1", "game Focused"]);
    log.borrow_mut().clear();
    assert!(tree.back_start() || tree.get(root).unwrap().stack.can_pop());
    tree.back();
    assert_eq!(*log.borrow(), ["home new 1", "game Hidden", "home Focused"]);
}

#[test]
fn saving_again_keeps_state_of_entries_never_rebuilt() {
    let (tree, _root, _log, _outcome) = restore(&saved_two_deep(), 7);
    let again = tree.save(7).unwrap();
    let (mut tree, root, log, outcome) = restore(&again, 7);
    assert_eq!(outcome, RestoreOutcome::Restored);
    tree.back();
    assert!(log.borrow().contains(&"home new 1".to_string()));
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
}

#[test]
fn unusable_saved_state_means_a_clean_start() {
    let (tree, root, _log, outcome) = restore(&saved_two_deep(), 8);
    assert_eq!(outcome, RestoreOutcome::SchemaChanged);
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
    let (tree, root, _log, outcome) = restore(&[0xff; 3], 7);
    assert_eq!(outcome, RestoreOutcome::Corrupt);
    assert_eq!(tree.get(root).unwrap().stack.depth(), 1);
}

struct Bg {
    sender_slot: Arc<Mutex<Option<compono::Sender<Bg>>>>,
    value: u32,
}
impl Component for Bg {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.value = msg;
    }
}

#[test]
fn another_thread_reaches_a_component_only_through_the_queue() {
    let host = TestHost::default();
    let wakes = Arc::clone(&host.wakes);
    let mut tree = Tree::new(host);
    let slot = Arc::new(Mutex::new(None));
    let init_slot = Arc::clone(&slot);
    let bg = tree.mount(move |cx: &mut Cx<'_, Bg>| {
        *init_slot.lock().unwrap() = Some(cx.sender());
        Bg {
            sender_slot: init_slot,
            value: 0,
        }
    });
    let sender = slot.lock().unwrap().clone().unwrap();
    std::thread::spawn(move || sender.send(42).unwrap())
        .join()
        .unwrap();
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        tree.get(bg).unwrap().value,
        0,
        "nothing runs until the host pumps"
    );
    tree.pump().unwrap();
    assert_eq!(tree.get(bg).unwrap().value, 42);
    assert!(tree.get(bg).unwrap().sender_slot.lock().is_ok());
}

type BoxedTask = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone, Default)]
struct ManualSpawner {
    tasks: Rc<RefCell<Vec<BoxedTask>>>,
    cancelled: Arc<AtomicBool>,
}
struct CancelFlag(Arc<AtomicBool>);
impl Drop for CancelFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
impl Spawner for ManualSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard {
        self.tasks.borrow_mut().push(task);
        TaskGuard::new(CancelFlag(Arc::clone(&self.cancelled)))
    }
}

fn run_all(spawner: &ManualSpawner) {
    let mut cx = Context::from_waker(Waker::noop());
    for task in spawner.tasks.borrow_mut().iter_mut() {
        let _ = task.as_mut().poll(&mut cx);
    }
}

struct Loader {
    value: Option<u64>,
}
enum LoaderMsg {
    Start,
    Done(u64),
}
impl Component for Loader {
    type Msg = LoaderMsg;
    type Output = ();
    fn update(&mut self, msg: LoaderMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            LoaderMsg::Start => {
                cx.perform(async { 9 }, LoaderMsg::Done).unwrap();
            }
            LoaderMsg::Done(value) => self.value = Some(value),
        }
    }
}

struct LoaderParent {
    child: Option<Handle<Loader>>,
}
impl Component for LoaderParent {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        if let Some(child) = self.child.take() {
            cx.destroy(child).unwrap();
        }
    }
}

#[test]
fn a_task_delivers_its_result_as_a_message() {
    let spawner = ManualSpawner::default();
    let mut tree = Tree::new(TestHost::default());
    tree.set_spawner(spawner.clone());
    let loader = tree.mount(|_: &mut Cx<'_, Loader>| Loader { value: None });
    tree.send(loader, LoaderMsg::Start).unwrap();
    run_all(&spawner);
    tree.pump().unwrap();
    assert_eq!(tree.get(loader).unwrap().value, Some(9));
}

#[test]
fn destroying_a_component_cancels_its_task_and_drops_late_results() {
    let spawner = ManualSpawner::default();
    let mut tree = Tree::new(TestHost::default());
    tree.set_spawner(spawner.clone());
    let slot = Rc::new(RefCell::new(None));
    let init = Rc::clone(&slot);
    let parent = tree.mount(move |cx: &mut Cx<'_, LoaderParent>| {
        let child = cx.child(
            |_| Loader { value: None },
            |_: &mut LoaderParent, (), _cx| {},
        );
        *init.borrow_mut() = Some(child);
        LoaderParent { child: Some(child) }
    });
    let child = slot.borrow().unwrap();
    tree.send(child, LoaderMsg::Start).unwrap();
    assert!(!spawner.cancelled.load(Ordering::SeqCst));
    tree.send(parent, ()).unwrap();
    assert!(
        spawner.cancelled.load(Ordering::SeqCst),
        "destroy must stop the task"
    );
    run_all(&spawner);
    tree.pump().unwrap();
    assert!(tree.get(child).is_none());
}

#[test]
fn a_reused_slot_does_not_revive_old_handles() {
    let (mut tree, root, _log, _host) = app();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    let old = game(&tree, root);
    tree.send(old, GameMsg::Exit).unwrap();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    let new = game(&tree, root);
    assert_eq!(
        old.id().to_bits() & 0xffff_ffff,
        new.id().to_bits() & 0xffff_ffff
    );
    assert!(tree.get(old).is_none());
    assert!(tree.get(new).is_some());
    assert_eq!(NodeId::from_bits(new.id().to_bits()), new.id());
}

#[test]
fn a_root_mounted_after_the_app_is_focused_becomes_focused() {
    let mut tree = Tree::new(TestHost::default());
    tree.set_phase(Phase::Focused);
    let bg = tree.mount(|cx: &mut Cx<'_, Bg>| Bg {
        sender_slot: Arc::new(Mutex::new(Some(cx.sender()))),
        value: 0,
    });
    assert_eq!(tree.phase(bg), Some(Phase::Focused));
}

#[test]
fn a_gesture_ends_when_its_target_is_destroyed() {
    let (mut tree, root, _log, _host) = app();
    tree.send(home(&tree, root), HomeMsg::Play).unwrap();
    tree.send(game(&tree, root), GameMsg::OpenSheet).unwrap();
    assert!(tree.back_start());
    let target = tree.gesture_target().unwrap();
    let Some(sheet_owner) = (match tree.get(root).unwrap().stack.active() {
        Child::Game(game) => Some(*game),
        Child::Home(_) => None,
    }) else {
        panic!("game is not active");
    };
    assert_eq!(target, sheet_owner.id());
    tree.send(sheet_owner, GameMsg::Exit).unwrap();
    assert!(tree.gesture_target().is_none());
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Handle<Home>>();
    assert_send_sync::<NodeId>();
    assert_send_sync::<compono::Sender<Bg>>();
};

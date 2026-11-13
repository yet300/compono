//! Browser history against a stack, with a browser simulated in memory.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

use compono::router::link::{Navigation, Paths};
use compono::router::stack::Stack;
use compono::{Commit, Component, Cx, Handle, Host, Tree};
use compono_web::history::{Browser, EntryState, History};

struct NullHost;
impl Host for NullHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

/// Routes are numbers. `/n` opens `[0, n]`, and `/0` opens the root alone.
struct Numbers;

impl Paths for Numbers {
    type Route = u32;

    fn path(&self, route: &u32) -> String {
        format!("/{route}")
    }

    fn routes(&self, path: &str) -> Vec<u32> {
        match path.strip_prefix('/').and_then(|n| n.parse().ok()) {
            Some(0) => vec![0],
            Some(n) => vec![0, n],
            None => Vec::new(),
        }
    }
}

enum Msg {
    Push(u32),
    Pop,
    Replace(u32),
    Go(Navigation<u32>),
}

struct Screens {
    stack: Stack<u32, (), Self>,
}

impl Component for Screens {
    type Msg = Msg;
    type Output = ();

    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Push(route) => self.stack.push(route, cx),
            Msg::Pop => {
                self.stack.pop(cx).unwrap();
            }
            Msg::Replace(route) => {
                let _old = self.stack.replace(route, cx);
            }
            Msg::Go(navigation) => navigation.apply(&mut self.stack, cx),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Call {
    Push(String, u32),
    Replace(String, u32),
    Go(i32),
}

/// History as a browser keeps it: entries, a position and queued traversals.
#[derive(Default)]
struct FakeBrowser {
    entries: Vec<(String, Option<EntryState>)>,
    at: usize,
    queued: Vec<i32>,
    calls: Vec<Call>,
    epochs: u32,
}

impl FakeBrowser {
    fn opened_at(path: &str) -> Self {
        Self {
            entries: vec![(path.to_owned(), None)],
            ..Self::default()
        }
    }

    fn calls(&mut self) -> Vec<Call> {
        std::mem::take(&mut self.calls)
    }

    fn paths(&self) -> Vec<&str> {
        self.entries.iter().map(|(path, _)| path.as_str()).collect()
    }

    /// The user presses Back or Forward: the browser moves and reports it.
    fn user_go(&mut self, delta: i32) -> (Option<EntryState>, String) {
        self.at = self.at.checked_add_signed(delta as isize).unwrap();
        self.current()
    }

    /// Finishes a traversal the application asked for.
    fn arrive(&mut self) -> (Option<EntryState>, String) {
        let delta = self.queued.remove(0);
        self.user_go(delta)
    }

    fn current(&self) -> (Option<EntryState>, String) {
        let (path, state) = &self.entries[self.at];
        (*state, path.clone())
    }
}

impl Browser for FakeBrowser {
    fn push(&mut self, path: &str, state: EntryState) {
        self.calls.push(Call::Push(path.to_owned(), state.index));
        self.entries.truncate(self.at + 1);
        self.entries.push((path.to_owned(), Some(state)));
        self.at += 1;
    }

    fn replace(&mut self, path: &str, state: EntryState) {
        self.calls.push(Call::Replace(path.to_owned(), state.index));
        self.entries[self.at] = (path.to_owned(), Some(state));
    }

    fn go(&mut self, delta: i32) {
        self.calls.push(Call::Go(delta));
        self.queued.push(delta);
    }

    fn new_epoch(&mut self) -> u32 {
        self.epochs += 1;
        self.epochs
    }
}

struct App {
    tree: Tree,
    root: Handle<Screens>,
    history: History<Numbers>,
    browser: FakeBrowser,
}

impl App {
    fn open(path: &str) -> Self {
        Self::resume(FakeBrowser::opened_at(path), &[0])
    }

    /// Builds a stack of `routes`, as restoring saved state would, then starts.
    fn resume(mut browser: FakeBrowser, routes: &[u32]) -> Self {
        let mut tree = Tree::new(NullHost);
        let routes = routes.to_vec();
        let root = tree.mount(move |cx| {
            let mut stack = Stack::new(0, cx, |_route: &u32, _cx| ());
            stack.replace_all(routes, cx).unwrap();
            Screens { stack }
        });
        let mut history = History::new(Numbers, 7);
        let (state, path) = browser.current();
        let stack = &tree.get(root).unwrap().stack;
        let navigation = history.start(&mut browser, stack, state, &path);
        let mut app = Self {
            tree,
            root,
            history,
            browser,
        };
        if let Some(navigation) = navigation {
            app.send(Msg::Go(navigation));
        }
        app.sync();
        app
    }

    fn stack(&self) -> &Stack<u32, (), Screens> {
        &self.tree.get(self.root).unwrap().stack
    }

    fn routes(&self) -> Vec<u32> {
        self.stack().entries().map(|(_key, route)| *route).collect()
    }

    fn sync(&mut self) {
        let stack = &self.tree.get(self.root).unwrap().stack;
        self.history.sync(&mut self.browser, stack);
    }

    fn send(&mut self, msg: Msg) {
        self.tree.send(self.root, msg).unwrap();
        self.sync();
    }

    fn popped(&mut self, (state, path): (Option<EntryState>, String)) {
        let stack = &self.tree.get(self.root).unwrap().stack;
        if let Some(navigation) = self.history.popped(&mut self.browser, stack, state, &path) {
            self.send(Msg::Go(navigation));
        }
    }

    fn user_go(&mut self, delta: i32) {
        let arrived = self.browser.user_go(delta);
        self.popped(arrived);
    }

    fn arrive(&mut self) {
        let arrived = self.browser.arrive();
        self.popped(arrived);
    }
}

#[test]
fn opening_writes_the_root_into_the_first_entry() {
    let mut app = App::open("/elsewhere");
    assert_eq!(app.browser.calls(), [Call::Replace("/0".into(), 0)]);
    app.sync();
    assert_eq!(app.browser.calls(), [], "an unchanged stack writes nothing");
}

#[test]
fn pushes_become_entries_and_back_pops_them() {
    let mut app = App::open("/0");
    app.send(Msg::Push(1));
    app.send(Msg::Push(2));
    assert_eq!(app.browser.paths(), ["/0", "/1", "/2"]);

    app.browser.calls();
    app.user_go(-1);
    assert_eq!(app.routes(), [0, 1]);
    assert_eq!(app.browser.calls(), [], "the browser already moved");
}

#[test]
fn forward_reopens_the_entry_from_its_path() {
    let mut app = App::open("/0");
    app.send(Msg::Push(1));
    app.send(Msg::Push(2));
    app.user_go(-2);
    assert_eq!(app.routes(), [0]);
    app.browser.calls();

    app.user_go(2);
    assert_eq!(app.routes(), [0, 1, 2]);
    assert_eq!(app.browser.calls(), []);
    app.user_go(-1);
    assert_eq!(app.routes(), [0, 1], "reopened entries are matched again");
}

#[test]
fn popping_in_the_app_moves_the_browser_back() {
    let mut app = App::open("/0");
    app.send(Msg::Push(1));
    app.send(Msg::Push(2));
    app.browser.calls();

    app.send(Msg::Pop);
    app.send(Msg::Pop);
    assert_eq!(
        app.browser.calls(),
        [Call::Go(-1)],
        "one traversal at a time"
    );
    app.arrive();
    assert_eq!(app.browser.calls(), [Call::Go(-1)], "then the rest");
    app.arrive();
    assert_eq!(app.browser.calls(), []);
    assert_eq!(app.browser.at, 0);

    app.send(Msg::Push(3));
    assert_eq!(
        app.browser.paths(),
        ["/0", "/3"],
        "a push drops forward entries"
    );
}

#[test]
fn replacing_the_top_replaces_the_entry() {
    let mut app = App::open("/0");
    app.send(Msg::Push(1));
    app.browser.calls();
    app.send(Msg::Replace(4));
    assert_eq!(app.browser.calls(), [Call::Replace("/4".into(), 1)]);
    assert_eq!(app.browser.paths(), ["/0", "/4"]);
}

#[test]
fn a_link_opens_its_screens_with_back_entries() {
    let mut app = App::open("/5");
    assert_eq!(app.routes(), [0, 5]);
    assert_eq!(
        app.browser.calls(),
        [Call::Replace("/0".into(), 0), Call::Push("/5".into(), 1)]
    );
    app.user_go(-1);
    assert_eq!(app.routes(), [0]);
}

#[test]
fn a_reload_adopts_the_restored_stack() {
    let mut app = App::open("/0");
    app.send(Msg::Push(1));
    app.send(Msg::Push(2));
    app.user_go(-1);
    let mut browser = std::mem::take(&mut app.browser);
    browser.calls();

    let mut again = App::resume(browser, &[0, 1]);
    assert_eq!(again.browser.calls(), [], "nothing is rewritten");
    again.user_go(1);
    assert_eq!(again.routes(), [0, 1, 2], "forward works across the reload");
    again.user_go(-2);
    assert_eq!(again.routes(), [0]);
}

#[test]
fn a_restore_that_disagrees_with_the_entry_follows_the_address() {
    let mut app = App::open("/0");
    app.send(Msg::Push(3));
    let browser = std::mem::take(&mut app.browser);

    let again = App::resume(browser, &[0]);
    assert_eq!(again.routes(), [0, 3], "the address names screen 3");
}

#[test]
fn an_entry_from_another_run_is_opened_from_its_path() {
    let mut app = App::open("/0");
    let root_key = app.stack().entries().next().unwrap().0;
    app.send(Msg::Push(1));
    app.browser.entries.insert(0, ("/6".into(), None));
    app.browser.at += 1;
    app.user_go(-2);
    assert_eq!(app.routes(), [0, 6]);
    assert_eq!(
        app.stack().entries().next().unwrap().0,
        root_key,
        "the root the stack already shows keeps its visit and state"
    );
    assert_eq!(
        app.browser.calls().last(),
        Some(&Call::Push("/6".into(), 1))
    );
}

#[test]
fn changes_during_a_traversal_wait_for_it() {
    let mut app = App::open("/0");
    app.send(Msg::Push(1));
    app.send(Msg::Push(2));
    app.browser.calls();

    app.send(Msg::Pop);
    app.send(Msg::Push(9));
    assert_eq!(app.browser.calls(), [Call::Go(-1)]);
    app.arrive();
    assert_eq!(app.browser.calls(), [Call::Push("/9".into(), 2)]);

    app.tree.send(app.root, Msg::Pop).unwrap();
    app.tree.send(app.root, Msg::Pop).unwrap();
    app.sync();
    assert_eq!(app.browser.calls(), [Call::Go(-2)], "two pops in one frame");
    app.send(Msg::Push(8));
    assert_eq!(app.browser.calls(), [], "the browser is still moving");
    app.arrive();
    assert_eq!(app.browser.calls(), [Call::Push("/8".into(), 1)]);
    assert_eq!(app.browser.paths(), ["/0", "/8"]);
}

#[derive(Clone, Debug)]
enum Step {
    Push(u32),
    Pop,
    Replace(u32),
    /// Pop twice in one frame, so one traversal covers both.
    PopTwice,
    Back(i32),
    Forward(i32),
    /// The browser arrives where the application sent it.
    Arrive,
}

fn step() -> impl proptest::strategy::Strategy<Value = Step> {
    use proptest::prelude::*;
    prop_oneof![
        3 => (0..5_u32).prop_map(Step::Push),
        2 => Just(Step::Pop),
        1 => (0..5_u32).prop_map(Step::Replace),
        1 => Just(Step::PopTwice),
        2 => (1..3_i32).prop_map(Step::Back),
        2 => (1..3_i32).prop_map(Step::Forward),
        2 => Just(Step::Arrive),
    ]
}

impl App {
    fn settle(&mut self) {
        while !self.browser.queued.is_empty() {
            self.arrive();
        }
    }

    fn take(&mut self, step: Step) {
        let depth = self.stack().depth();
        match step {
            Step::Push(route) => self.send(Msg::Push(route)),
            Step::Pop if depth > 1 => self.send(Msg::Pop),
            Step::Replace(route) => self.send(Msg::Replace(route)),
            Step::PopTwice if depth > 2 => {
                self.tree.send(self.root, Msg::Pop).unwrap();
                self.tree.send(self.root, Msg::Pop).unwrap();
                self.sync();
            }
            Step::Back(by) => {
                self.settle();
                if usize::try_from(by).is_ok_and(|by| by <= self.browser.at) {
                    self.user_go(-by);
                }
            }
            Step::Forward(by) => {
                self.settle();
                let ahead = self.browser.entries.len() - 1 - self.browser.at;
                if usize::try_from(by).is_ok_and(|by| by <= ahead) {
                    self.user_go(by);
                }
            }
            Step::Arrive => {
                if !self.browser.queued.is_empty() {
                    self.arrive();
                }
            }
            Step::Pop | Step::PopTwice => {}
        }
    }
}

proptest::proptest! {
    #[test]
    fn the_browser_always_ends_where_the_stack_is(
        steps in proptest::collection::vec(step(), 0..40),
    ) {
        let mut app = App::open("/0");
        for step in steps {
            app.take(step);
            if app.browser.queued.is_empty() {
                let shown: Vec<String> =
                    app.routes().iter().map(|route| format!("/{route}")).collect();
                let entries: Vec<&str> = app.browser.paths();
                proptest::prop_assert_eq!(app.browser.at + 1, shown.len());
                proptest::prop_assert_eq!(&entries[..shown.len()], &shown[..]);
            }
        }
        app.settle();
        proptest::prop_assert_eq!(app.browser.at + 1, app.stack().depth());
    }
}

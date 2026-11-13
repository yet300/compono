//! Behaviour every [`Host`] must preserve, runnable against any implementation.

use crate::recording::{CommitLog, record};
use compono::{Component, Cx, Handle, Host, Persist, Phase, RestoreOutcome, Sender, Tree};
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

/// A contract the host under test broke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    /// The name of the failed check.
    pub check: &'static str,
    /// What was observed.
    pub detail: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.check, self.detail)
    }
}
impl std::error::Error for Violation {}

fn ensure(check: &'static str, holds: bool, detail: impl Fn() -> String) -> Result<(), Violation> {
    if holds {
        Ok(())
    } else {
        Err(Violation {
            check,
            detail: detail(),
        })
    }
}

type Trace = Arc<Mutex<Vec<&'static str>>>;

fn note(trace: &Trace, entry: &'static str) {
    trace
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(entry);
}

fn take(trace: &Trace) -> Vec<&'static str> {
    std::mem::take(&mut *trace.lock().unwrap_or_else(PoisonError::into_inner))
}

enum ParentMsg {
    Bounce,
    Quiet,
    Set(u32),
}

struct Parent {
    value: u32,
    child: Handle<Child>,
    sender: Sender<Parent>,
    trace: Trace,
}

struct Child {
    trace: Trace,
}

impl Component for Child {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        note(&self.trace, "child ran");
        cx.emit(());
    }

    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        note(
            &self.trace,
            if phase == Phase::Focused {
                "child up"
            } else {
                "child down"
            },
        );
    }

    fn back(&self) -> Option<()> {
        Some(())
    }
}

impl Component for Parent {
    type Msg = ParentMsg;
    type Output = ();

    fn update(&mut self, msg: ParentMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            ParentMsg::Bounce => {
                // The child lives as long as this parent, so the message is queued.
                let _queued = cx.send(self.child, ());
            }
            ParentMsg::Quiet => cx.unchanged(),
            ParentMsg::Set(value) => self.value = value,
        }
    }

    fn phase_changed(&mut self, phase: Phase, _cx: &mut Cx<'_, Self>) {
        note(
            &self.trace,
            if phase == Phase::Focused {
                "parent up"
            } else {
                "parent down"
            },
        );
    }
}

impl Persist for Parent {
    type Saved = u32;

    fn save(&self) -> u32 {
        self.value
    }
}

fn mount(tree: &mut Tree, trace: &Trace) -> Handle<Parent> {
    let trace = Arc::clone(trace);
    tree.mount_persistent(move |cx| {
        let child_trace = Arc::clone(&trace);
        let value = cx.restored().unwrap_or(0);
        let child = cx.child(
            move |_cx| Child { trace: child_trace },
            |_parent, (), _cx| {},
        );
        // Created just above, so it is this component's own live child.
        let _own_child = cx.set_child_phase(child, Phase::Focused);
        Parent {
            value,
            child,
            sender: cx.sender(),
            trace,
        }
    })
}

struct Fixture {
    tree: Tree,
    root: Handle<Parent>,
    log: CommitLog,
    trace: Trace,
}

fn fixture(host: impl Host + 'static) -> Fixture {
    let (host, log) = record(host);
    let trace = Trace::default();
    let mut tree = Tree::new(host);
    let root = mount(&mut tree, &trace);
    Fixture {
        tree,
        root,
        log,
        trace,
    }
}

/// Runs every check against hosts produced by `make`.
///
/// `wakes` reports how many times the host has asked its UI to wake; the count
/// must grow when another thread queues a message.
///
/// # Errors
///
/// Returns the first [`Violation`] found.
pub fn check_host<H: Host + 'static>(
    mut make: impl FnMut() -> H,
    wakes: impl Fn() -> usize,
) -> Result<(), Violation> {
    one_commit_per_operation(make())?;
    no_commit_without_change(make())?;
    phases_keep_their_order(make())?;
    back_reports_availability(make())?;
    state_round_trips(&mut make)?;
    other_threads_wake_the_host(make(), wakes)
}

fn one_commit_per_operation<H: Host + 'static>(host: H) -> Result<(), Violation> {
    const CHECK: &str = "one_commit_per_operation";
    let mut app = fixture(host);
    app.log.clear();
    let sent = app.tree.send(app.root, ParentMsg::Bounce);
    ensure(CHECK, sent.is_ok(), || format!("{sent:?}"))?;
    ensure(CHECK, app.log.len() == 1, || {
        format!("{} commits", app.log.len())
    })?;
    let dirty = app.log.last().map_or(0, |commit| commit.dirty.len());
    ensure(CHECK, dirty == 2, || format!("{dirty} dirty components"))
}

fn no_commit_without_change<H: Host + 'static>(host: H) -> Result<(), Violation> {
    const CHECK: &str = "no_commit_without_change";
    let mut app = fixture(host);
    app.log.clear();
    let sent = app.tree.send(app.root, ParentMsg::Quiet);
    ensure(CHECK, sent.is_ok(), || format!("{sent:?}"))?;
    ensure(CHECK, app.log.is_empty(), || {
        format!("{} commits", app.log.len())
    })
}

fn phases_keep_their_order<H: Host + 'static>(host: H) -> Result<(), Violation> {
    const CHECK: &str = "phases_keep_their_order";
    let mut app = fixture(host);
    app.tree.set_phase(Phase::Focused);
    let up = take(&app.trace);
    ensure(CHECK, up == ["parent up", "child up"], || format!("{up:?}"))?;
    app.tree.set_phase(Phase::Hidden);
    let down = take(&app.trace);
    ensure(CHECK, down == ["child down", "parent down"], || {
        format!("{down:?}")
    })
}

fn back_reports_availability<H: Host + 'static>(host: H) -> Result<(), Violation> {
    const CHECK: &str = "back_reports_availability";
    let mut app = fixture(host);
    let hidden = app.tree.back();
    ensure(CHECK, !hidden, || "hidden app handled Back".to_owned())?;
    app.tree.set_phase(Phase::Focused);
    let can_back = app.log.last().is_some_and(|commit| commit.can_back);
    ensure(CHECK, can_back, || "commit did not report Back".to_owned())?;
    let handled = app.tree.back();
    ensure(CHECK, handled, || {
        "visible child did not handle Back".to_owned()
    })?;
    let trace = take(&app.trace);
    ensure(CHECK, trace.contains(&"child ran"), || format!("{trace:?}"))
}

fn state_round_trips<H: Host + 'static>(make: &mut impl FnMut() -> H) -> Result<(), Violation> {
    const CHECK: &str = "state_round_trips";
    let mut app = fixture(make());
    let sent = app.tree.send(app.root, ParentMsg::Set(41));
    ensure(CHECK, sent.is_ok(), || format!("{sent:?}"))?;
    let bytes = app.tree.save(7).map_err(|error| Violation {
        check: CHECK,
        detail: error.to_string(),
    })?;
    let (mut tree, outcome) = Tree::with_saved(make(), &bytes, 7);
    ensure(CHECK, outcome == RestoreOutcome::Restored, || {
        format!("{outcome:?}")
    })?;
    let root = mount(&mut tree, &app.trace);
    let value = tree.get(root).map(|parent| parent.value);
    ensure(CHECK, value == Some(41), || format!("{value:?}"))?;
    let (_tree, outcome) = Tree::with_saved(make(), &bytes, 8);
    ensure(CHECK, outcome == RestoreOutcome::SchemaChanged, || {
        format!("{outcome:?}")
    })
}

fn other_threads_wake_the_host<H: Host + 'static>(
    host: H,
    wakes: impl Fn() -> usize,
) -> Result<(), Violation> {
    const CHECK: &str = "other_threads_wake_the_host";
    let mut app = fixture(host);
    let Some(sender) = app.tree.get(app.root).map(|parent| parent.sender.clone()) else {
        return Err(Violation {
            check: CHECK,
            detail: "root is missing".to_owned(),
        });
    };
    let before = wakes();
    let joined = std::thread::spawn(move || sender.send(ParentMsg::Set(5)).is_ok()).join();
    ensure(CHECK, matches!(joined, Ok(true)), || format!("{joined:?}"))?;
    ensure(CHECK, wakes() > before, || "host was not woken".to_owned())?;
    ensure(CHECK, app.log.wakes() > 0, || {
        "waker was not installed".to_owned()
    })?;
    let pumped = app.tree.pump();
    ensure(CHECK, pumped.is_ok(), || format!("{pumped:?}"))?;
    let value = app.tree.get(app.root).map(|parent| parent.value);
    ensure(CHECK, value == Some(5), || format!("{value:?}"))
}

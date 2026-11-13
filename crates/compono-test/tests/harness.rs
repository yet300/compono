//! The test support itself, and task bookkeeping it makes observable.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]
#![allow(clippy::panic, reason = "a task panics on purpose")]

use compono::{Commit, Component, Cx, Host, Phase, Tree};
use compono_test::{NoHost, VirtualTime, check_host, headless};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Wake, Waker};
use std::time::Duration;

struct Ticker {
    time: VirtualTime,
    ticks: u32,
}

enum Msg {
    Arm,
    Fired,
}

impl Component for Ticker {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        if matches!(msg, Msg::Fired) {
            self.ticks += 1;
        }
        let sleep = self.time.sleep(Duration::from_millis(10));
        cx.perform(sleep, |()| Msg::Fired).unwrap();
    }
}

#[test]
fn timers_rearmed_inside_a_window_fire_in_that_window() {
    let (host, _log) = headless();
    let time = VirtualTime::new();
    let mut tree = Tree::new(host);
    tree.set_spawner(time.clone());
    let clock = time.clone();
    let ticker = tree.mount(move |_cx| Ticker {
        time: clock,
        ticks: 0,
    });
    tree.send(ticker, Msg::Arm).unwrap();
    time.advance(&mut tree, Duration::from_millis(35)).unwrap();
    assert_eq!(tree.get(ticker).unwrap().ticks, 3);
    assert_eq!(time.now(), Duration::from_millis(35));
    assert_eq!(time.live_tasks(), 1, "finished tasks are forgotten");
}

#[test]
fn a_long_running_timer_loop_does_not_accumulate_guards() {
    let (host, _log) = headless();
    let time = VirtualTime::new();
    let mut tree = Tree::new(host);
    tree.set_spawner(time.clone());
    let clock = time.clone();
    let ticker = tree.mount(move |_cx| Ticker {
        time: clock,
        ticks: 0,
    });
    tree.send(ticker, Msg::Arm).unwrap();
    time.advance(&mut tree, Duration::from_secs(100)).unwrap();
    assert_eq!(tree.get(ticker).unwrap().ticks, 10_000);
    assert_eq!(time.live_tasks(), 1);
}

#[test]
fn removing_the_component_cancels_its_timer() {
    let (host, _log) = headless();
    let time = VirtualTime::new();
    let mut tree = Tree::new(host);
    tree.set_spawner(time.clone());
    let clock = time.clone();
    let ticker = tree.mount(move |_cx| Ticker {
        time: clock,
        ticks: 0,
    });
    tree.send(ticker, Msg::Arm).unwrap();
    time.settle(&mut tree).unwrap();
    assert_eq!(time.live_tasks(), 1);
    tree.mount(|_cx| Idle);
    assert_eq!(time.live_tasks(), 0);
}

struct Idle;
impl Component for Idle {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

struct CountWakes(Arc<AtomicUsize>);
impl Wake for CountWakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct WakingHost(Arc<AtomicUsize>);
impl Host for WakingHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
    fn waker(&self) -> Waker {
        Waker::from(Arc::new(CountWakes(Arc::clone(&self.0))))
    }
}

#[test]
fn a_correct_host_passes() {
    let count = Arc::new(AtomicUsize::new(0));
    let shared = Arc::clone(&count);
    let reader = Arc::clone(&count);
    let result = check_host(
        move || WakingHost(Arc::clone(&shared)),
        move || reader.load(Ordering::SeqCst),
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn a_host_that_never_wakes_is_reported() {
    let error = check_host(|| NoHost, || 0).unwrap_err();
    assert_eq!(error.check, "other_threads_wake_the_host");
}

#[test]
fn phase_is_public_through_the_tree() {
    let (host, _log) = headless();
    let mut tree = Tree::new(host);
    let idle = tree.mount(|_cx| Idle);
    tree.set_phase(Phase::Visible);
    assert_eq!(tree.phase(idle), Some(Phase::Visible));
}

#[test]
fn a_delay_too_long_to_add_waits_forever_instead_of_overflowing() {
    let time = VirtualTime::new();
    let mut tree = Tree::new(headless().0);
    time.advance(&mut tree, Duration::from_secs(1)).unwrap();
    let sleep = time.sleep(Duration::MAX);
    time.advance(&mut tree, Duration::MAX).unwrap();
    assert_eq!(time.now(), Duration::MAX);
    drop(sleep);
}

/// Starts tasks that panic or answer, and reports whether the last is outstanding.
#[derive(Default)]
struct Risky {
    answers: Vec<u32>,
    task: Option<compono::TaskId>,
    outstanding: Option<bool>,
}

enum RiskyMsg {
    Panic,
    Answer(u32),
    Got(u32),
    Check,
}

impl Component for Risky {
    type Msg = RiskyMsg;
    type Output = ();
    fn update(&mut self, msg: RiskyMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            RiskyMsg::Panic => {
                let failing = async { panic!("the task fails on purpose") };
                self.task = Some(cx.perform(failing, RiskyMsg::Got).unwrap());
            }
            RiskyMsg::Answer(value) => {
                self.task = Some(cx.perform(async move { value }, RiskyMsg::Got).unwrap());
            }
            RiskyMsg::Got(value) => self.answers.push(value),
            RiskyMsg::Check => {
                self.outstanding = self.task.map(|task| cx.cancel(task));
            }
        }
    }
}

#[test]
fn a_task_that_panics_leaves_no_guard_and_delivers_nothing() {
    let (host, _log) = headless();
    let time = VirtualTime::new();
    let mut tree = Tree::new(host);
    tree.set_spawner(time.clone());
    let risky = tree.mount(|_cx| Risky::default());
    tree.send(risky, RiskyMsg::Panic).unwrap();
    time.settle(&mut tree).unwrap();
    tree.send(risky, RiskyMsg::Check).unwrap();
    assert_eq!(
        tree.get(risky).unwrap().outstanding,
        Some(false),
        "the guard went at the pump after the panic"
    );
    assert!(
        tree.get(risky).unwrap().answers.is_empty(),
        "nothing delivered"
    );

    tree.send(risky, RiskyMsg::Answer(4)).unwrap();
    time.settle(&mut tree).unwrap();
    assert_eq!(tree.get(risky).unwrap().answers, [4], "a later task works");
}

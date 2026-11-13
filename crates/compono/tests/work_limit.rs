//! One operation delivers a bounded amount of queued work, so components that keep
//! causing each other work cannot hold the host's thread.

#![cfg(feature = "std")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Commit, Component, Cx, Handle, Host, Tree};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Wake, Waker};

#[derive(Default)]
struct Wakes(AtomicUsize);

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// Counts commits and wake-ups, as a UI counts redraws and pump requests.
#[derive(Clone, Default)]
struct Counting {
    commits: Rc<Cell<usize>>,
    wakes: Arc<Wakes>,
}

impl Counting {
    fn commits(&self) -> usize {
        self.commits.get()
    }

    fn wakes(&self) -> usize {
        self.wakes.0.load(Ordering::Relaxed)
    }
}

impl Host for Counting {
    fn commit(&mut self, _commit: &Commit<'_>) {
        self.commits.set(self.commits.get() + 1);
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.wakes))
    }
}

/// Answers every message with an output, `left` more times.
struct Echo {
    left: u32,
}

impl Component for Echo {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        if self.left > 0 {
            self.left -= 1;
            cx.emit(());
        }
    }
}

/// Answers every output of its echo with another message, so each round trip
/// queues two pieces of work.
struct Parent {
    echo: Handle<Echo>,
    heard: u32,
}

impl Component for Parent {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

fn ping_pong(host: &Counting, rounds: u32) -> (Tree, Handle<Parent>, Handle<Echo>) {
    let mut tree = Tree::new(host.clone());
    let parent = tree.mount(|cx| Parent {
        echo: cx.child(
            |_cx| Echo { left: rounds },
            |parent: &mut Parent, (), cx| {
                parent.heard += 1;
                cx.send(parent.echo, ()).unwrap();
            },
        ),
        heard: 0,
    });
    let echo = tree.get(parent).unwrap().echo;
    (tree, parent, echo)
}

fn heard(tree: &Tree, parent: Handle<Parent>) -> u32 {
    tree.get(parent).unwrap().heard
}

#[test]
fn a_feedback_loop_returns_to_the_host_after_the_limit() {
    let host = Counting::default();
    let (mut tree, parent, echo) = ping_pong(&host, 50);
    tree.set_work_limit(10);
    let (commits, wakes) = (host.commits(), host.wakes());

    tree.send(echo, ()).unwrap();

    assert_eq!(
        heard(&tree, parent),
        5,
        "ten pieces of work are five round trips"
    );
    assert_eq!(host.commits(), commits + 1, "what was done is committed");
    assert!(host.wakes() > wakes, "the host is asked to pump the rest");
}

#[test]
fn pumping_delivers_the_work_an_operation_left() {
    let host = Counting::default();
    let (mut tree, parent, echo) = ping_pong(&host, 50);
    tree.set_work_limit(10);
    tree.send(echo, ()).unwrap();

    tree.pump().unwrap();
    assert_eq!(heard(&tree, parent), 10);

    for _ in 0..20 {
        tree.pump().unwrap();
    }
    assert_eq!(heard(&tree, parent), 50, "nothing is lost");
    let wakes = host.wakes();
    tree.pump().unwrap();
    assert_eq!(host.wakes(), wakes, "no wake-up once the work is done");
}

#[test]
fn work_within_the_limit_finishes_in_one_operation() {
    let host = Counting::default();
    let (mut tree, parent, echo) = ping_pong(&host, 4);
    tree.set_work_limit(10);
    let (commits, wakes) = (host.commits(), host.wakes());

    tree.send(echo, ()).unwrap();

    assert_eq!(heard(&tree, parent), 4);
    assert_eq!(host.commits(), commits + 1);
    assert_eq!(host.wakes(), wakes, "nothing is left to pump");
}

#[test]
fn an_endless_loop_returns_under_the_default_limit() {
    let host = Counting::default();
    let (mut tree, parent, echo) = ping_pong(&host, u32::MAX);
    let wakes = host.wakes();

    tree.send(echo, ()).unwrap();

    assert!(heard(&tree, parent) < u32::MAX);
    assert!(host.wakes() > wakes);
}

/// A future that is ready at once and counts how often it is polled.
struct Counted(Rc<Cell<usize>>);

impl std::future::Future for Counted {
    type Output = ();
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        self.0.set(self.0.get() + 1);
        std::task::Poll::Ready(())
    }
}

/// Starts ten local tasks when it is created and counts their results.
struct Many {
    done: u32,
}

impl Component for Many {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        self.done += 1;
    }
}

#[test]
fn one_pump_polls_no_more_local_tasks_than_the_poll_limit() {
    let host = Counting::default();
    let mut tree = Tree::new(host.clone());
    tree.set_poll_limit(3);
    let polls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&polls);
    let many = tree.mount(move |cx| {
        for _ in 0..10 {
            cx.perform_local(Counted(Rc::clone(&counter)), |()| ());
        }
        Many { done: 0 }
    });
    let wakes = host.wakes();

    tree.pump().unwrap();
    assert_eq!(polls.get(), 3, "one pump polls three tasks");
    assert!(host.wakes() > wakes, "the host is asked to pump the rest");

    for _ in 0..10 {
        tree.pump().unwrap();
    }
    assert_eq!(polls.get(), 10, "every task is polled once");
    assert_eq!(tree.get(many).unwrap().done, 10, "every result arrives");
}

#[test]
fn the_work_limit_does_not_bound_polls() {
    let host = Counting::default();
    let mut tree = Tree::new(host.clone());
    tree.set_work_limit(3);
    let polls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&polls);
    tree.mount(move |cx| {
        for _ in 0..10 {
            cx.perform_local(Counted(Rc::clone(&counter)), |()| ());
        }
        Many { done: 0 }
    });
    tree.pump().unwrap();
    assert_eq!(
        polls.get(),
        10,
        "below the poll limit, one pump polls them all"
    );
}

#[test]
fn a_poll_limit_of_zero_polls_one_task() {
    let mut tree = Tree::new(Counting::default());
    tree.set_poll_limit(0);
    let polls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&polls);
    tree.mount(move |cx| {
        for _ in 0..2 {
            cx.perform_local(Counted(Rc::clone(&counter)), |()| ());
        }
        Many { done: 0 }
    });
    tree.pump().unwrap();
    assert_eq!(polls.get(), 1);
}

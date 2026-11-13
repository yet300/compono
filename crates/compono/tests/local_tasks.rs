//! Tasks that are not `Send`, polled by the tree on its own thread.

#![cfg(feature = "std")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Commit, Component, Cx, Host, TaskId, Tree};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

/// A host whose waker counts wake-ups, as a UI would count repaint requests.
struct Counting(Arc<Wakes>);

#[derive(Default)]
struct Wakes(AtomicUsize);

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

impl Host for Counting {
    fn commit(&mut self, _commit: &Commit<'_>) {}
    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.0))
    }
}

/// A single-threaded one-shot value, which is not `Send`.
#[derive(Default)]
struct Shared {
    value: Cell<Option<u32>>,
    waker: RefCell<Option<Waker>>,
    dropped: Cell<bool>,
}

struct Receive(Rc<Shared>);

impl Future for Receive {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        match self.0.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                *self.0.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Drop for Receive {
    fn drop(&mut self) {
        self.0.dropped.set(true);
    }
}

fn send(shared: &Shared, value: u32) {
    shared.value.set(Some(value));
    if let Some(waker) = shared.waker.borrow_mut().take() {
        waker.wake();
    }
}

enum Msg {
    Start(Rc<Shared>),
    Got(u32),
    Cancel,
}

#[derive(Default)]
struct Loader {
    got: Vec<u32>,
    task: Option<TaskId>,
    cancelled: bool,
}

impl Component for Loader {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Start(shared) => self.task = Some(cx.perform_local(Receive(shared), Msg::Got)),
            Msg::Got(value) => self.got.push(value),
            Msg::Cancel => self.cancelled = self.task.is_some_and(|task| cx.cancel(task)),
        }
    }
}

struct Holder {
    loader: Option<compono::Handle<Loader>>,
}

impl Component for Holder {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        if let Some(loader) = self.loader.take() {
            cx.destroy(loader).unwrap();
        }
    }
}

fn setup() -> (
    Tree,
    compono::Handle<Holder>,
    compono::Handle<Loader>,
    Arc<Wakes>,
) {
    let wakes = Arc::new(Wakes::default());
    let mut tree = Tree::new(Counting(Arc::clone(&wakes)));
    let mut loader = None;
    let holder = tree.mount(|cx| {
        loader = Some(cx.child(|_cx| Loader::default(), |_holder, (), _cx| {}));
        Holder { loader }
    });
    (tree, holder, loader.unwrap(), wakes)
}

#[test]
fn a_task_that_is_not_send_delivers_after_the_pump_it_asks_for() {
    let (mut tree, _holder, loader, wakes) = setup();
    let shared = Rc::new(Shared::default());
    shared.value.set(Some(7));
    tree.send(loader, Msg::Start(Rc::clone(&shared))).unwrap();
    assert_eq!(
        wakes.0.load(Ordering::Relaxed),
        1,
        "starting a task asks for a pump"
    );
    assert!(
        tree.get(loader).unwrap().got.is_empty(),
        "nothing runs before the pump"
    );
    tree.pump().unwrap();
    assert_eq!(tree.get(loader).unwrap().got, [7]);
    assert!(shared.dropped.get(), "a finished future is dropped");
}

#[test]
fn a_task_woken_later_is_polled_at_the_next_pump() {
    let (mut tree, _holder, loader, wakes) = setup();
    let shared = Rc::new(Shared::default());
    tree.send(loader, Msg::Start(Rc::clone(&shared))).unwrap();
    tree.pump().unwrap();
    assert!(
        tree.get(loader).unwrap().got.is_empty(),
        "the value has not arrived"
    );
    let before = wakes.0.load(Ordering::Relaxed);
    send(&shared, 9);
    assert_eq!(
        wakes.0.load(Ordering::Relaxed),
        before + 1,
        "a wake-up reaches the host"
    );
    tree.pump().unwrap();
    assert_eq!(tree.get(loader).unwrap().got, [9]);
    tree.pump().unwrap();
    assert_eq!(tree.get(loader).unwrap().got, [9], "a result arrives once");
}

#[test]
fn destroying_the_component_drops_its_task() {
    let (mut tree, holder, loader, _wakes) = setup();
    let shared = Rc::new(Shared::default());
    tree.send(loader, Msg::Start(Rc::clone(&shared))).unwrap();
    tree.pump().unwrap();
    tree.send(holder, ()).unwrap();
    assert!(shared.dropped.get(), "the future goes with its component");
    send(&shared, 5);
    tree.pump().unwrap();
    assert!(tree.get(loader).is_none());
}

#[test]
fn a_cancelled_task_never_delivers() {
    let (mut tree, _holder, loader, _wakes) = setup();
    let shared = Rc::new(Shared::default());
    shared.value.set(Some(3));
    tree.send(loader, Msg::Start(Rc::clone(&shared))).unwrap();
    tree.send(loader, Msg::Cancel).unwrap();
    assert!(tree.get(loader).unwrap().cancelled, "cancel finds the task");
    assert!(shared.dropped.get());
    tree.pump().unwrap();
    assert!(tree.get(loader).unwrap().got.is_empty());
}

/// Never finishes, counts its polls and hands out its waker.
struct Counted {
    polls: Rc<Cell<u32>>,
    waker: Rc<RefCell<Option<Waker>>>,
}

impl Future for Counted {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        self.polls.set(self.polls.get() + 1);
        *self.waker.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[test]
fn many_wakes_between_two_pumps_poll_the_task_once() {
    let wakes = Arc::new(Wakes::default());
    let mut tree = Tree::new(Counting(Arc::clone(&wakes)));
    let polls = Rc::new(Cell::new(0));
    let waker = Rc::new(RefCell::new(None));
    let future = Counted {
        polls: Rc::clone(&polls),
        waker: Rc::clone(&waker),
    };
    tree.mount(|cx| {
        cx.perform_local(future, |_value| ());
        Unit
    });
    tree.pump().unwrap();
    assert_eq!(polls.get(), 1, "the first pump polls the task");
    let waker = waker.borrow_mut().take().unwrap();
    let before = wakes.0.load(Ordering::Relaxed);
    std::thread::spawn(move || {
        for _ in 0..10_000 {
            waker.wake_by_ref();
        }
    })
    .join()
    .unwrap();
    assert_eq!(
        wakes.0.load(Ordering::Relaxed),
        before + 1,
        "the host is woken once for a task already waiting"
    );
    tree.pump().unwrap();
    assert_eq!(polls.get(), 2, "the next pump polls the task once");
    tree.pump().unwrap();
    assert_eq!(polls.get(), 2, "nothing is left waiting");
}

struct Unit;

impl Component for Unit {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

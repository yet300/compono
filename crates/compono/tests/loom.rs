//! Model checks of the concurrency module over every thread interleaving.
//!
//! Run with `RUSTFLAGS="--cfg loom" cargo test -p compono --test loom --release`,
//! in its own target directory so the rest of the build is not recompiled.

#![cfg(loom)]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Commit, Component, Cx, Host, Tree, WakeSignal};
use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Wake, Waker};

/// Records whether it was woken.
#[derive(Default)]
struct Flag(AtomicBool);

impl Wake for Flag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[test]
fn a_wake_racing_with_a_poll_is_never_lost() {
    loom::model(|| {
        let signal = WakeSignal::new();
        let waker = signal.waker();
        let wake = loom::thread::spawn(move || waker.wake());

        let listener = Arc::new(Flag::default());
        let listener_waker = Waker::from(Arc::clone(&listener));
        let ready = signal
            .poll_wake(&Context::from_waker(&listener_waker))
            .is_ready();
        wake.join().unwrap();

        assert!(
            ready || listener.0.load(Ordering::SeqCst),
            "the poll saw the wake-up, or the wake-up reached the listener"
        );
    });
}

#[test]
fn many_wakes_before_a_wait_become_one() {
    loom::model(|| {
        let signal = WakeSignal::new();
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let waker = signal.waker();
                loom::thread::spawn(move || waker.wake())
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let idle = Waker::noop();
        assert!(signal.poll_wake(&Context::from_waker(idle)).is_ready());
        assert!(signal.poll_wake(&Context::from_waker(idle)).is_pending());
    });
}

/// A host whose waker counts wake-ups.
struct Counting(Arc<Wakes>);

#[derive(Default)]
struct Wakes(AtomicUsize);

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl Host for Counting {
    fn commit(&mut self, _commit: &Commit<'_>) {}

    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.0))
    }
}

struct Total(u32);

impl Component for Total {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 += msg;
    }
}

#[test]
fn messages_sent_while_pumping_are_delivered_once() {
    loom::model(|| {
        let wakes = Arc::new(Wakes::default());
        let mut tree = Tree::new(Counting(Arc::clone(&wakes)));
        let mut sender = None;
        let root = tree.mount(|cx| {
            sender = Some(cx.sender());
            Total(0)
        });
        let sender = sender.unwrap();
        let senders: Vec<_> = [1, 10]
            .into_iter()
            .map(|amount| {
                let sender = sender.clone();
                loom::thread::spawn(move || sender.send(amount).unwrap())
            })
            .collect();
        tree.pump().unwrap();
        for thread in senders {
            thread.join().unwrap();
        }
        tree.pump().unwrap();

        assert_eq!(
            tree.get(root).unwrap().0,
            11,
            "each message arrives exactly once"
        );
        assert_eq!(
            wakes.0.load(Ordering::SeqCst),
            2,
            "each send wakes the host"
        );
    });
}

/// A message that shows whether it is still held somewhere.
struct Parcel(#[expect(dead_code, reason = "held only to be counted")] Arc<()>);

struct Sink;

impl Component for Sink {
    type Msg = Parcel;
    type Output = ();
    fn update(&mut self, _msg: Parcel, _cx: &mut Cx<'_, Self>) {}
}

#[test]
fn a_send_racing_with_the_trees_drop_is_dropped_or_refused() {
    loom::model(|| {
        let mut tree = Tree::new(Counting(Arc::default()));
        let mut sender = None;
        tree.mount(|cx| {
            sender = Some(cx.sender());
            Sink
        });
        let sender = sender.unwrap();
        let held = Arc::new(());
        let parcel = Parcel(Arc::clone(&held));
        let thread = loom::thread::spawn(move || {
            let _accepted_or_refused = sender.send(parcel);
            // The sender outlives the tree in some interleavings.
            sender
        });
        drop(tree);
        let sender = thread.join().unwrap();
        assert_eq!(
            Arc::strong_count(&held),
            1,
            "no queue keeps a message once its tree is gone"
        );
        drop(sender);
    });
}

/// Becomes ready once another thread sets its flag and wakes it.
struct Flagged {
    ready: Arc<AtomicBool>,
    waker: Arc<loom::sync::Mutex<Option<Waker>>>,
}

impl std::future::Future for Flagged {
    type Output = u32;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<u32> {
        *self.waker.lock().unwrap() = Some(cx.waker().clone());
        if self.ready.load(Ordering::SeqCst) {
            std::task::Poll::Ready(1)
        } else {
            std::task::Poll::Pending
        }
    }
}

struct Local(u32);

impl Component for Local {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 += msg;
    }
}

#[test]
fn a_local_task_woken_from_another_thread_is_never_lost() {
    loom::model(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let waker: Arc<loom::sync::Mutex<Option<Waker>>> = Arc::default();
        let mut tree = Tree::new(Counting(Arc::new(Wakes::default())));
        let future = Flagged {
            ready: Arc::clone(&ready),
            waker: Arc::clone(&waker),
        };
        let root = tree.mount(|cx| {
            cx.perform_local(future, |value| value);
            Local(0)
        });
        tree.pump().unwrap();

        let (flag, slot) = (Arc::clone(&ready), Arc::clone(&waker));
        let other = loom::thread::spawn(move || {
            flag.store(true, Ordering::SeqCst);
            if let Some(waker) = slot.lock().unwrap().take() {
                waker.wake();
            }
        });
        tree.pump().unwrap();
        other.join().unwrap();
        tree.pump().unwrap();

        assert_eq!(
            tree.get(root).unwrap().0,
            1,
            "the result arrives exactly once"
        );
    });
}

#[test]
fn a_local_task_woken_while_it_is_polled_is_polled_again() {
    loom::model(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let waker: Arc<loom::sync::Mutex<Option<Waker>>> = Arc::default();
        let mut tree = Tree::new(Counting(Arc::new(Wakes::default())));
        let future = Flagged {
            ready: Arc::clone(&ready),
            waker: Arc::clone(&waker),
        };
        let root = tree.mount(|cx| {
            cx.perform_local(future, |value| value);
            Local(0)
        });
        tree.pump().unwrap();
        // Listed again, so the next pump polls the task while the other
        // thread finishes it and wakes it.
        waker.lock().unwrap().as_ref().unwrap().wake_by_ref();

        let (flag, slot) = (Arc::clone(&ready), Arc::clone(&waker));
        let other = loom::thread::spawn(move || {
            flag.store(true, Ordering::Relaxed);
            let waker = slot.lock().unwrap().clone();
            if let Some(waker) = waker {
                waker.wake();
            }
        });
        tree.pump().unwrap();
        other.join().unwrap();
        tree.pump().unwrap();

        assert_eq!(
            tree.get(root).unwrap().0,
            1,
            "a wake-up during the poll is not lost"
        );
    });
}

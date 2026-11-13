//! `IcedHost` under the host conformance suite, and the wake signal.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Component, Cx, Tree, WakeSignal};
use compono_iced::{IcedHost, send};
use compono_test::check_host;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::task::{Context, Poll, Wake, Waker};

#[test]
fn iced_host_conforms() {
    let latest: Arc<Mutex<Option<WakeSignal>>> = Arc::default();
    let writer = Arc::clone(&latest);
    let result = check_host(
        move || {
            let host = IcedHost::new();
            *writer.lock().unwrap_or_else(PoisonError::into_inner) = Some(host.signal());
            host
        },
        move || {
            let signal = latest.lock().unwrap_or_else(PoisonError::into_inner);
            usize::from(signal.as_ref().is_some_and(WakeSignal::is_pending))
        },
    );
    assert_eq!(result, Ok(()));
}

struct Count(AtomicUsize);
impl Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn many_wakes_become_one_and_the_listener_is_called() {
    let host = IcedHost::new();
    let signal = host.signal();
    let count = Arc::new(Count(AtomicUsize::new(0)));
    let listener = Waker::from(Arc::clone(&count));
    let context = Context::from_waker(&listener);

    assert_eq!(signal.poll_wake(&context), Poll::Pending);
    let tree_waker = compono::Host::waker(&host);
    for _ in 0..50 {
        tree_waker.wake_by_ref();
    }
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
    assert_eq!(signal.poll_wake(&context), Poll::Ready(()));
    assert_eq!(signal.poll_wake(&context), Poll::Pending);
}

struct Counter(u32);
impl Component for Counter {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 += msg;
    }
}

#[test]
fn an_action_can_be_applied_repeatedly() {
    let mut tree = Tree::new(IcedHost::new());
    let counter = tree.mount(|_cx| Counter(0));
    let action = send(counter, 2);
    let copy = action.clone();
    action.apply(&mut tree);
    copy.apply(&mut tree);
    assert_eq!(tree.get(counter).unwrap().0, 4);
}

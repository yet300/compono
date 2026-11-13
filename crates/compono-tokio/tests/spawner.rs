//! Real threads and real time: a task result reaches the tree through the waker.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Commit, Component, Cx, Host, Tree};
use compono_tokio::TokioSpawner;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::task::{Wake, Waker};
use std::time::Duration;

struct Notify(Mutex<Sender<()>>);
impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        let _closed = self.0.lock().unwrap().send(());
    }
}

struct WakeHost(Arc<Notify>);
impl Host for WakeHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.0))
    }
}

struct Waiter {
    done: bool,
}

struct Dropped(Option<Sender<()>>);
impl Drop for Dropped {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _closed = sender.send(());
        }
    }
}

enum Msg {
    Wait(Duration, Option<Sender<()>>),
    Finished,
}

impl Component for Waiter {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Wait(delay, on_drop) => {
                let marker = Dropped(on_drop);
                let task = async move {
                    tokio::time::sleep(delay).await;
                    drop(marker);
                };
                cx.perform(task, |()| Msg::Finished).unwrap();
            }
            Msg::Finished => self.done = true,
        }
    }
}

fn setup(runtime: &tokio::runtime::Runtime) -> (Tree, Receiver<()>) {
    let (sender, receiver) = channel();
    let mut tree = Tree::new(WakeHost(Arc::new(Notify(Mutex::new(sender)))));
    tree.set_spawner(TokioSpawner::new(runtime.handle().clone()));
    (tree, receiver)
}

#[test]
fn a_finished_task_wakes_the_host_and_arrives_as_a_message() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (mut tree, woken) = setup(&runtime);
    let waiter = tree.mount(|_cx| Waiter { done: false });
    tree.send(waiter, Msg::Wait(Duration::from_millis(5), None))
        .unwrap();
    woken.recv_timeout(Duration::from_secs(5)).unwrap();
    tree.pump().unwrap();
    assert!(tree.get(waiter).unwrap().done);
}

#[test]
fn removing_the_component_aborts_the_running_task() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (mut tree, _woken) = setup(&runtime);
    let waiter = tree.mount(|_cx| Waiter { done: false });
    let (sender, dropped) = channel();
    tree.send(waiter, Msg::Wait(Duration::from_secs(3600), Some(sender)))
        .unwrap();
    tree.mount(|_cx| Waiter { done: false });
    dropped.recv_timeout(Duration::from_secs(5)).unwrap();
}

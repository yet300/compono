//! Measures the showcase under load, without any UI.
//!
//! ```text
//! CARGO_PROFILE_RELEASE_PANIC=abort cargo run -p showcase --release --example load
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a measurement may stop on failure"
)]

use compono::{Commit, Handle, Host, Phase, Tree};
use showcase::Services;
use showcase::feature::counters::{Counters, CountersMsg, DEEP_PUSH};
use showcase::feature::feed::{Feed, FeedMsg, ROWS};
use showcase::feature::root::{Child, Root};
use showcase::feature::tabs::{Tab, TabChild, Tabs, TabsMsg};
use std::cell::Cell;
use std::future::{Future, pending};
use std::rc::Rc;
use std::time::Instant;

struct CountingHost(Rc<Cell<u64>>);
impl Host for CountingHost {
    fn commit(&mut self, commit: &Commit<'_>) {
        self.0.set(self.0.get() + commit.dirty.len() as u64);
    }
}

struct IdleSpawner;
impl compono::Spawner for IdleSpawner {
    fn spawn(
        &self,
        task: std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> compono::TaskGuard {
        compono::TaskGuard::new(core::cell::Cell::new(Some(task)))
    }
}

fn report(name: &str, operations: u64, started: Instant) {
    let nanos = started.elapsed().as_nanos() as f64 / operations as f64;
    println!("{name:<44} {nanos:>12.0} ns");
}

fn select(tree: &mut Tree, tabs: Handle<Tabs>, tab: Tab) -> TabChild {
    tree.send(tabs, TabsMsg::Select(tab)).unwrap();
    *tree.get(tabs).unwrap().stack.active()
}

fn main() {
    let dirty = Rc::new(Cell::new(0));
    let mut tree = Tree::new(CountingHost(Rc::clone(&dirty)));
    tree.set_spawner(IdleSpawner);
    let services = Services::new(|_delay| Box::pin(pending()));
    let root: Handle<Root> = tree.mount(move |cx| Root::new(cx, services));
    tree.set_phase(Phase::Focused);
    let Child::Tabs(tabs) = *tree.get(root).unwrap().stack.active() else {
        panic!("expected tabs");
    };

    let TabChild::Counters(counters) = select(&mut tree, tabs, Tab::Counters) else {
        panic!("expected counters");
    };
    deep_stack(&mut tree, counters);
    tab_switching(&mut tree, tabs);

    let TabChild::Feed(feed) = select(&mut tree, tabs, Tab::Feed) else {
        panic!("expected feed");
    };
    scrolling(&mut tree, feed);
    live_frames(&mut tree, feed, &dirty);
}

fn deep_stack(tree: &mut Tree, counters: Handle<Counters>) {
    let started = Instant::now();
    tree.send(counters, CountersMsg::GoDeep).unwrap();
    report(
        "push one screen, in a burst of 1000",
        DEEP_PUSH as u64,
        started,
    );
    let started = Instant::now();
    tree.send(counters, CountersMsg::PopToRoot).unwrap();
    report(
        "pop one screen, in a burst of 1000",
        DEEP_PUSH as u64,
        started,
    );
}

fn tab_switching(tree: &mut Tree, tabs: Handle<Tabs>) {
    const SWITCHES: u64 = 100_000;
    for (_title, tab) in Tab::ALL {
        tree.send(tabs, TabsMsg::Select(tab)).unwrap();
    }
    let started = Instant::now();
    for index in 0..SWITCHES {
        let tab = if index % 2 == 0 {
            Tab::Cards
        } else {
            Tab::Articles
        };
        tree.send(tabs, TabsMsg::Select(tab)).unwrap();
    }
    report("switch tab, keeping every tab alive", SWITCHES, started);
}

fn scrolling(tree: &mut Tree, feed: Handle<Feed>) {
    const WINDOW: usize = 30;
    let rows = usize::try_from(ROWS).unwrap();
    let started = Instant::now();
    for start in 0..rows - WINDOW {
        tree.send(feed, FeedMsg::Window(start..start + WINDOW))
            .unwrap();
    }
    report(
        "scroll one row, through all 100000",
        (rows - WINDOW) as u64,
        started,
    );
    let started = Instant::now();
    for start in (0..rows - WINDOW).step_by(WINDOW) {
        tree.send(feed, FeedMsg::Window(start..start + WINDOW))
            .unwrap();
    }
    report(
        "jump a full window (30 rows replaced)",
        ((rows - WINDOW) / WINDOW) as u64,
        started,
    );
    tree.send(feed, FeedMsg::Window(1_000..1_000 + WINDOW))
        .unwrap();
}

fn live_frames(tree: &mut Tree, feed: Handle<Feed>, dirty: &Cell<u64>) {
    const FRAMES: u64 = 100_000;
    dirty.set(0);
    let started = Instant::now();
    for _ in 0..FRAMES {
        tree.send(feed, FeedMsg::Frame).unwrap();
    }
    report("frame updating every live row (30)", FRAMES, started);
    println!(
        "components reported changed per frame: {}",
        dirty.get() / FRAMES
    );
}

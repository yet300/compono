//! Removing the entries of dropped routers counts against the work limit, so a
//! component that drops large routers cannot hold the host's thread.

#![cfg(feature = "std")]
#![expect(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::router::set::ChildSet;
use compono::{Commit, Component, Cx, Handle, Host, Phase, Tree};
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

#[derive(Clone, Default)]
struct Waking(Arc<Wakes>);

impl Host for Waking {
    fn commit(&mut self, _commit: &Commit<'_>) {}

    fn waker(&self) -> Waker {
        Waker::from(Arc::clone(&self.0))
    }
}

/// How many leaves were removed.
#[derive(Clone, Default)]
struct Removed(Rc<Cell<usize>>);

impl Removed {
    fn get(&self) -> usize {
        self.0.get()
    }
}

struct Leaf(Removed);

impl Component for Leaf {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Drop for Leaf {
    fn drop(&mut self) {
        self.0.0.set(self.0.get() + 1);
    }
}

const LEAVES: u32 = 1_000;

/// Drops a new router of `LEAVES` entries in each `phase_changed`.
struct Dropper(Removed);

impl Component for Dropper {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
    fn phase_changed(&mut self, _phase: Phase, cx: &mut Cx<'_, Self>) {
        let removed = self.0.clone();
        let mut set = ChildSet::new(cx, move |_key: &u32, cx: &mut Cx<'_, Self>| {
            let removed = removed.clone();
            cx.child(|_cx| Leaf(removed), |_dropper, (), _cx| {});
        });
        let keys = (0..LEAVES).map(|key| (key, Phase::Hidden));
        set.reconcile(keys, cx).unwrap();
        // The dropped router takes its entries with it when the operation ends.
        drop(set);
    }
}

/// Hides and shows its dropper on each message.
struct Root {
    dropper: Handle<Dropper>,
    shown: bool,
}

impl Component for Root {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        self.shown = !self.shown;
        let phase = if self.shown {
            Phase::Focused
        } else {
            Phase::Hidden
        };
        cx.set_child_phase(self.dropper, phase).unwrap();
    }
}

const LIMIT: usize = 100;

fn dropping() -> (Tree, Waking, Removed, Handle<Root>) {
    let host = Waking::default();
    let removed = Removed::default();
    let mut tree = Tree::new(host.clone());
    tree.set_phase(Phase::Focused);
    let root = tree.mount(|cx| {
        let removed = removed.clone();
        Root {
            dropper: cx.child(|_cx| Dropper(removed), |_root, (), _cx| {}),
            shown: true,
        }
    });
    tree.set_work_limit(LIMIT);
    (tree, host, removed, root)
}

#[test]
fn one_operation_removes_no_more_than_the_work_limit() {
    let (mut tree, host, removed, root) = dropping();
    let before = removed.get();
    tree.send(root, ()).unwrap();
    let removed_now = removed.get() - before;
    assert!(
        removed_now <= LIMIT,
        "one operation removed {removed_now} components"
    );
    assert!(
        host.0.0.load(Ordering::Relaxed) > 0,
        "the host is asked to pump"
    );
}

#[test]
fn pumping_continues_the_removals_until_they_end() {
    let (mut tree, _host, removed, root) = dropping();
    let before = removed.get();
    tree.send(root, ()).unwrap();
    let mut pumps = 0;
    while removed.get() - before < LEAVES as usize && pumps < LEAVES {
        let step = removed.get();
        tree.pump().unwrap();
        assert!(
            removed.get() - step <= LIMIT,
            "a pump stays within the limit"
        );
        pumps += 1;
    }
    assert_eq!(
        removed.get() - before,
        LEAVES as usize,
        "every dropped entry is removed"
    );
}

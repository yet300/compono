//! A tree driven the way a native UI drives it.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Component, Cx, Handle, NodeId, Phase, RestoreOutcome};
use compono_ffi::{
    NativeError, NativeHost, NativeTree, Platform, collect_retired, phase_code, phase_from_code,
};
use compono_test::check_host;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

#[derive(Default)]
struct Native {
    commits: Mutex<Vec<(Vec<u64>, bool)>>,
    pumps: AtomicUsize,
    /// Set to read the tree back from inside `commit`.
    reentry: Mutex<Option<Arc<NativeTree>>>,
    reentry_result: Mutex<Option<Result<(), NativeError>>>,
}

impl Platform for Native {
    fn commit(&self, dirty: &[u64], can_back: bool) {
        self.commits
            .lock()
            .unwrap()
            .push((dirty.to_vec(), can_back));
        if let Some(tree) = self.reentry.lock().unwrap().as_ref() {
            *self.reentry_result.lock().unwrap() = Some(tree.read(|_tree| ()));
        }
    }

    fn schedule_pump(&self) {
        self.pumps.fetch_add(1, Ordering::SeqCst);
    }
}

struct Counter(u32);

impl Component for Counter {
    type Msg = u32;
    type Output = ();

    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 += msg;
    }
}

/// Reads the counter at `node`, as an exported function of a native UI does.
fn count(tree: &NativeTree, node: u64) -> Result<u32, NativeError> {
    tree.read(|tree| {
        let handle = tree
            .handle_of::<Counter>(NodeId::from_bits(node))
            .ok_or(NativeError::Gone)?;
        tree.get(handle)
            .map(|counter| counter.0)
            .ok_or(NativeError::Gone)
    })?
}

struct Other;

impl Component for Other {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

fn setup() -> (Arc<Native>, NativeTree, u64) {
    let native = Arc::new(Native::default());
    let tree = NativeTree::new(native.clone());
    let counter = tree.with(|tree| tree.mount(|_cx| Counter(0))).unwrap();
    native.commits.lock().unwrap().clear();
    (native, tree, counter.id().to_bits())
}

#[test]
fn native_host_conforms() {
    let native = Arc::new(Native::default());
    let wakes = Arc::clone(&native);
    let result = check_host(
        move || NativeHost::new(native.clone()),
        move || wakes.pumps.swap(0, Ordering::SeqCst),
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn a_message_by_address_reports_one_commit_and_updates_the_model() {
    let (native, tree, counter) = setup();
    tree.send::<Counter>(counter, 5).unwrap();
    assert_eq!(count(&tree, counter), Ok(5));
    assert_eq!(*native.commits.lock().unwrap(), [(vec![counter], false)]);
}

#[test]
fn several_changes_in_one_call_list_each_component_once() {
    let (native, tree, counter) = setup();
    let handle = tree
        .read(|tree| {
            tree.handle_of::<Counter>(compono::NodeId::from_bits(counter))
                .unwrap()
        })
        .unwrap();
    tree.with(|tree| {
        tree.send(handle, 1).unwrap();
        tree.send(handle, 1).unwrap();
    })
    .unwrap();
    assert_eq!(*native.commits.lock().unwrap(), [(vec![counter], false)]);
}

#[test]
fn the_platform_may_read_the_tree_from_inside_commit() {
    let native = Arc::new(Native::default());
    let tree = Arc::new(NativeTree::new(native.clone()));
    let counter: Handle<Counter> = tree.with(|tree| tree.mount(|_cx| Counter(0))).unwrap();
    *native.reentry.lock().unwrap() = Some(Arc::clone(&tree));
    tree.send::<Counter>(counter.id().to_bits(), 1).unwrap();
    assert_eq!(*native.reentry_result.lock().unwrap(), Some(Ok(())));
    native.reentry.lock().unwrap().take();
}

#[test]
fn a_call_from_inside_a_call_is_refused() {
    let (_native, tree, _counter) = setup();
    let inner = tree.with(|_tree| tree.with(|_inner| ())).unwrap();
    assert_eq!(inner, Err(NativeError::Busy));
}

#[test]
fn another_thread_is_refused_and_the_tree_is_untouched() {
    let (_native, tree, counter) = setup();
    let result = std::thread::scope(|scope| {
        scope
            .spawn(|| tree.send::<Counter>(counter, 1))
            .join()
            .unwrap()
    });
    assert_eq!(result, Err(NativeError::WrongThread));
    assert_eq!(count(&tree, counter), Ok(0));
}

#[test]
fn a_gone_or_mistyped_address_is_an_error() {
    let (_native, tree, counter) = setup();
    assert_eq!(tree.send::<Other>(counter, ()), Err(NativeError::Gone));
    assert_eq!(tree.send::<Counter>(u64::MAX, 1), Err(NativeError::Gone));
}

#[test]
fn work_from_another_thread_asks_for_a_pump_and_arrives_on_it() {
    let native = Arc::new(Native::default());
    let tree = NativeTree::new(native.clone());
    let mut sender = None;
    let counter = tree
        .with(|tree| {
            tree.mount(|cx: &mut Cx<'_, Counter>| {
                sender = Some(cx.sender());
                Counter(0)
            })
        })
        .unwrap();
    let counter = counter.id().to_bits();
    let sender = sender.unwrap();
    std::thread::spawn(move || sender.send(7).unwrap())
        .join()
        .unwrap();
    assert_eq!(native.pumps.load(Ordering::SeqCst), 1);
    assert_eq!(count(&tree, counter), Ok(0));
    tree.pump().unwrap();
    assert_eq!(count(&tree, counter), Ok(7));
}

struct Watched(Arc<AtomicUsize>);

impl Component for Watched {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Drop for Watched {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn dropping_on_another_thread_frees_the_tree_on_its_owner() {
    let drops = Arc::new(AtomicUsize::new(0));
    let tree = NativeTree::new(Arc::new(Native::default()));
    let watched = Arc::clone(&drops);
    tree.with(|tree| tree.mount(|_cx| Watched(watched)))
        .unwrap();
    std::thread::spawn(move || drop(tree)).join().unwrap();
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "wrong thread must not free"
    );
    collect_retired();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn dropping_on_another_thread_asks_the_owner_to_collect_the_tree() {
    let drops = Arc::new(AtomicUsize::new(0));
    let native = Arc::new(Native::default());
    let platform = Arc::downgrade(&native);
    let tree = NativeTree::new(Arc::clone(&native) as Arc<dyn Platform>);
    let watched = Arc::clone(&drops);
    tree.with(|tree| tree.mount(|_cx| Watched(watched)))
        .unwrap();
    let before = native.pumps.load(Ordering::SeqCst);
    std::thread::spawn(move || drop(tree)).join().unwrap();
    assert_eq!(
        native.pumps.load(Ordering::SeqCst),
        before + 1,
        "the owner is asked for a call"
    );
    drop(native);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "wrong thread must not free"
    );
    // The call the platform asked for, on a thread with no live tree left.
    collect_retired();
    assert_eq!(drops.load(Ordering::SeqCst), 1, "the record is gone");
    assert!(platform.upgrade().is_none(), "the platform is released");
}

#[test]
fn dropping_on_the_owner_frees_at_once_and_a_platform_is_released() {
    let native = Arc::new(Native::default());
    let weak: Weak<Native> = Arc::downgrade(&native);
    let tree = NativeTree::new(native);
    drop(tree);
    assert!(weak.upgrade().is_none());
}

#[test]
fn phase_codes_round_trip_and_unknown_codes_are_refused() {
    for phase in [Phase::Hidden, Phase::Visible, Phase::Focused] {
        assert_eq!(phase_from_code(phase_code(phase)), Some(phase));
    }
    assert_eq!(phase_from_code(3), None);
}

#[test]
fn state_saved_by_one_tree_is_read_by_another() {
    let (_native, tree, _counter) = setup();
    let bytes = tree.save(4).unwrap();
    let (_tree, outcome) = NativeTree::restore(Arc::new(Native::default()), &bytes, 4);
    assert_eq!(outcome, RestoreOutcome::Restored);
    let (_tree, outcome) = NativeTree::restore(Arc::new(Native::default()), &bytes, 5);
    assert_eq!(outcome, RestoreOutcome::SchemaChanged);
}

struct Closable;

impl Component for Closable {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}

    fn back(&self) -> Option<()> {
        Some(())
    }
}

#[test]
fn a_gesture_names_its_target_and_ends_on_either_side() {
    let tree = NativeTree::new(Arc::new(Native::default()));
    assert_eq!(tree.back_start(), Ok(None), "nothing handles Back yet");
    let node = tree.with(|tree| tree.mount(|_cx| Closable)).unwrap();
    tree.set_phase(Phase::Focused).unwrap();
    assert_eq!(tree.back_start(), Ok(Some(node.id().to_bits())));
    tree.back_cancel().unwrap();
    assert_eq!(tree.read(compono::Tree::gesture_target).unwrap(), None);
    tree.back_start().unwrap();
    assert_eq!(tree.back_commit(), Ok(true));
    assert_eq!(tree.read(compono::Tree::gesture_target).unwrap(), None);
}

#[test]
fn several_commits_in_one_call_never_name_a_removed_component() {
    let (native, tree, counter) = setup();
    let old = tree
        .read(|tree| {
            tree.handle_of::<Counter>(compono::NodeId::from_bits(counter))
                .unwrap()
        })
        .unwrap();
    tree.with(|tree| {
        tree.send(old, 1).unwrap();
        // Replacing the root removes the old one and reports nothing itself.
        tree.mount(|_cx| Counter(0));
    })
    .unwrap();
    let commits = native.commits.lock().unwrap().clone();
    assert_eq!(commits, [(vec![], false)]);
}

struct Bomb;

impl Component for Bomb {
    type Msg = ();
    type Output = ();
    #[expect(clippy::panic, reason = "the test needs a component that panics")]
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        panic!("the component failed");
    }
}

#[test]
fn a_tree_whose_call_panicked_answers_poisoned() {
    let tree = NativeTree::new(Arc::new(Native::default()));
    let bomb = tree
        .with(|tree| tree.mount(|_cx| Bomb).id().to_bits())
        .unwrap();
    let caught =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tree.send::<Bomb>(bomb, ())));
    assert!(caught.is_err(), "the panic reached the caller");

    assert_eq!(tree.pump(), Err(NativeError::Poisoned));
    assert_eq!(tree.send::<Bomb>(bomb, ()), Err(NativeError::Poisoned));
    assert!(
        tree.read(|tree| tree.is_poisoned()).unwrap(),
        "reads still work"
    );
}

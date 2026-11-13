//! The Compono side of the comparison. `kotlin/kmp/src/commonMain` is the other side.

use compono::router::items::Items;
use compono::router::pages::Pages;
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono::{Commit, Component, Cx, Handle, Host, Phase, Tree};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Counts live bytes for the memory scenario.
struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to `System` and only adds a counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const ROUNDS: usize = 31;
const WARMUP_ROUNDS: usize = 20;

/// Stands in for a UI that hears about every change.
struct Ui(u64);

impl Host for Ui {
    fn commit(&mut self, commit: &Commit<'_>) {
        self.0 = self.0.wrapping_add(commit.dirty.len() as u64);
        black_box(self.0);
    }
}

fn tree() -> Tree {
    let mut tree = Tree::new(Ui(0));
    tree.set_phase(Phase::Focused);
    tree
}

fn measure(name: &str, iterations: usize, mut body: impl FnMut()) {
    for _ in 0..WARMUP_ROUNDS {
        for _ in 0..iterations {
            body();
        }
    }
    let mut samples = [0.0_f64; ROUNDS];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..iterations {
            body();
        }
        *sample = start.elapsed().as_nanos() as f64 / iterations as f64;
    }
    report(name, &mut samples);
}

/// For a cycle that runs `per_cycle` operations.
fn measure_per(name: &str, cycles: usize, per_cycle: usize, mut body: impl FnMut()) {
    let scale = per_cycle as f64;
    for _ in 0..WARMUP_ROUNDS {
        for _ in 0..cycles {
            body();
        }
    }
    let mut samples = [0.0_f64; ROUNDS];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..cycles {
            body();
        }
        *sample = start.elapsed().as_nanos() as f64 / cycles as f64 / scale;
    }
    report(name, &mut samples);
}

fn report(name: &str, samples: &mut [f64; ROUNDS]) {
    samples.sort_by(f64::total_cmp);
    println!(
        "{name}\t{:.1}\t{:.1}\t{:.1}",
        samples[ROUNDS / 2],
        samples[ROUNDS / 10],
        samples[ROUNDS * 9 / 10]
    );
}

struct Leaf(u64);

impl Component for Leaf {
    type Msg = ();
    type Output = ();

    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        self.0 = self.0.wrapping_add(1);
    }
}

fn leaf<P: Component<Msg = M>, M>(cx: &mut Cx<'_, P>) -> Handle<Leaf> {
    cx.child(|_cx| Leaf(0), |_parent: &mut P, (), _cx| {})
}

enum StackMsg {
    Push(u32),
    Pop,
    PopAll,
}

struct StackRoot {
    stack: Stack<u32, Handle<Leaf>, Self>,
}

impl Component for StackRoot {
    type Msg = StackMsg;
    type Output = ();

    fn update(&mut self, msg: StackMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            StackMsg::Push(route) => self.stack.push(route, cx),
            StackMsg::Pop => {
                let _at_root = self.stack.pop(cx);
            }
            StackMsg::PopAll => {
                self.stack.pop_to(1, cx);
            }
        }
    }
}

fn stack_root(tree: &mut Tree) -> Handle<StackRoot> {
    tree.mount(|cx| StackRoot {
        stack: Stack::new(0, cx, |_route: &u32, cx| leaf(cx)),
    })
}

fn stack_push_pop(depth: u32) {
    let mut tree = tree();
    let root = stack_root(&mut tree);
    for route in 1..depth {
        tree.send(root, StackMsg::Push(route)).ok();
    }
    measure(&format!("stack_push_pop_{depth}"), 20_000, || {
        tree.send(root, StackMsg::Push(1_000_000)).ok();
        tree.send(root, StackMsg::Pop).ok();
    });
}

fn deep_stack() {
    const DEPTH: u32 = 1_000;
    measure_per("deep_stack_per_entry", 20, DEPTH as usize, || {
        let mut tree = tree();
        let root = stack_root(&mut tree);
        for route in 1..DEPTH {
            tree.send(root, StackMsg::Push(route)).ok();
        }
        tree.send(root, StackMsg::PopAll).ok();
    });
}

enum SlotMsg {
    Open,
    Close,
}

struct SlotRoot {
    slot: Slot<u32, Handle<Leaf>, Self>,
}

impl Component for SlotRoot {
    type Msg = SlotMsg;
    type Output = ();

    fn update(&mut self, msg: SlotMsg, cx: &mut Cx<'_, Self>) {
        match msg {
            SlotMsg::Open => {
                self.slot.show(1, cx);
            }
            SlotMsg::Close => {
                self.slot.close(cx);
            }
        }
    }
}

fn slot_open_close() {
    let mut tree = tree();
    let root = tree.mount(|cx| SlotRoot {
        slot: Slot::new(cx, |_route: &u32, cx| leaf(cx)),
    });
    measure("slot_open_close", 20_000, || {
        tree.send(root, SlotMsg::Open).ok();
        tree.send(root, SlotMsg::Close).ok();
    });
}

struct PagesRoot {
    pages: Option<Pages<u32, Handle<Leaf>, Self>>,
}

impl Component for PagesRoot {
    type Msg = usize;
    type Output = ();

    fn update(&mut self, index: usize, cx: &mut Cx<'_, Self>) {
        if let Some(pages) = &mut self.pages {
            pages.select(index, cx);
        }
    }
}

fn pages_select() {
    let mut tree = tree();
    let root = tree.mount(|cx| PagesRoot {
        pages: Pages::new((0..100).collect(), 0, 1, cx, |_route: &u32, cx| leaf(cx)).ok(),
    });
    let mut index = 0_usize;
    let mut step = 1_isize;
    measure("pages_select_next", 20_000, || {
        if index == 99 {
            step = -1;
        } else if index == 0 {
            step = 1;
        }
        index = index.wrapping_add_signed(step);
        tree.send(root, index).ok();
    });
}

struct ItemsRoot {
    items: Items<u32, Handle<Leaf>, Self>,
}

impl Component for ItemsRoot {
    type Msg = usize;
    type Output = ();

    fn update(&mut self, first: usize, cx: &mut Cx<'_, Self>) {
        let _unique = self.items.set_window(first..first + 30, cx);
    }
}

fn items_scroll() {
    let mut tree = tree();
    let root = tree.mount(|cx| ItemsRoot {
        items: Items::new((0..100_000).collect(), cx, |_route: &u32, cx| leaf(cx)),
    });
    tree.send(root, 0).ok();
    let mut first = 0_usize;
    let mut step = 1_isize;
    measure("items_scroll_one_row", 2_000, || {
        if first == 1_000 {
            step = -1;
        } else if first == 0 {
            step = 1;
        }
        first = first.wrapping_add_signed(step);
        tree.send(root, first).ok();
    });
}

fn update_child() {
    let mut tree = tree();
    let root = stack_root(&mut tree);
    let child = tree.get(root).map(|root| *root.stack.active());
    let Some(child) = child else { return };
    measure("update_child", 200_000, || {
        tree.send(child, ()).ok();
    });
}

fn create_destroy() {
    measure("create_destroy_tree", 20_000, || {
        let mut tree = tree();
        black_box(stack_root(&mut tree));
    });
}

fn memory() {
    const ENTRIES: u32 = 10_000;
    let mut tree = tree();
    let root = stack_root(&mut tree);
    let before = LIVE.load(Ordering::Relaxed);
    for route in 1..=ENTRIES {
        tree.send(root, StackMsg::Push(route)).ok();
    }
    let after = LIVE.load(Ordering::Relaxed);
    println!(
        "memory_bytes_per_entry\t{}",
        (after - before) / ENTRIES as usize
    );
    drop(tree);
}

fn main() {
    stack_push_pop(1);
    stack_push_pop(10);
    stack_push_pop(100);
    deep_stack();
    slot_open_close();
    pages_select();
    items_scroll();
    update_child();
    create_destroy();
    memory();
    println!("# done");
}

//! Measures one push and pop through the real `Stack`, with payloads and allocations,
//! for a numbered and a named stack, and one hide and show of the component that
//! owns the stack.
#![expect(unsafe_code, reason = "a counting allocator needs it")]

use compono::router::stack::Stack;
use compono::{Commit, Component, Cx, Handle, Host, Phase, Tree};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct Counting;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

// SAFETY: every call forwards unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the caller upholds the contract of `alloc`.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds the contract of `dealloc`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

struct NoHost;
impl Host for NoHost {
    fn commit(&mut self, _commit: &Commit<'_>) {}
}

struct Screen {
    board: [u64; 16],
}
impl Component for Screen {
    type Msg = ();
    type Output = u64;
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.emit(self.board[0]);
    }
}

struct Root {
    stack: Stack<u32, Handle<Screen>, Root>,
    sum: u64,
}
enum RootMsg {
    Push,
}
impl Component for Root {
    type Msg = RootMsg;
    type Output = ();
    fn update(&mut self, RootMsg::Push: RootMsg, cx: &mut Cx<'_, Self>) {
        self.stack.push(self.stack.depth() as u32, cx);
    }
}

/// A named stack makes every new entry look up which router holds its name.
fn measure(depth: u32, named: bool) -> (u64, u64) {
    let mut tree = Tree::new(NoHost);
    let root = tree.mount(|cx: &mut Cx<'_, Root>| {
        let build = |cx: &mut Cx<'_, Root>| {
            Stack::new(0, cx, |route: &u32, cx: &mut Cx<'_, Root>| {
                let seed = u64::from(*route);
                cx.child(
                    move |_| Screen { board: [seed; 16] },
                    move |root: &mut Root, value, cx| {
                        root.sum = root.sum.wrapping_add(value ^ seed);
                        let _ = root.stack.pop(cx);
                    },
                )
            })
        };
        let stack = if named {
            cx.named_routers("screens", build)
        } else {
            build(cx)
        };
        Root { stack, sum: 0 }
    });
    tree.set_phase(Phase::Focused);
    for _ in 1..depth {
        let _ = tree.send(root, RootMsg::Push);
    }

    const ROUNDS: u64 = 1_000_000;
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let started = Instant::now();
    for _ in 0..ROUNDS {
        let _ = tree.send(root, RootMsg::Push);
        if let Some(active) = tree.get(root).map(|root| *root.stack.active()) {
            let _ = tree.send(active, ());
        }
    }
    let nanos = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;
    (nanos / ROUNDS, allocations / ROUNDS)
}

/// Hides and shows a component that owns a stack of `depth` entries, as a tab
/// does when another tab covers it.
fn cover(depth: u32) -> u64 {
    let mut tree = Tree::new(NoHost);
    let root = tree.mount(|cx: &mut Cx<'_, Root>| Root {
        stack: Stack::new(0, cx, |_route: &u32, cx: &mut Cx<'_, Root>| {
            cx.child(
                |_| Screen { board: [0; 16] },
                |_root: &mut Root, _value, _cx| {},
            )
        }),
        sum: 0,
    });
    tree.set_phase(Phase::Focused);
    for _ in 1..depth {
        let _ = tree.send(root, RootMsg::Push);
    }

    const ROUNDS: u64 = 10_000;
    let started = Instant::now();
    for _ in 0..ROUNDS {
        tree.set_phase(Phase::Hidden);
        tree.set_phase(Phase::Focused);
    }
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX) / ROUNDS
}

fn main() {
    for depth in [1, 10, 100] {
        let (nanos, allocations) = measure(depth, false);
        println!("depth {depth}: push+pop {nanos} ns, {allocations} allocations");
    }
    for depth in [1, 10, 100] {
        let (nanos, allocations) = measure(depth, true);
        println!("depth {depth}: push+pop on a named stack {nanos} ns, {allocations} allocations");
    }
    for depth in [1, 100, 10_000] {
        let nanos = cover(depth);
        println!("depth {depth}: hide+show of the stack's owner {nanos} ns");
    }
}

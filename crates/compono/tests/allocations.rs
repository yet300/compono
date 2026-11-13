//! The allocation budget, measured exactly so it can be enforced in CI.

#![cfg(feature = "std")]
#![expect(unsafe_code, reason = "a counting allocator needs it")]
#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

mod common;

use common::NullHost;
use compono::router::stack::Stack;
use compono::{Component, Cx, Handle, Phase, Tree};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

std::thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    /// Bytes allocated on this thread and not yet freed.
    static LIVE_BYTES: Cell<i64> = const { Cell::new(0) };
}

fn count_bytes(layout: Layout, sign: i64) {
    let size = i64::try_from(layout.size()).unwrap_or(i64::MAX);
    let _during_thread_exit = LIVE_BYTES.try_with(|bytes| bytes.set(bytes.get() + sign * size));
}

// SAFETY: every call forwards unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _during_thread_exit = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        count_bytes(layout, 1);
        // SAFETY: the caller upholds the contract of `alloc`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count_bytes(layout, -1);
        // SAFETY: the caller upholds the contract of `dealloc`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_in(operation: impl FnOnce()) -> u64 {
    let before = ALLOCATIONS.with(Cell::get);
    operation();
    ALLOCATIONS.with(Cell::get) - before
}

struct Screen {
    _payload: [u64; 8],
}

impl Component for Screen {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), cx: &mut Cx<'_, Self>) {
        cx.unchanged();
    }
}

struct Root {
    stack: Stack<u32, Handle<Screen>, Root>,
}

enum Nav {
    Push,
    Pop,
}

impl Component for Root {
    type Msg = Nav;
    type Output = ();
    fn update(&mut self, msg: Nav, cx: &mut Cx<'_, Self>) {
        match msg {
            Nav::Push => self.stack.push(1, cx),
            Nav::Pop => {
                let _at_root = self.stack.pop(cx);
            }
        }
    }
}

fn app() -> (Tree, Handle<Root>) {
    let mut tree = Tree::new(NullHost);
    tree.set_phase(Phase::Focused);
    let root = tree.mount(|cx| Root {
        stack: Stack::new(0, cx, |_route, cx| {
            cx.child(|_cx| Screen { _payload: [0; 8] }, |_root, (), _cx| {})
        }),
    });
    (tree, root)
}

fn cycle_cost(depth: usize) -> u64 {
    let (mut tree, root) = app();
    for _ in 1..depth {
        tree.send(root, Nav::Push).unwrap();
    }
    // The first cycle grows internal buffers once.
    tree.send(root, Nav::Push).unwrap();
    tree.send(root, Nav::Pop).unwrap();
    const CYCLES: u64 = 64;
    let total = allocations_in(|| {
        for _ in 0..CYCLES {
            tree.send(root, Nav::Push).unwrap();
            tree.send(root, Nav::Pop).unwrap();
        }
    });
    total / CYCLES
}

#[test]
fn navigation_stays_within_budget_at_every_depth() {
    let shallow = cycle_cost(1);
    assert!(shallow <= 3, "{shallow} allocations per push and pop");
    assert_eq!(cycle_cost(10), shallow);
    assert_eq!(cycle_cost(100), shallow);
}

#[test]
fn push_and_pop_leave_no_memory_behind() {
    let (mut tree, root) = app();
    for _ in 1..10 {
        tree.send(root, Nav::Push).unwrap();
    }
    // The first cycle grows internal buffers once.
    tree.send(root, Nav::Push).unwrap();
    tree.send(root, Nav::Pop).unwrap();
    let before = LIVE_BYTES.with(Cell::get);
    for _ in 0..10_000 {
        tree.send(root, Nav::Push).unwrap();
        tree.send(root, Nav::Pop).unwrap();
    }
    assert_eq!(LIVE_BYTES.with(Cell::get), before);
}

#[test]
fn a_message_that_changes_nothing_does_not_allocate() {
    let (mut tree, root) = app();
    let screen = *tree.get(root).unwrap().stack.active();
    tree.send(screen, ()).unwrap();
    assert_eq!(
        allocations_in(|| {
            for _ in 0..100 {
                tree.send(screen, ()).unwrap();
            }
        }),
        0
    );
}

#[test]
fn pumping_an_empty_queue_does_not_allocate() {
    let (mut tree, _root) = app();
    tree.pump().unwrap();
    assert_eq!(allocations_in(|| tree.pump().unwrap()), 0);
}

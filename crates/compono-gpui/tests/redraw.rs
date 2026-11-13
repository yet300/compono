//! A change redraws the view that shows the component, not its siblings.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::{Component, Cx, Handle, Tree};
use compono_gpui::{Changes, ComponentViews, GpuiHost, TreeView, update};
use gpui::prelude::*;
use gpui::{AnyElement, Context, Entity, StyleRefinement, TestAppContext, Window, div};
use std::cell::Cell;
use std::rc::Rc;

struct Number(u32);

/// A component with a child that has no view of its own.
struct Wrapper {
    inner: Handle<Number>,
}
impl Component for Wrapper {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

impl Component for Number {
    type Msg = u32;
    type Output = ();
    fn update(&mut self, msg: u32, _cx: &mut Cx<'_, Self>) {
        self.0 = msg;
    }
}

struct Pair {
    first: Handle<Wrapper>,
    second: Handle<Number>,
}
impl Component for Pair {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
}

struct Owner {
    tree: Tree,
    root: Handle<Pair>,
    changes: Changes,
    views: ComponentViews,
    first_draws: Rc<Cell<u32>>,
    second_draws: Rc<Cell<u32>>,
}

impl TreeView for Owner {
    fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }
    fn changes(&self) -> &Changes {
        &self.changes
    }
    fn views_mut(&mut self) -> &mut ComponentViews {
        &mut self.views
    }
}

fn draw_first(
    owner: &mut Owner,
    handle: Handle<Wrapper>,
    _cx: &mut Context<'_, Owner>,
) -> AnyElement {
    owner.first_draws.set(owner.first_draws.get() + 1);
    let inner = owner.tree.get(handle).unwrap().inner;
    div()
        .child(owner.tree.get(inner).unwrap().0.to_string())
        .into_any_element()
}

fn draw_second(
    owner: &mut Owner,
    handle: Handle<Number>,
    _cx: &mut Context<'_, Owner>,
) -> AnyElement {
    owner.second_draws.set(owner.second_draws.get() + 1);
    div()
        .child(owner.tree.get(handle).unwrap().0.to_string())
        .into_any_element()
}

fn full() -> StyleRefinement {
    StyleRefinement::default().size_full()
}

impl Render for Owner {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let pair = self.tree.get(self.root).unwrap();
        let (first, second) = (pair.first, pair.second);
        let mut views = std::mem::take(&mut self.views);
        let first = views.view(first, draw_first, cx).cached(full());
        let second = views.view(second, draw_second, cx).cached(full());
        self.views = views;
        div().size_full().child(first).child(second)
    }
}

fn open(cx: &mut TestAppContext) -> (Entity<Owner>, &mut gpui::VisualTestContext) {
    cx.add_window_view(|_window, _cx| {
        let host = GpuiHost::new();
        let changes = host.changes();
        let mut tree = Tree::new(host);
        let root = tree.mount(|cx| Pair {
            first: cx.child(
                |cx| Wrapper {
                    inner: cx.child(|_cx| Number(0), |_wrapper, (), _cx| {}),
                },
                |_pair, (), _cx| {},
            ),
            second: cx.child(|_cx| Number(0), |_pair, (), _cx| {}),
        });
        let _mounted = changes.take();
        Owner {
            tree,
            root,
            changes,
            views: ComponentViews::default(),
            first_draws: Rc::default(),
            second_draws: Rc::default(),
        }
    })
}

#[gpui::test]
fn changing_one_component_leaves_its_sibling_cached(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.run_until_parked();
    let draws = |cx: &gpui::VisualTestContext| {
        view.read_with(cx, |owner, _cx| {
            (owner.first_draws.get(), owner.second_draws.get())
        })
    };
    assert_eq!(draws(cx), (1, 1));

    view.update(cx, |owner, cx| {
        let first = owner.tree.get(owner.root).unwrap().first;
        let inner = owner.tree.get(first).unwrap().inner;
        update(owner, cx, |tree| tree.send(inner, 5).unwrap());
    });
    cx.run_until_parked();
    assert_eq!(
        draws(cx),
        (2, 1),
        "a component with no view redraws its nearest ancestor, and not the sibling",
    );
}

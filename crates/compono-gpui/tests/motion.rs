//! Router transitions drawn in a test window on the executor's clock.

#![allow(clippy::unwrap_used, reason = "tests assert by failing")]

use compono::router::pages::Pages;
use compono::router::panels::{Panel, PanelTypes, Panels, PanelsMode};
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono::{Component, Cx, Handle, Tree};
use compono_gpui::{GpuiHost, RouterView, set_motion};
use compono_motion::{Easing, Motion, Timing};
use gpui::prelude::*;
use gpui::{Bounds, Context, Entity, Pixels, TestAppContext, VisualTestContext, Window, div};
use std::time::Duration;

struct Types;

impl PanelTypes for Types {
    type Main = u8;
    type MainChild = u8;
    type Details = u8;
    type DetailsChild = u8;
    type Extra = u8;
    type ExtraChild = u8;
}

struct Root {
    panels: Panels<Types, Root>,
    stack: Stack<u8, u8, Root>,
    slot: Slot<u8, u8, Root>,
    pages: Pages<u8, u8, Root>,
}

type Change = Box<dyn FnOnce(&mut Root, &mut Cx<'_, Root>)>;

impl Component for Root {
    type Msg = Change;
    type Output = ();
    fn update(&mut self, change: Change, cx: &mut Cx<'_, Self>) {
        change(self, cx);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Router {
    Stack,
    Slot,
    Pages,
    Panels,
}

struct Owner {
    tree: Tree,
    root: Handle<Root>,
    router: Router,
}

fn entry(value: u8) -> gpui::AnyElement {
    div()
        .debug_selector(move || format!("entry-{value}"))
        .size_full()
        .into_any_element()
}

impl Render for Owner {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let root = self.tree.get(self.root).unwrap();
        let routers = div().debug_selector(|| "area".to_owned()).size_full();
        match self.router {
            Router::Stack => routers.child(root.stack.view(|value| entry(*value))),
            Router::Slot => routers.children(root.slot.view(|value| entry(*value))),
            Router::Pages => routers.children(root.pages.view(|value| entry(*value))),
            Router::Panels => routers.child(root.panels.view(|panel| match panel {
                Panel::Main(value) | Panel::Details(value) | Panel::Extra(value) => entry(*value),
            })),
        }
    }
}

fn open(cx: &mut TestAppContext, router: Router) -> (Entity<Owner>, &mut VisualTestContext) {
    // Every test here uses the same motion, so they may run at once.
    set_motion(Motion::cupertino().with_timing(Timing::Tween {
        millis: 100,
        easing: Easing::Linear,
    }));
    cx.add_window_view(move |_window, _cx| {
        let mut tree = Tree::new(GpuiHost::new());
        let root = tree.mount(|cx| Root {
            panels: Panels::new(
                5,
                PanelsMode::Single,
                cx,
                |value, _cx| *value,
                |value, _cx| *value,
                |value, _cx| *value,
            ),
            stack: Stack::new(1, cx, |value, _cx| *value),
            slot: Slot::new(cx, |value, _cx| *value),
            pages: Pages::new(vec![1, 2, 3], 0, 1, cx, |value, _cx| *value).unwrap(),
        });
        Owner { tree, root, router }
    })
}

fn change(
    view: &Entity<Owner>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Root, &mut Cx<'_, Root>) + 'static,
) {
    view.update(cx, |owner, cx| {
        owner.tree.send(owner.root, Box::new(f)).unwrap();
        cx.notify();
    });
    cx.run_until_parked();
}

/// Lets `millis` pass on the executor's clock and draws a frame.
fn wait(cx: &mut VisualTestContext, millis: u64) {
    cx.executor().advance_clock(Duration::from_millis(millis));
    cx.update(|window, _cx| window.refresh());
    cx.run_until_parked();
}

fn bounds(cx: &mut VisualTestContext, name: &'static str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(name)
}

fn left(cx: &mut VisualTestContext, name: &'static str) -> f32 {
    let area = bounds(cx, "area").unwrap();
    f32::from(bounds(cx, name).unwrap().origin.x - area.origin.x)
}

fn width(cx: &mut VisualTestContext) -> f32 {
    f32::from(bounds(cx, "area").unwrap().size.width)
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0,
        "{actual} is not {expected}"
    );
}

#[gpui::test]
fn a_push_slides_the_new_entry_in_and_a_pop_slides_the_revealed_one_in(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, Router::Stack);
    cx.run_until_parked();
    close(left(cx, "entry-1"), 0.0);

    change(&view, cx, |r, cx| r.stack.push(2, cx));
    wait(cx, 50);
    close(left(cx, "entry-2"), 0.5 * width(cx));
    // The covered entry is not drawn.
    assert!(bounds(cx, "entry-1").is_none());
    wait(cx, 100);
    close(left(cx, "entry-2"), 0.0);

    change(&view, cx, |r, cx| {
        let _ = r.stack.pop(cx);
    });
    wait(cx, 50);
    // The popped entry is gone at once; the revealed one comes from the left.
    assert!(bounds(cx, "entry-2").is_none());
    close(left(cx, "entry-1"), -0.3 * 0.5 * width(cx));
    wait(cx, 100);
    close(left(cx, "entry-1"), 0.0);
}

#[gpui::test]
fn pages_slide_by_their_order(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, Router::Pages);
    cx.run_until_parked();
    change(&view, cx, |r, cx| assert!(r.pages.select(1, cx)));
    wait(cx, 50);
    close(left(cx, "entry-2"), 0.5 * width(cx));
    wait(cx, 100);
    change(&view, cx, |r, cx| assert!(r.pages.select(0, cx)));
    wait(cx, 50);
    close(left(cx, "entry-1"), -0.5 * width(cx));
}

#[gpui::test]
fn a_dialog_is_drawn_while_it_fades_in(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, Router::Slot);
    cx.run_until_parked();
    assert!(bounds(cx, "entry-4").is_none());
    change(&view, cx, |r, cx| {
        let _ = r.slot.show(4, cx);
    });
    wait(cx, 50);
    // A fade does not move the entry.
    close(left(cx, "entry-4"), 0.0);
    change(&view, cx, |r, cx| {
        let _ = r.slot.close(cx);
    });
    assert!(bounds(cx, "entry-4").is_none());
}

#[gpui::test]
fn a_narrow_window_slides_the_details_panel_in(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, Router::Panels);
    cx.run_until_parked();
    change(&view, cx, |r, cx| r.panels.set_details(Some(6), cx));
    wait(cx, 50);
    close(left(cx, "entry-6"), 0.5 * width(cx));
    assert!(bounds(cx, "entry-5").is_none());
    wait(cx, 100);
    change(&view, cx, |r, cx| r.panels.set_details(None, cx));
    wait(cx, 50);
    close(left(cx, "entry-5"), -0.3 * 0.5 * width(cx));
    // Two panels fit: the details panel appears beside without moving.
    change(&view, cx, |r, cx| r.panels.set_mode(PanelsMode::Dual, cx));
    wait(cx, 100);
    change(&view, cx, |r, cx| r.panels.set_details(Some(7), cx));
    close(left(cx, "entry-5"), 0.0);
    assert!(left(cx, "entry-7") > 0.4 * width(cx));
}

#[gpui::test]
fn gpuis_reduce_motion_setting_fades_in_place(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = open(cx, Router::Stack);
    cx.run_until_parked();
    change(&view, cx, |r, cx| r.stack.push(2, cx));
    wait(cx, 50);
    // Reduced motion turns the slide into a fade: the new entry stays in place.
    close(left(cx, "entry-2"), 0.0);
}

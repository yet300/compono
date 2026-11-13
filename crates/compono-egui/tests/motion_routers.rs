//! `Slot`, `Pages` and `Panels` transitions drawn headless at fixed times.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

use compono::router::pages::Pages;
use compono::router::panels::{PanelTypes, Panels, PanelsMode};
use compono::router::slot::Slot;
use compono::{Component, Cx, Handle, Tree};
use compono_egui::{EguiHost, RouterView, set_motion};
use compono_motion::{Easing, Motion, Timing};
use egui::{Area, Color32, Context, Id, Order, Pos2, RawInput, Rect, Shape, Ui, Vec2};
use std::cell::Cell;

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
    slot: Slot<u8, u8, Root>,
    pages: Pages<u8, u8, Root>,
    panels: Panels<Types, Root>,
}

type Change = Box<dyn FnOnce(&mut Root, &mut Cx<'_, Root>)>;

impl Component for Root {
    type Msg = Change;
    type Output = ();
    fn update(&mut self, change: Change, cx: &mut Cx<'_, Self>) {
        change(self, cx);
    }
}

#[derive(Clone, Copy)]
enum Router {
    Slot,
    Pages,
    Panels,
}

/// A rectangle whose width names the entry: `20 + 10 × value`.
fn width(value: u8) -> f32 {
    20.0 + 10.0 * f32::from(value)
}

struct Fixture {
    ctx: Context,
    tree: Tree,
    root: Handle<Root>,
    /// Where the router's area starts, and its width.
    area: Cell<(f32, f32)>,
}

impl Fixture {
    fn new(mode: PanelsMode) -> Self {
        let ctx = Context::default();
        set_motion(
            &ctx,
            Motion::desktop().with_timing(Timing::Tween {
                millis: 100,
                easing: Easing::Linear,
            }),
        );
        let mut tree = Tree::new(EguiHost::new(ctx.clone()));
        let root = tree.mount(|cx| Root {
            slot: Slot::new(cx, |value, _cx| *value),
            pages: Pages::new(vec![1, 2, 3], 0, 1, cx, |value, _cx| *value).unwrap(),
            panels: Panels::new(
                1,
                mode,
                cx,
                |value, _cx| *value,
                |value, _cx| *value,
                |value, _cx| *value,
            ),
        });
        Self {
            ctx,
            tree,
            root,
            area: Cell::new((0.0, 0.0)),
        }
    }

    fn change(&mut self, f: impl FnOnce(&mut Root, &mut Cx<'_, Root>) + 'static) {
        self.tree.send(self.root, Box::new(f)).unwrap();
    }

    /// Draws `router` at `time` and returns each entry rectangle, bottom first,
    /// with its value, its left edge relative to the router and its opacity.
    fn frame(&self, time: f64, router: Router) -> Vec<(u8, f32, f32)> {
        let input = RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
            ..RawInput::default()
        };
        let root = self.tree.get(self.root).unwrap();
        let mut output = self.ctx.run_ui(input, |ui| {
            let rect = ui.available_rect_before_wrap();
            self.area.set((rect.min.x, rect.width()));
            let paint = |ui: &mut Ui, value: u8| {
                let rect = Rect::from_min_size(ui.max_rect().min, Vec2::new(width(value), 50.0));
                ui.painter().rect_filled(rect, 0.0, Color32::RED);
            };
            match router {
                // A dialog paints into a layer of its own, as `egui::Modal` does.
                // egui's own fade-in of a new area is off, to see the adapter's.
                Router::Slot => root.slot.view(ui, |ui, value| {
                    Area::new(Id::unique("dialog"))
                        .fade_in(false)
                        .order(Order::Foreground)
                        .fixed_pos(Pos2::new(100.0, 100.0))
                        .show(ui.ctx(), |ui| paint(ui, *value));
                }),
                Router::Pages => root.pages.view(ui, |ui, value| paint(ui, *value)),
                Router::Panels => root.panels.view(ui, |ui, panel| {
                    let value = match panel {
                        compono::router::panels::Panel::Main(value)
                        | compono::router::panels::Panel::Details(value)
                        | compono::router::panels::Panel::Extra(value) => *value,
                    };
                    paint(ui, value);
                }),
            }
        });
        output.textures_delta.clear();
        let (left, _width) = self.area.get();
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) => (1..=6)
                    .find(|&value| (rect.rect.width() - width(value)).abs() < 0.01)
                    .map(|value| {
                        (
                            value,
                            rect.rect.min.x - left,
                            f32::from(rect.fill.a()) / 255.0,
                        )
                    }),
                _ => None,
            })
            .collect()
    }

    fn width(&self) -> f32 {
        self.area.get().1
    }
}

fn values(frame: &[(u8, f32, f32)]) -> Vec<u8> {
    frame.iter().map(|entry| entry.0).collect()
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.6,
        "{actual} is not {expected}"
    );
}

#[test]
fn a_dialog_fades_in_and_out() {
    let mut f = Fixture::new(PanelsMode::Single);
    assert!(f.frame(0.0, Router::Slot).is_empty());
    f.change(|r, cx| {
        let _ = r.slot.show(2, cx);
    });
    let _ = f.frame(1.0, Router::Slot);
    let opening = f.frame(1.025, Router::Slot);
    assert_eq!(values(&opening), [2]);
    assert!((opening[0].2 - 0.25).abs() < 0.01, "{opening:?}");
    assert!((f.frame(1.2, Router::Slot)[0].2 - 1.0).abs() < 0.01);

    f.change(|r, cx| {
        let _ = r.slot.close(cx);
    });
    let _ = f.frame(2.0, Router::Slot);
    let closing = f.frame(2.025, Router::Slot);
    assert_eq!(values(&closing), [2]);
    assert!((closing[0].2 - 0.75).abs() < 0.01, "{closing:?}");
    assert!(f.frame(2.2, Router::Slot).is_empty());
}

#[test]
fn pages_slide_in_the_direction_of_their_order() {
    let mut f = Fixture::new(PanelsMode::Single);
    let _ = f.frame(0.0, Router::Pages);
    f.change(|r, cx| assert!(r.pages.select(1, cx)));
    let _ = f.frame(1.0, Router::Pages);
    let forward = f.frame(1.025, Router::Pages);
    let w = f.width();
    // The old page leaves to the left and the next one comes from the right.
    assert_eq!(values(&forward), [1, 2]);
    close(forward[0].1, -0.25 * w);
    close(forward[1].1, 0.75 * w);

    let _ = f.frame(1.2, Router::Pages);
    f.change(|r, cx| assert!(r.pages.select(0, cx)));
    let _ = f.frame(2.0, Router::Pages);
    let back = f.frame(2.025, Router::Pages);
    // Back to an earlier page: it comes from the left, the old one leaves right.
    assert_eq!(values(&back), [1, 2]);
    close(back[0].1, -0.75 * w);
    close(back[1].1, 0.25 * w);
}

#[test]
fn a_narrow_window_slides_the_details_panel_in_and_out() {
    let mut f = Fixture::new(PanelsMode::Single);
    let _ = f.frame(0.0, Router::Panels);
    f.change(|r, cx| r.panels.set_details(Some(2), cx));
    let _ = f.frame(1.0, Router::Panels);
    let opening = f.frame(1.05, Router::Panels);
    let w = f.width();
    assert_eq!(values(&opening), [1, 2]);
    close(opening[0].1, -0.3 * 0.5 * w);
    close(opening[1].1, 0.5 * w);

    let _ = f.frame(1.2, Router::Panels);
    f.change(|r, cx| r.panels.set_details(None, cx));
    let _ = f.frame(2.0, Router::Panels);
    let closing = f.frame(2.05, Router::Panels);
    assert_eq!(values(&closing), [1, 2]);
    close(closing[0].1, -0.3 * 0.5 * w);
    close(closing[1].1, 0.5 * w);
    assert_eq!(values(&f.frame(2.2, Router::Panels)), [1]);
}

#[test]
fn wide_panels_appear_beside_without_moving() {
    let mut f = Fixture::new(PanelsMode::Dual);
    let _ = f.frame(0.0, Router::Panels);
    f.change(|r, cx| r.panels.set_details(Some(2), cx));
    let frame = f.frame(1.0, Router::Panels);
    assert_eq!(values(&frame), [1, 2]);
    close(frame[0].1, 0.0);
    assert!(frame[1].1 > 0.4 * f.width());
    assert!(frame.iter().all(|entry| (entry.2 - 1.0).abs() < 0.01));
}

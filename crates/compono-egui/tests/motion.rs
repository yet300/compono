//! Stack transitions drawn headless at fixed times.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests assert by failing"
)]

use compono::router::stack::Stack;
use compono::{Component, Cx, Handle, Tree};
use compono_egui::{EguiHost, RouterView, StackView, set_motion, set_motion_preference};
use compono_motion::{Easing, Motion, MotionPreference, Timing};
use egui::{Color32, Context, Pos2, RawInput, Rect, Shape, Vec2};
use std::cell::Cell;

struct Root {
    stack: Stack<u8, u8, Root>,
}

enum Msg {
    Push(u8),
    Pop,
}

impl Component for Root {
    type Msg = Msg;
    type Output = ();
    fn update(&mut self, msg: Msg, cx: &mut Cx<'_, Self>) {
        match msg {
            Msg::Push(route) => self.stack.push(route, cx),
            Msg::Pop => {
                let _ = self.stack.pop(cx);
            }
        }
    }
}

/// A rectangle whose width names the entry: `20 + 10 × route`.
fn width(route: u8) -> f32 {
    20.0 + 10.0 * f32::from(route)
}

struct Fixture {
    ctx: Context,
    tree: Tree,
    root: Handle<Root>,
    /// The width of the area the stack draws in.
    area: Cell<f32>,
}

impl Fixture {
    fn new() -> Self {
        let ctx = Context::default();
        let mut tree = Tree::new(EguiHost::new(ctx.clone()));
        let root = tree.mount(|cx| Root {
            stack: Stack::new(1, cx, |route, _cx| *route),
        });
        Self {
            ctx,
            tree,
            root,
            area: Cell::new(0.0),
        }
    }

    /// Draws one frame at `time` and returns the entry rectangles, bottom first,
    /// with their route and fill.
    fn frame(&self, time: f64, motion: Option<Motion>) -> Vec<(u8, Rect, Color32)> {
        let input = RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
            ..RawInput::default()
        };
        let root = self.tree.get(self.root).expect("the root exists");
        let mut output = self.ctx.run_ui(input, |ui| {
            let content = |ui: &mut egui::Ui, route: &u8| {
                self.area.set(ui.max_rect().width());
                let rect = Rect::from_min_size(
                    ui.max_rect().min + Vec2::splat(10.0),
                    Vec2::new(width(*route), 50.0),
                );
                ui.painter().rect_filled(rect, 0.0, Color32::RED);
            };
            match motion {
                Some(motion) => StackView::new(&root.stack).motion(motion).show(ui, content),
                None => root.stack.view(ui, content),
            }
        });
        output.textures_delta.clear();
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) => (1..=4)
                    .find(|&route| {
                        (rect.rect.width() - width(route) * rect_scale(rect.rect)).abs() < 0.01
                    })
                    .map(|route| (route, rect.rect, rect.fill)),
                _ => None,
            })
            .collect()
    }
}

/// Entry rectangles are 50 high, so their height gives their scale.
fn rect_scale(rect: Rect) -> f32 {
    rect.height() / 50.0
}

fn alpha(fill: Color32) -> f32 {
    f32::from(fill.a()) / 255.0
}

#[test]
fn a_push_fades_with_the_desktop_preset_by_default() {
    let mut f = Fixture::new();
    assert_eq!(f.frame(0.0, None).len(), 1);
    f.tree.send(f.root, Msg::Push(2)).unwrap();

    let start = f.frame(1.0, None);
    assert_eq!(start.iter().map(|e| e.0).collect::<Vec<_>>(), [1, 2]);

    // Half of the 150 ms fade, on the Material standard easing.
    let eased = Easing::STANDARD.ease(0.5);
    let middle = f.frame(1.075, None);
    assert_eq!(middle.iter().map(|e| e.0).collect::<Vec<_>>(), [1, 2]);
    assert!(
        (alpha(middle[0].2) - (1.0 - eased)).abs() < 0.01,
        "{:?}",
        middle[0]
    );
    assert!((alpha(middle[1].2) - eased).abs() < 0.01, "{:?}", middle[1]);
    assert!(f.ctx.has_requested_repaint());

    let end = f.frame(1.2, None);
    assert_eq!(end, [(2, end[0].1, Color32::RED)]);
}

#[test]
fn a_pop_replays_the_last_frame_of_the_destroyed_entry() {
    let mut f = Fixture::new();
    let slide = Motion::cupertino().with_timing(Timing::Tween {
        millis: 100,
        easing: Easing::Linear,
    });
    f.tree.send(f.root, Msg::Push(2)).unwrap();
    let rest = f.frame(0.0, Some(slide));
    let at_rest = rest[0].1.min.x;
    let _ = f.frame(1.0, Some(slide));

    f.tree.send(f.root, Msg::Pop).unwrap();
    let _ = f.frame(2.0, Some(slide));
    let middle = f.frame(2.05, Some(slide));
    let area = f.area.get();
    // The revealed entry is drawn first, moving in from 30 % to the left; the
    // destroyed one is drawn above it, halfway out to the right.
    assert_eq!(middle.iter().map(|e| e.0).collect::<Vec<_>>(), [1, 2]);
    assert!(
        (middle[0].1.min.x - (at_rest - 0.3 * 0.5 * area)).abs() < 0.5,
        "{middle:?}"
    );
    assert!(
        (middle[1].1.min.x - (at_rest + 0.5 * area)).abs() < 0.5,
        "{middle:?}"
    );

    let end = f.frame(2.1, Some(slide));
    assert_eq!(end.iter().map(|e| e.0).collect::<Vec<_>>(), [1]);
    assert!((end[0].1.min.x - at_rest).abs() < 0.01);
}

#[test]
fn the_context_motion_applies_to_stacks_without_their_own() {
    let mut f = Fixture::new();
    set_motion(&f.ctx, Motion::none());
    let _ = f.frame(0.0, None);
    f.tree.send(f.root, Msg::Push(2)).unwrap();
    let frame = f.frame(1.0, None);
    assert_eq!(frame.iter().map(|e| e.0).collect::<Vec<_>>(), [2]);
}

#[test]
fn reduced_motion_fades_in_place_and_no_motion_switches_at_once() {
    let mut f = Fixture::new();
    let slide = Motion::cupertino().with_timing(Timing::Tween {
        millis: 100,
        easing: Easing::Linear,
    });
    set_motion_preference(&f.ctx, MotionPreference::Reduced);
    let at_rest = f.frame(0.0, Some(slide))[0].1.min.x;
    f.tree.send(f.root, Msg::Push(2)).unwrap();
    let _ = f.frame(1.0, Some(slide));
    // A 150 ms fade on the standard easing, with nothing moving.
    let middle = f.frame(1.075, Some(slide));
    assert_eq!(middle.iter().map(|e| e.0).collect::<Vec<_>>(), [1, 2]);
    assert!(
        middle.iter().all(|e| (e.1.min.x - at_rest).abs() < 0.01),
        "{middle:?}"
    );
    assert!((alpha(middle[1].2) - Easing::STANDARD.ease(0.5)).abs() < 0.01);

    set_motion_preference(&f.ctx, MotionPreference::Off);
    let _ = f.frame(2.0, Some(slide));
    f.tree.send(f.root, Msg::Push(3)).unwrap();
    let frame = f.frame(3.0, Some(slide));
    assert_eq!(frame.iter().map(|e| e.0).collect::<Vec<_>>(), [3]);
}

//! `Animated` drawn into a recording renderer at fixed times.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "tests assert by failing"
)]

use super::{Animated, Rule};
use compono_motion::{Easing, Motion, Timing};
use iced::advanced::clipboard;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::Tree;
use iced::advanced::{Shell, Widget};
use iced::time::{Duration, Instant};
use iced::{
    Background, Color, Element, Event, Length, Rectangle, Size, Theme, Transformation, mouse,
    window,
};

/// What the renderer was asked to draw.
#[derive(Debug, Clone, PartialEq)]
enum Op {
    Layer,
    EndLayer,
    Transform(Transformation),
    EndTransform,
    Quad(Rectangle, Color),
}

#[derive(Default)]
struct Recorder(Vec<Op>);

impl iced::advanced::Renderer for Recorder {
    fn start_layer(&mut self, _bounds: Rectangle) {
        self.0.push(Op::Layer);
    }
    fn end_layer(&mut self) {
        self.0.push(Op::EndLayer);
    }
    fn start_transformation(&mut self, transformation: Transformation) {
        self.0.push(Op::Transform(transformation));
    }
    fn end_transformation(&mut self) {
        self.0.push(Op::EndTransform);
    }
    fn fill_quad(&mut self, quad: Quad, background: impl Into<Background>) {
        let color = match background.into() {
            Background::Color(color) => color,
            Background::Gradient(_) => Color::TRANSPARENT,
        };
        self.0.push(Op::Quad(quad.bounds, color));
    }
    fn reset(&mut self, _new_bounds: Rectangle) {}
    fn allocate_image(
        &mut self,
        _handle: &iced::advanced::image::Handle,
        _callback: impl FnOnce(Result<iced::advanced::image::Allocation, iced::advanced::image::Error>)
        + Send
        + 'static,
    ) {
    }
}

/// An entry: a red block filling the router.
struct Block;

impl<Message> Widget<Message, Theme, Recorder> for Block {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Recorder,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }
    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Recorder,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        iced::advanced::Renderer::fill_quad(
            renderer,
            Quad {
                bounds: layout.bounds(),
                ..Quad::default()
            },
            Color::from_rgb(1.0, 0.0, 0.0),
        );
    }
}

fn linear(motion: Motion) -> Motion {
    motion.with_timing(Timing::Tween {
        millis: 100,
        easing: Easing::Linear,
    })
}

fn element(rule: Rule, active: u64, motion: Motion) -> Element<'static, (), Theme, Recorder> {
    Animated::new(Element::new(Block), rule, active, motion).into()
}

const AREA: Size = Size::new(400.0, 300.0);

/// One router drawn over several frames.
struct Harness {
    element: Element<'static, (), Theme, Recorder>,
    tree: Tree,
    node: layout::Node,
    start: Instant,
}

impl Harness {
    fn new(element: Element<'static, (), Theme, Recorder>) -> Self {
        let mut element = element;
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &Recorder::default(),
            &layout::Limits::new(Size::ZERO, AREA),
        );
        Self {
            element,
            tree,
            node,
            start: Instant::now(),
        }
    }

    /// Rebuilds the view, as iced does after a message.
    fn view(&mut self, element: Element<'static, (), Theme, Recorder>) {
        self.element = element;
        self.tree.diff(&self.element);
        self.node = self.element.as_widget_mut().layout(
            &mut self.tree,
            &Recorder::default(),
            &layout::Limits::new(Size::ZERO, AREA),
        );
    }

    /// Sends the frame at `secs` and returns whether another frame was asked for.
    fn redraw(&mut self, secs: f64) -> bool {
        let at = self.start + Duration::from_secs_f64(secs);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        self.element.as_widget_mut().update(
            &mut self.tree,
            &Event::Window(window::Event::RedrawRequested(at)),
            Layout::new(&self.node),
            mouse::Cursor::Unavailable,
            &Recorder::default(),
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(AREA),
        );
        shell.redraw_request() != window::RedrawRequest::Wait
    }

    fn draw(&self) -> Vec<Op> {
        let mut renderer = Recorder::default();
        self.element.as_widget().draw(
            &self.tree,
            &mut renderer,
            &Theme::Light,
            &renderer::Style::default(),
            Layout::new(&self.node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(AREA),
        );
        renderer.0
    }
}

fn translation(ops: &[Op]) -> Option<(f32, f32)> {
    ops.iter().find_map(|op| match op {
        Op::Transform(t) => Some((t.translation().x, t.translation().y)),
        _ => None,
    })
}

/// The colours drawn over the entry, after it.
fn covers(ops: &[Op]) -> Vec<Color> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Quad(_, color) if *color != Color::from_rgb(1.0, 0.0, 0.0) => Some(*color),
            _ => None,
        })
        .collect()
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.5,
        "{actual} is not {expected}"
    );
}

#[test]
fn a_push_moves_the_new_entry_in_and_a_pop_moves_the_revealed_one_in() {
    let slide = linear(Motion::cupertino());
    let mut h = Harness::new(element(Rule::Stack(vec![0]), 0, slide));
    assert!(!h.redraw(0.0));
    assert_eq!(
        h.draw(),
        [Op::Quad(
            Rectangle::with_size(AREA),
            Color::from_rgb(1.0, 0.0, 0.0)
        )]
    );

    h.view(element(Rule::Stack(vec![0, 1]), 1, slide));
    assert!(h.redraw(1.0));
    assert!(h.redraw(1.05));
    // Halfway: the new entry is half a width to the right, and nothing else is drawn.
    let (x, y) = translation(&h.draw()).expect("the entry moves");
    close(x, 0.5 * AREA.width);
    close(y, 0.0);
    assert!(covers(&h.draw()).is_empty());
    assert!(!h.redraw(1.1));
    assert_eq!(translation(&h.draw()), None);

    h.view(element(Rule::Stack(vec![0]), 0, slide));
    assert!(h.redraw(2.0));
    assert!(h.redraw(2.05));
    // The revealed entry comes from 30 % to the left under a fading shade; the
    // popped one is gone.
    let ops = h.draw();
    close(
        translation(&ops).expect("the entry moves").0,
        -0.3 * 0.5 * AREA.width,
    );
    let shade = covers(&ops);
    assert_eq!(shade.len(), 1);
    assert!((shade[0].a - 0.06).abs() < 1e-3, "{shade:?}");
}

#[test]
fn bring_to_front_moves_forward() {
    let slide = linear(Motion::cupertino());
    let mut h = Harness::new(element(Rule::Stack(vec![0, 1, 2]), 2, slide));
    let _ = h.redraw(0.0);
    h.view(element(Rule::Stack(vec![1, 2, 0]), 0, slide));
    let _ = h.redraw(1.0);
    let _ = h.redraw(1.05);
    close(
        translation(&h.draw()).expect("the entry moves").0,
        0.5 * AREA.width,
    );
}

#[test]
fn a_slot_fades_its_entry_in_when_it_opens() {
    let mut h = Harness::new(element(Rule::Slot, 4, linear(Motion::desktop())));
    assert!(h.redraw(0.0));
    assert!(h.redraw(0.025));
    // A quarter in: the background covers three quarters of the entry.
    let cover = covers(&h.draw());
    assert_eq!(cover.len(), 1);
    assert!((cover[0].a - 0.75).abs() < 1e-3, "{cover:?}");
    assert_eq!(
        Color { a: 1.0, ..cover[0] },
        Theme::Light.palette().background
    );
    assert!(!h.redraw(0.2));
    assert!(covers(&h.draw()).is_empty());
}

#[test]
fn pages_move_by_their_order() {
    let slide = linear(Motion::desktop());
    let pages = |index| element(Rule::Pages, index, super::page_motion(slide));
    let mut h = Harness::new(pages(1));
    let _ = h.redraw(0.0);
    h.view(pages(0));
    let _ = h.redraw(1.0);
    let _ = h.redraw(1.05);
    // An earlier page comes in from the left.
    close(
        translation(&h.draw()).expect("the page moves").0,
        -0.5 * AREA.width,
    );
}

#[test]
fn wide_panels_and_no_motion_do_not_move() {
    let mut h = Harness::new(element(
        Rule::Panels { single: false },
        1 << 2,
        linear(Motion::cupertino()),
    ));
    let _ = h.redraw(0.0);
    h.view(element(
        Rule::Panels { single: false },
        (1 << 2) | 1,
        linear(Motion::cupertino()),
    ));
    assert!(!h.redraw(1.0));
    assert_eq!(translation(&h.draw()), None);

    let mut h = Harness::new(element(Rule::Stack(vec![0]), 0, Motion::none()));
    let _ = h.redraw(0.0);
    h.view(element(Rule::Stack(vec![0, 1]), 1, Motion::none()));
    assert!(!h.redraw(1.0));
    assert_eq!(translation(&h.draw()), None);
}

#[test]
fn the_preference_applies_to_the_application_motion() {
    use compono_motion::{MotionPreference, Transition};
    super::set_motion_preference(MotionPreference::Reduced);
    let reduced = super::preferred(Motion::cupertino());
    super::set_motion_preference(MotionPreference::Off);
    let off = super::preferred(Motion::cupertino());
    super::set_motion_preference(MotionPreference::Full);
    let full = super::preferred(Motion::cupertino());
    assert!(matches!(reduced.transition, Transition::Fade));
    assert!(matches!(off.transition, Transition::None));
    assert!(matches!(full.transition, Transition::Slide { .. }));
}

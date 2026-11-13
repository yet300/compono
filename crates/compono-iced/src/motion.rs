//! Transitions between router entries, from `compono-motion`.
//!
//! [`Animated`] wraps the element of a router's active entry. It keeps a
//! `TransitionState` in its widget state, starts it when the router's keys change,
//! takes time from `RedrawRequested` events, and draws the active entry with the
//! style of its layer. An `Element` borrows the tree and cannot outlive it, so the
//! entry that leaves is not drawn: on a pop only the revealed entry moves in, and
//! on a push the new entry moves in over the background.

use compono_motion::{
    Direction, LayerStyle, Motion, MotionPreference, Size, StackChange, Transition, TransitionState,
};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay};
use iced::time::Instant;
use iced::{
    Color, Element, Event, Length, Rectangle, Theme, Transformation, Vector, mouse, window,
};
use std::sync::{Mutex, PoisonError};

/// The motion of routers that do not choose their own. `Motion::desktop()` until
/// [`set_motion`] changes it.
static MOTION: Mutex<Option<Motion>> = Mutex::new(None);

/// Sets the motion of every router the application draws that does not choose its
/// own.
///
/// A `Slot` always fades, `Pages` slide by page order, and `Panels` slide the
/// deepest panel in when only one fits; each takes its timing from this motion,
/// and none moves with `Motion::none()`.
pub fn set_motion(motion: Motion) {
    *MOTION.lock().unwrap_or_else(PoisonError::into_inner) = Some(motion);
}

/// What the user asked of motion. `Full` until [`set_motion_preference`] changes it.
static PREFERENCE: Mutex<MotionPreference> = Mutex::new(MotionPreference::Full);

/// Sets what the user asked of motion, which applies to every router.
///
/// iced does not read the system setting, so the application passes it in.
pub fn set_motion_preference(preference: MotionPreference) {
    *PREFERENCE.lock().unwrap_or_else(PoisonError::into_inner) = preference;
}

/// `motion` changed for the preference set with [`set_motion_preference`].
pub(crate) fn preferred(motion: Motion) -> Motion {
    let preference = *PREFERENCE.lock().unwrap_or_else(PoisonError::into_inner);
    motion.for_preference(preference)
}

/// The motion set with [`set_motion`].
pub(crate) fn app_motion() -> Motion {
    MOTION
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .unwrap_or_else(Motion::desktop)
}

/// `motion` with `transition`, unless `motion` does not move at all.
fn with_transition(motion: Motion, transition: Transition) -> Motion {
    if matches!(motion.transition, Transition::None) {
        motion
    } else {
        motion.with_transition(transition)
    }
}

/// The motion of a `Slot`: a fade, with `motion`'s timing.
pub(crate) fn slot_motion(motion: Motion) -> Motion {
    with_transition(motion, Transition::Fade)
}

/// The motion of `Pages`: a full-width slide by page order, with `motion`'s timing.
pub(crate) fn page_motion(motion: Motion) -> Motion {
    with_transition(
        motion,
        Transition::Slide {
            parallax: 1.0,
            dim: 0.0,
        },
    )
}

/// The motion of `Panels` when one fits: the deeper panel slides over.
pub(crate) fn panel_motion(motion: Motion) -> Motion {
    with_transition(
        motion,
        Transition::Slide {
            parallax: 0.3,
            dim: 0.0,
        },
    )
}

/// The key of the panel on screen when one fits: its visit, then its depth.
pub(crate) fn panel_key(
    keys: compono::router::panels::PanelKeys,
    shown: compono::router::panels::ShownPanels,
) -> u64 {
    let (depth, visit) = if shown.extra {
        (2, keys.extra)
    } else if shown.details {
        (1, keys.details)
    } else {
        (0, keys.main)
    };
    visit.unwrap_or_default().wrapping_shl(2) | depth
}

/// How a router's keys turn into a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Rule {
    /// A stack, with the key of every entry, oldest first.
    Stack(Vec<u64>),
    /// A slot: its first frame is an opening.
    Slot,
    /// Pages, keyed by index: a later page is forward.
    Pages,
    /// Panels, keyed by `visit << 2 | depth`: a deeper panel is forward. Nothing
    /// moves unless one panel fits.
    Panels { single: bool },
}

/// The key of an empty slot.
const EMPTY: u64 = u64::MAX;

/// What [`Animated`] keeps between frames.
struct State {
    transition: TransitionState,
    keys: Vec<u64>,
    origin: Option<Instant>,
    now: f64,
    /// A change seen since the last frame, started again at the next frame's time.
    pending: Option<(StackChange, Motion)>,
}

/// The active entry of a router, moving with its layer's style.
pub(crate) struct Animated<'a, Message, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    rule: Rule,
    active: u64,
    motion: Motion,
}

impl<'a, Message, Renderer> Animated<'a, Message, Renderer> {
    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        rule: Rule,
        active: u64,
        motion: Motion,
    ) -> Self {
        Self {
            content: content.into(),
            rule,
            active,
            motion,
        }
    }
}

impl State {
    fn change(&self, rule: &Rule, active: u64) -> StackChange {
        let from = self.transition.active();
        if from == active {
            return StackChange::None;
        }
        let forward = StackChange::Forward { from, to: active };
        let backward = StackChange::Backward { from, to: active };
        match rule {
            Rule::Stack(keys) => {
                StackChange::between(&self.keys, from, keys.iter().copied(), active)
            }
            Rule::Slot => forward,
            Rule::Pages => {
                if active > from {
                    forward
                } else {
                    backward
                }
            }
            Rule::Panels { single: false } => StackChange::None,
            Rule::Panels { single: true } => {
                if active & 3 < from & 3 {
                    backward
                } else {
                    forward
                }
            }
        }
    }

    /// The style of the active entry now, at rest when nothing runs.
    fn style(&self, active: u64) -> LayerStyle {
        let mut transition = self.transition;
        transition
            .frame(self.now)
            .layers()
            .find(|layer| {
                layer.key == active
                    && matches!(
                        layer.direction,
                        Direction::EnterFront | Direction::EnterBack
                    )
            })
            .map_or(LayerStyle::REST, |layer| layer.style)
    }
}

/// Moves and scales about the centre of `bounds`.
fn transformation(style: LayerStyle, bounds: Rectangle) -> Transformation {
    let centre = bounds.center();
    let offset = Vector::new(
        style.offset_x * bounds.width,
        style.offset_y * bounds.height,
    );
    Transformation::translate(centre.x + offset.x, centre.y + offset.y)
        * Transformation::scale(style.scale)
        * Transformation::translate(-centre.x, -centre.y)
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for Animated<'_, Message, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> iced::Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> iced::Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    /// A new widget rests on its entry, except a slot's, which is opening.
    fn state(&self) -> tree::State {
        let mut state = State {
            transition: TransitionState::new(self.active),
            keys: Vec::new(),
            origin: None,
            now: 0.0,
            pending: None,
        };
        if let Rule::Stack(keys) = &self.rule {
            state.keys.clone_from(keys);
        }
        if self.rule == Rule::Slot {
            let open = StackChange::Forward {
                from: EMPTY,
                to: self.active,
            };
            state.transition = TransitionState::new(EMPTY);
            state.transition.start(open, self.motion, 0.0);
            state.pending = Some((open, self.motion));
        }
        tree::State::new(state)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(core::slice::from_ref(&self.content));
        let state = tree.state.downcast_mut::<State>();
        let change = state.change(&self.rule, self.active);
        if change != StackChange::None {
            state.transition.start(change, self.motion, state.now);
            state.pending = Some((change, self.motion));
        }
        if let Rule::Stack(keys) = &self.rule {
            state.keys.clone_from(keys);
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let Some(child) = tree.children.first_mut() else {
            return layout::Node::new(iced::Size::ZERO);
        };
        let node = self.content.as_widget_mut().layout(child, renderer, limits);
        layout::Node::with_children(node.size(), vec![node])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let (Some(child), Some(child_layout)) = (tree.children.first(), layout.children().next())
        else {
            return;
        };
        let draw = |renderer: &mut Renderer| {
            self.content.as_widget().draw(
                child,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        };
        let layer = tree.state.downcast_ref::<State>().style(self.active);
        if layer == LayerStyle::REST {
            draw(renderer);
            return;
        }
        let bounds = layout.bounds();
        let moved = transformation(layer, bounds);
        renderer.with_layer(bounds, |renderer| renderer.with_transformation(moved, draw));
        // No subtree opacity in iced: cover the entry with the background instead,
        // and with black for the scrim, in layers above it.
        let area = bounds * moved;
        let mut cover = |color: Color| {
            renderer.with_layer(bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: area,
                        ..Quad::default()
                    },
                    color,
                );
            });
        };
        if layer.opacity < 1.0 {
            cover(Color {
                a: 1.0 - layer.opacity.clamp(0.0, 1.0),
                ..theme.palette().background
            });
        }
        if layer.dim > 0.0 {
            cover(Color {
                a: layer.dim.clamp(0.0, 1.0),
                ..Color::BLACK
            });
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(at)) = event {
            let state = tree.state.downcast_mut::<State>();
            let origin = *state.origin.get_or_insert(*at);
            state.now = at.saturating_duration_since(origin).as_secs_f64();
            if let Some((change, motion)) = state.pending.take() {
                state.transition.start(change, motion, state.now);
            }
            let bounds = layout.bounds();
            state
                .transition
                .set_size(Size::new(bounds.width, bounds.height));
            let _ = state.transition.frame(state.now);
            if state.transition.is_running() {
                shell.request_redraw();
            }
        }
        if let (Some(child), Some(child_layout)) =
            (tree.children.first_mut(), layout.children().next())
        {
            self.content.as_widget_mut().update(
                child,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        match (tree.children.first(), layout.children().next()) {
            (Some(child), Some(child_layout)) => self.content.as_widget().mouse_interaction(
                child,
                child_layout,
                cursor,
                viewport,
                renderer,
            ),
            _ => mouse::Interaction::None,
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let (Some(child), Some(child_layout)) =
            (tree.children.first_mut(), layout.children().next())
        {
            self.content
                .as_widget_mut()
                .operate(child, child_layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = tree.children.first_mut()?;
        let child_layout = layout.children().next()?;
        self.content
            .as_widget_mut()
            .overlay(child, child_layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a, Renderer> From<Animated<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(animated: Animated<'a, Message, Renderer>) -> Self {
        Element::new(animated)
    }
}

#[cfg(test)]
#[path = "motion_tests.rs"]
mod tests;

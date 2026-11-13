//! Transitions between router entries, from `compono-motion`.
//!
//! [`Animated`] wraps the element of a router's active entry. It keeps a
//! `TransitionState` in GPUI's element state, starts it when the router's keys
//! change, takes time from the executor's clock, and draws the entry moved and
//! faded by its layer's style. GPUI cannot redraw a removed component, so the entry
//! that leaves is not drawn: on a pop only the revealed entry moves in, and on a
//! push the new entry moves in over the background. GPUI has no transform for a
//! subtree, so a layer's scale is not applied. GPUI's `App::reduce_motion` setting
//! applies `MotionPreference::Reduced`.

use compono::router::panels::{PanelKeys, ShownPanels};
use compono_motion::{
    Direction, LayerStyle, Motion, MotionPreference, Size, StackChange, Transition, TransitionState,
};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, ElementId, GlobalElementId, Hsla, InspectorElementId, LayoutId,
    Pixels, Window, div, px,
};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

/// The motion of routers that do not choose their own.
static MOTION: Mutex<Option<Motion>> = Mutex::new(None);

/// Sets the motion of every router the application draws.
///
/// It is `Motion::desktop()` until this is called. A `Slot` always fades, `Pages`
/// slide by page order, and `Panels` slide the deepest panel in when only one fits;
/// each takes its timing from this motion, and none moves with `Motion::none()`.
pub fn set_motion(motion: Motion) {
    *MOTION.lock().unwrap_or_else(PoisonError::into_inner) = Some(motion);
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
pub(crate) fn panel_key(keys: PanelKeys, shown: ShownPanels) -> u64 {
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
    /// Panels, keyed by [`panel_key`]: a deeper panel is forward. Nothing moves
    /// unless one panel fits.
    Panels { single: bool },
}

/// The key of an empty slot.
const EMPTY: u64 = u64::MAX;

/// What [`Animated`] keeps between frames.
struct State {
    transition: TransitionState,
    keys: Vec<u64>,
    origin: Instant,
    size: Size,
}

impl State {
    /// A new router rests on its entry, except a slot's, which is opening.
    fn new(rule: &Rule, active: u64, motion: Motion, now: Instant) -> Self {
        let mut state = Self {
            transition: TransitionState::new(active),
            keys: Vec::new(),
            origin: now,
            size: Size::default(),
        };
        if let Rule::Stack(keys) = rule {
            state.keys.clone_from(keys);
        }
        if *rule == Rule::Slot {
            let open = StackChange::Forward {
                from: EMPTY,
                to: active,
            };
            state.transition = TransitionState::new(EMPTY);
            state.transition.start(open, motion, 0.0);
        }
        state
    }

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
            Rule::Pages if active > from => forward,
            Rule::Pages => backward,
            Rule::Panels { single: false } => StackChange::None,
            Rule::Panels { single: true } if active & 3 < from & 3 => backward,
            Rule::Panels { single: true } => forward,
        }
    }
}

/// The active entry of a router, moving with its layer's style.
pub(crate) struct Animated {
    id: ElementId,
    child: Option<AnyElement>,
    rule: Rule,
    active: u64,
    motion: Motion,
}

impl Animated {
    /// Wraps `child`. `owner` tells routers apart: the address of the router.
    pub(crate) fn new<R>(
        kind: &'static str,
        owner: &R,
        child: AnyElement,
        rule: Rule,
        active: u64,
        motion: Motion,
    ) -> Self {
        let address = core::ptr::from_ref(owner) as usize;
        Self {
            id: ElementId::named_usize(kind, address),
            child: Some(child),
            rule,
            active,
            motion,
        }
    }
}

impl IntoElement for Animated {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Animated {
    type RequestLayoutState = Option<AnyElement>;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let Some(child) = self.child.take() else {
            return (div().into_any_element().request_layout(window, cx), None);
        };
        let Some(id) = id else {
            let mut child = child;
            return (child.request_layout(window, cx), Some(child));
        };
        let now = cx.background_executor().now();
        // GPUI's own setting, `App::reduce_motion`, asks for reduced motion.
        let motion = if cx.reduce_motion() {
            self.motion.for_preference(MotionPreference::Reduced)
        } else {
            self.motion
        };
        let (rule, active) = (&self.rule, self.active);
        let style = window.with_element_state(id, |state: Option<State>, _window| {
            let mut state = state.unwrap_or_else(|| State::new(rule, active, motion, now));
            let secs = now.saturating_duration_since(state.origin).as_secs_f64();
            let change = state.change(rule, active);
            state.transition.start(change, motion, secs);
            if let Rule::Stack(keys) = rule {
                state.keys.clone_from(keys);
            }
            state.transition.set_size(state.size);
            let frame = state.transition.frame(secs);
            let style = frame
                .layers()
                .find(|layer| {
                    layer.key == active
                        && matches!(
                            layer.direction,
                            Direction::EnterFront | Direction::EnterBack
                        )
                })
                .map_or(LayerStyle::REST, |layer| layer.style);
            ((style, state.transition.is_running(), state.size), state)
        });
        let (style, running, size) = style;
        if running {
            window.request_animation_frame();
        }
        let mut wrapped = if style == LayerStyle::REST {
            child
        } else {
            moved(child, style, size)
        };
        (wrapped.request_layout(window, cx), Some(wrapped))
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(id) = id {
            let now = cx.background_executor().now();
            let (rule, active, motion) = (&self.rule, self.active, self.motion);
            window.with_element_state(id, |state: Option<State>, _window| {
                let mut state = state.unwrap_or_else(|| State::new(rule, active, motion, now));
                state.size = Size::new(f32::from(bounds.size.width), f32::from(bounds.size.height));
                ((), state)
            });
        }
        if let Some(child) = child {
            child.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(child) = child {
            child.paint(window, cx);
        }
    }
}

/// `child` moved by the offsets of `style` in a router of `size`, faded, and
/// under its black scrim.
fn moved(child: AnyElement, style: LayerStyle, size: Size) -> AnyElement {
    let layer = div()
        .size_full()
        .relative()
        .left(px(style.offset_x * size.width))
        .top(px(style.offset_y * size.height))
        .opacity(style.opacity.clamp(0.0, 1.0))
        .child(child);
    let layer = if style.dim > 0.0 {
        layer.child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(Hsla::black().opacity(style.dim.clamp(0.0, 1.0))),
        )
    } else {
        layer
    };
    layer.into_any_element()
}

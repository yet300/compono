//! Transitions between router entries, from `compono-motion`.
//!
//! Each frame the adapter compares a router's keys with those of the last frame,
//! starts a `TransitionState` on a change, and applies the `LayerStyle` of each
//! layer to the shapes that layer painted, in every egui layer it painted into. The
//! active entry is drawn live. The other one is replayed from the shapes the
//! adapter copied on the last frame that drew it live, because a removed entry's
//! component no longer exists. A replayed entry takes no input.

mod layers;

use compono::Component;
use compono::router::pages::Pages;
use compono::router::panels::{Panel, PanelTypes, Panels, PanelsMode};
use compono::router::slot::Slot;
use compono::router::stack::Stack;
use compono_motion::{Motion, MotionPreference, StackChange, Transition};
use core::fmt;
use core::hash::Hash;
use egui::{Context, Id, Ui};
use layers::Layers;

/// Sets the motion of every router drawn in `ctx` that does not choose its own.
///
/// It is `Motion::desktop()` until this is called. A `Slot` always fades, `Pages`
/// slide by page order, and `Panels` slide the deepest panel in when only one fits;
/// each takes its timing from this motion, and none moves with `Motion::none()`.
pub fn set_motion(ctx: &Context, motion: Motion) {
    ctx.data_mut(|data| data.insert_temp(motion_id(), motion));
}

/// Sets what the user asked of motion, which applies to every router in `ctx`.
///
/// egui does not read the system setting, so the application passes it in. It is
/// `MotionPreference::Full` until this is called.
pub fn set_motion_preference(ctx: &Context, preference: MotionPreference) {
    ctx.data_mut(|data| data.insert_temp(preference_id(), preference));
}

fn motion_id() -> Id {
    Id::unique("compono-motion")
}

fn preference_id() -> Id {
    Id::unique("compono-motion-preference")
}

/// `motion` changed for the preference set on `ctx`.
fn preferred(ctx: &Context, motion: Motion) -> Motion {
    let preference = ctx
        .data(|data| data.get_temp(preference_id()))
        .unwrap_or_default();
    motion.for_preference(preference)
}

/// The motion of `ctx`.
fn context_motion(ctx: &Context) -> Motion {
    ctx.data(|data| data.get_temp(motion_id()))
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

/// Draws a [`Stack`] and animates changes of its active entry.
///
/// ```
/// # use compono::router::stack::Stack;
/// # use compono::{Component, Cx, Tree};
/// use compono_egui::StackView;
/// use compono_motion::Motion;
/// # struct Root { stack: Stack<u8, u8, Root> }
/// # impl Component for Root {
/// #     type Msg = ();
/// #     type Output = ();
/// #     fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {}
/// # }
/// # let ctx = egui::Context::default();
/// # let mut tree = Tree::new(compono_egui::EguiHost::new(ctx.clone()));
/// # let root = tree.mount(|cx| Root { stack: Stack::new(1, cx, |route, _cx| *route) });
/// # let root = tree.get(root).unwrap();
/// # let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| {
/// StackView::new(&root.stack)
///     .motion(Motion::material())
///     .show(ui, |ui, child| {
///         ui.label(format!("Screen {child}"));
///     });
/// # });
/// # frame.textures_delta.clear();
/// ```
pub struct StackView<'a, R, H, P> {
    stack: &'a Stack<R, H, P>,
    motion: Option<Motion>,
}

impl<R, H, P> fmt::Debug for StackView<'_, R, H, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StackView")
            .field("motion", &self.motion)
            .finish_non_exhaustive()
    }
}

impl<'a, R: 'static, H: 'static, P: Component> StackView<'a, R, H, P> {
    /// Creates a view of `stack` with the context's motion.
    #[must_use]
    pub const fn new(stack: &'a Stack<R, H, P>) -> Self {
        Self {
            stack,
            motion: None,
        }
    }

    /// Uses `motion` for this stack instead of the context's.
    #[must_use]
    pub const fn motion(mut self, motion: Motion) -> Self {
        self.motion = Some(motion);
        self
    }

    /// Draws the active entry, and during a transition the entry it replaced.
    pub fn show(self, ui: &mut Ui, mut content: impl FnMut(&mut Ui, &'a H)) {
        let stack = self.stack;
        let motion = self.motion.unwrap_or_else(|| context_motion(ui.ctx()));
        let motion = preferred(ui.ctx(), motion);
        let active = stack.active_key();
        let keys = || stack.entries().map(|(key, _route)| key);
        let mut layers = Layers::begin(ui, "stack");
        if let Some(previous) = layers.previous() {
            let change = StackChange::between(&layers.memory.keys, previous, keys(), active);
            layers.start(change, motion);
        }
        layers.memory.keys.clear();
        layers.memory.keys.extend(keys());
        layers.draw(ui, active, |ui| {
            ui.push_id(active, |ui| content(ui, stack.active()));
        });
    }
}

/// The key of an empty slot, which draws nothing.
const EMPTY: u64 = u64::MAX;

/// Draws a slot's entry, fading it in when it opens and out when it closes.
pub(crate) fn slot<'a, R: 'static, H: 'static, P: Component>(
    slot: &'a Slot<R, H, P>,
    ui: &mut Ui,
    mut content: impl FnMut(&mut Ui, &'a H),
) {
    let motion = with_transition(context_motion(ui.ctx()), Transition::Fade);
    let motion = preferred(ui.ctx(), motion);
    let active = slot.active_key().unwrap_or(EMPTY);
    let mut layers = Layers::begin(ui, "slot");
    if let Some(previous) = layers.previous().filter(|&previous| previous != active) {
        let change = if active == EMPTY {
            StackChange::Backward {
                from: previous,
                to: active,
            }
        } else {
            StackChange::Forward {
                from: previous,
                to: active,
            }
        };
        layers.start(change, motion);
    }
    layers.draw(ui, active, |ui| {
        if let Some(child) = slot.active() {
            content(ui, child);
        }
    });
}

/// Draws the selected page, sliding toward it by page order.
pub(crate) fn pages<'a, R, H: 'static, P: Component>(
    pages: &'a Pages<R, H, P>,
    ui: &mut Ui,
    mut content: impl FnMut(&mut Ui, &'a H),
) where
    R: Clone + Eq + Hash + 'static,
{
    let slide = Transition::Slide {
        parallax: 1.0,
        dim: 0.0,
    };
    let motion = preferred(ui.ctx(), with_transition(context_motion(ui.ctx()), slide));
    let active = pages.selected() as u64;
    let mut layers = Layers::begin(ui, "pages");
    if let Some(previous) = layers.previous().filter(|&previous| previous != active) {
        let change = if active > previous {
            StackChange::Forward {
                from: previous,
                to: active,
            }
        } else {
            StackChange::Backward {
                from: previous,
                to: active,
            }
        };
        layers.start(change, motion);
    }
    layers.draw(ui, active, |ui| {
        if let Some(child) = pages.active() {
            ui.push_id(pages.selected(), |ui| content(ui, child));
        }
    });
}

/// Draws the visible panels. When only one fits, the deepest panel slides in over
/// the one it covers and out again.
pub(crate) fn panels<'a, T: PanelTypes, P: Component>(
    panels: &'a Panels<T, P>,
    ui: &mut Ui,
    mut content: impl FnMut(&mut Ui, Panel<'a, T>),
) {
    let slide = Transition::Slide {
        parallax: 0.3,
        dim: 0.0,
    };
    let motion = preferred(ui.ctx(), with_transition(context_motion(ui.ctx()), slide));
    let keys = panels.keys();
    let shown = panels.shown();
    // The panel on screen when one fits: its depth and its visit key.
    let (depth, visit) = if shown.extra {
        (2, keys.extra)
    } else if shown.details {
        (1, keys.details)
    } else {
        (0, keys.main)
    };
    let active = visit.unwrap_or_default().wrapping_shl(2) | depth;
    let mut layers = Layers::begin(ui, "panels");
    if panels.mode() != PanelsMode::Single {
        layers.rest(active);
    } else if let Some(previous) = layers.previous().filter(|&previous| previous != active) {
        let change = if depth < previous & 3 {
            StackChange::Backward {
                from: previous,
                to: active,
            }
        } else {
            StackChange::Forward {
                from: previous,
                to: active,
            }
        };
        layers.start(change, motion);
    }
    layers.draw(ui, active, |ui| {
        ui.columns(panels.visible().count().max(1), |columns| {
            for (ui, panel) in columns.iter_mut().zip(panels.visible()) {
                content(ui, panel);
            }
        });
    });
}

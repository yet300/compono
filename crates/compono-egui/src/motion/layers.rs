//! Drawing the layers of one animated router in egui.

use compono_motion::{LayerStyle, Motion, Size, StackChange, TransitionState};
use egui::emath::TSTransform;
use egui::epaint::ClippedShape;
use egui::layers::{PaintList, ShapeIdx};
use egui::{Color32, Id, LayerId, Rect, Shape, Ui};

/// The shapes one entry painted into one egui layer.
#[derive(Clone)]
struct Painted {
    layer: LayerId,
    shapes: Vec<ClippedShape>,
}

/// What an animated router remembers between frames.
#[derive(Clone, Default)]
pub(crate) struct Memory {
    started: bool,
    state: TransitionState,
    /// The entry keys of the last frame, for a stack.
    pub(crate) keys: Vec<u64>,
    /// What the active entry painted on the last frame, untransformed.
    last: Vec<Painted>,
    /// What is replayed for the entry that is not active, during a transition.
    ghost: Vec<Painted>,
    /// Where each layer's paint list ended before the active entry was drawn.
    marks: Vec<(LayerId, usize)>,
}

/// One frame of an animated router.
pub(crate) struct Layers {
    id: Id,
    now: f64,
    rect: Rect,
    pub(crate) memory: Memory,
}

impl Layers {
    /// Takes the router's memory out of the context for this frame.
    pub(crate) fn begin(ui: &Ui, kind: &'static str) -> Self {
        let id = ui.scope_id().with(("compono-motion", kind));
        let memory = ui
            .ctx()
            .data_mut(|data| data.remove_temp::<Memory>(id))
            .unwrap_or_default();
        Self {
            id,
            now: ui.input(|input| input.time),
            rect: ui.available_rect_before_wrap(),
            memory,
        }
    }

    /// The active key of the last frame, or `None` on the router's first frame.
    pub(crate) const fn previous(&self) -> Option<u64> {
        if self.memory.started {
            Some(self.memory.state.active())
        } else {
            None
        }
    }

    /// Starts `change` with `motion`. The entry that was active is replayed.
    pub(crate) fn start(&mut self, change: StackChange, motion: Motion) {
        if change != StackChange::None {
            let m = &mut self.memory;
            core::mem::swap(&mut m.ghost, &mut m.last);
            m.state.start(change, motion, self.now);
        }
    }

    /// Shows `active` at rest, ending any transition.
    pub(crate) fn rest(&mut self, active: u64) {
        let m = &mut self.memory;
        m.started = true;
        m.state = TransitionState::new(active);
        m.ghost = Vec::new();
    }

    /// Draws the active entry live and the other one from its replayed shapes,
    /// bottom first, then puts the memory back.
    pub(crate) fn draw(mut self, ui: &mut Ui, active: u64, live: impl FnOnce(&mut Ui)) {
        let rect = self.rect;
        let m = &mut self.memory;
        if !m.started {
            m.started = true;
            m.state = TransitionState::new(active);
        }
        m.state.set_size(Size::new(rect.width(), rect.height()));
        let frame = m.state.frame(self.now);
        let mut live = Some(live);
        for layer in frame.layers() {
            if layer.key == active {
                mark(ui, &mut m.marks);
                if let Some(live) = live.take() {
                    live(ui);
                }
                capture(ui, &m.marks, &mut m.last, layer.style, rect);
            } else {
                ui.ctx().graphics_mut(|graphics| {
                    for painted in &m.ghost {
                        let list = graphics.entry(painted.layer);
                        for shape in &painted.shapes {
                            let mut shape = shape.clone();
                            apply(&mut shape, layer.style, rect);
                            list.add(shape.clip_rect, shape.shape);
                        }
                    }
                    dim(graphics.entry(ui.layer_id()), layer.style, rect);
                });
            }
        }
        if let Some(live) = live {
            // The active entry has no layer when nothing moves: draw it plainly.
            mark(ui, &mut m.marks);
            live(ui);
            capture(ui, &m.marks, &mut m.last, LayerStyle::REST, rect);
        }
        if m.state.is_running() {
            ui.ctx().request_repaint();
        } else {
            m.ghost = Vec::new();
        }
        let id = self.id;
        let memory = self.memory;
        ui.ctx().data_mut(|data| data.insert_temp(id, memory));
    }
}

/// The layers an entry may paint into: the router's own and every area, such as
/// a modal dialog the entry opens.
fn painted_layers(ui: &Ui) -> Vec<LayerId> {
    let mut layers: Vec<LayerId> = ui.ctx().memory(|memory| memory.layer_ids().collect());
    if !layers.contains(&ui.layer_id()) {
        layers.push(ui.layer_id());
    }
    layers
}

/// Records where each layer's paint list ends.
fn mark(ui: &Ui, marks: &mut Vec<(LayerId, usize)>) {
    marks.clear();
    let layers = painted_layers(ui);
    ui.ctx().graphics(|graphics| {
        marks.extend(layers.into_iter().map(|layer| {
            let end = graphics.get(layer).map_or(0, |list| list.next_idx().0);
            (layer, end)
        }));
    });
}

/// Copies what was painted since `marks` into `last`, then styles it in place.
fn capture(
    ui: &Ui,
    marks: &[(LayerId, usize)],
    last: &mut Vec<Painted>,
    style: LayerStyle,
    rect: Rect,
) {
    last.clear();
    let layers = painted_layers(ui);
    ui.ctx().graphics_mut(|graphics| {
        for layer in layers {
            let start = marks
                .iter()
                .find(|(marked, _end)| *marked == layer)
                .map_or(0, |(_layer, end)| *end);
            let Some(list) = graphics.get_mut(layer) else {
                continue;
            };
            let end = list.next_idx().0;
            if end <= start {
                continue;
            }
            last.push(Painted {
                layer,
                shapes: list
                    .all_entries()
                    .skip(start)
                    .take(end - start)
                    .cloned()
                    .collect(),
            });
            if style != LayerStyle::REST {
                for index in start..end {
                    list.mutate_shape(ShapeIdx(index), |shape| apply(shape, style, rect));
                }
            }
        }
        dim(graphics.entry(ui.layer_id()), style, rect);
    });
}

/// The transform of `style` in a router that fills `rect`: moved by its offsets
/// and scaled about the centre.
fn transform(style: LayerStyle, rect: Rect) -> TSTransform {
    let centre = rect.center().to_vec2();
    let offset = egui::vec2(
        style.offset_x * rect.width(),
        style.offset_y * rect.height(),
    );
    TSTransform::new(centre * (1.0 - style.scale) + offset, style.scale)
}

/// Moves, scales and fades one shape, and keeps it inside the router.
fn apply(shape: &mut ClippedShape, style: LayerStyle, rect: Rect) {
    shape.transform(transform(style, rect));
    if style.offset_x != 0.0 || style.offset_y != 0.0 || style.scale > 1.0 {
        shape.clip_rect = shape.clip_rect.intersect(rect);
    }
    if style.opacity < 1.0 {
        fade(&mut shape.shape, style.opacity.clamp(0.0, 1.0));
    }
}

fn fade(shape: &mut Shape, opacity: f32) {
    match shape {
        Shape::Text(text) => text.opacity_factor *= opacity,
        Shape::Vec(shapes) => {
            for shape in shapes {
                fade(shape, opacity);
            }
        }
        _ => egui::epaint::shape_transform::adjust_colors(shape, move |color| {
            if *color != Color32::PLACEHOLDER {
                *color = color.gamma_multiply(opacity);
            }
        }),
    }
}

/// Paints the black scrim of `style` over its entry.
fn dim(list: &mut PaintList, style: LayerStyle, rect: Rect) {
    if style.dim > 0.0 {
        let area = transform(style, rect) * rect;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=255"
        )]
        let alpha = (style.dim.clamp(0.0, 1.0) * 255.0) as u8;
        list.add(
            rect,
            Shape::rect_filled(area, 0.0, Color32::from_black_alpha(alpha)),
        );
    }
}

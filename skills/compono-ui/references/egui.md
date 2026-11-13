# egui and eframe

egui draws while it reads, so it cannot change the tree in the same moment.
Views read `&Tree` and queue messages in an `Outbox`; the app flushes the outbox
after the frame.

```rust,ignore
use compono::{Handle, Phase, Tree};
use compono_egui::{BeforeDraw, EguiHost, Outbox, RouterView, back_requested, window_phase};

pub struct App {
    tree: Tree,
    root: Handle<Root>,
    outbox: Outbox,
    before_draw: BeforeDraw,
    phase: Phase,
}

impl App {
    pub fn new(ctx: &egui::Context, saved: Option<&[u8]>) -> Self {
        let host = EguiHost::new(ctx.clone());
        let before_draw = host.before_draw();
        let mut tree = match saved {
            Some(bytes) => Tree::with_saved(host, bytes, SCHEMA).0,
            None => Tree::new(host),
        };
        tree.set_spawner(spawner);
        let root = tree.mount_persistent(|cx| Root::new(cx));
        Self { tree, root, outbox: Outbox::default(), before_draw, phase: Phase::Hidden }
    }

    /// Call from `eframe::App::ui`.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        let phase = window_phase(ui.ctx());
        let (tree, before_draw) = (&mut self.tree, &self.before_draw);
        before_draw.run(|| {
            if phase != self.phase {
                self.phase = phase;
                tree.set_phase(phase);
            }
            let _closed = tree.pump(); // work finished on other threads
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui::root(ui, &self.tree, self.root, &mut self.outbox);
        });
        if back_requested(ui.ctx()) {
            self.outbox.back();
        }
        self.outbox.flush(&mut self.tree);
    }
}
```

## Views

```rust,ignore
pub fn view(ui: &mut Ui, tree: &Tree, handle: Handle<Counters>, out: &mut Outbox) {
    let Some(counters) = tree.get(handle) else { return };
    ui.heading(format!("Stack depth {}", counters.stack.depth()));
    if ui.button("Back to first").clicked() {
        out.send(handle, CountersMsg::PopToRoot);
    }
    counters.stack.view(ui, |ui, child| counter::view(ui, tree, *child, out));
}
```

- `router.view(ui, |ui, child| ...)` for `Stack`, `Slot`, `Pages` and `Panels`.
- A `Stack` animates its changes with `compono_motion::Motion::desktop()`, a 150 ms
  fade. `compono_egui::set_motion(ctx, Motion::web())` changes the motion of every
  router in a context; `StackView::new(&stack).motion(Motion::material()).show(ui, |ui, child| ...)`
  changes one stack. A popped entry is replayed from its last frame and takes no input.
- With the same timing, a `Slot` fades its entry in and out, even one drawn in an
  `egui::Modal`; `Pages` slide by page order; `Panels` in `PanelsMode::Single`
  slide the deepest panel in and out. `Motion::none()` turns all of it off.
- `compono_egui::set_motion_preference(ctx, MotionPreference::Reduced)` passes the
  system's reduced-motion setting, which egui does not read: everything fades in
  place. In a browser, take it from `compono_web::prefers_reduced_motion()`.
- `ListView::new(&items, handle, Msg::Window).row_height(h).view(ui, out, |ui, out, index, child| ...)`.
- Give every `ScrollArea` an `id_salt`, so two lists on one screen do not clash.
- Draw only glyphs the font has. egui's default font has no emoji.
- `EguiHost` repaints when the tree changes, including from other threads.

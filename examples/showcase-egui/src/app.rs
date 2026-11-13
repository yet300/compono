use crate::ui;
use compono::{Handle, Phase, Spawner, Tree};
use compono_egui::{BeforeDraw, EguiHost, Outbox, back_requested, window_phase};
use egui::{Context, Ui};
use showcase::Services;
use showcase::feature::root::Root;

/// A component tree connected to an egui context.
#[derive(Debug)]
pub struct App {
    tree: Tree,
    root: Handle<Root>,
    outbox: Outbox,
    before_draw: BeforeDraw,
    phase: Phase,
}

impl App {
    /// Builds the tree, resuming `saved` from [`App::save`] when it is given.
    pub fn new(
        ctx: &Context,
        services: Services,
        spawner: impl Spawner + 'static,
        saved: Option<&[u8]>,
    ) -> Self {
        let host = EguiHost::new(ctx.clone());
        let before_draw = host.before_draw();
        let mut tree = showcase::saved::tree(host, saved);
        tree.set_spawner(spawner);
        let root = tree.mount_persistent(move |cx| Root::new(cx, services));
        Self {
            tree,
            root,
            outbox: Outbox::default(),
            before_draw,
            phase: Phase::Hidden,
        }
    }

    /// Encodes the tree for the next launch.
    #[must_use]
    pub fn save(&self) -> Option<Vec<u8>> {
        showcase::saved::save(&self.tree)
    }

    /// Returns the tree, for tests.
    pub const fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }

    /// Returns the tree, for tests.
    #[must_use]
    pub const fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Returns the root component.
    #[must_use]
    pub const fn root(&self) -> Handle<Root> {
        self.root
    }

    /// Draws one frame and applies what the user did during it.
    ///
    /// The tree's phase follows the window.
    pub fn frame(&mut self, ui: &mut Ui) {
        let phase = window_phase(ui.ctx());
        self.frame_in(ui, phase);
    }

    /// Draws one frame with the tree at `phase`, which the caller tracks.
    ///
    /// A browser hides a page without drawing it, so the web build tracks the
    /// page's phase itself.
    pub fn frame_in(&mut self, ui: &mut Ui, phase: Phase) {
        let (tree, before_draw) = (&mut self.tree, &self.before_draw);
        before_draw.run(|| {
            if phase != self.phase {
                self.phase = phase;
                tree.set_phase(phase);
            }
            // A poisoned queue only happens after a panic elsewhere, which aborts.
            let _closed = tree.pump();
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui::root(ui, &self.tree, self.root, &mut self.outbox);
        });
        if back_requested(ui.ctx()) {
            self.outbox.back();
        }
        self.outbox.flush(&mut self.tree);
    }

    /// Changes the tree's phase between frames.
    pub fn set_phase(&mut self, phase: Phase) {
        if phase != self.phase {
            self.phase = phase;
            self.tree.set_phase(phase);
        }
    }
}

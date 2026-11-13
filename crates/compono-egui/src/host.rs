use compono::{Commit, Host};
use egui::Context;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Wake, Waker};

/// A [`Host`] that repaints an egui context.
#[derive(Clone, Debug)]
pub struct EguiHost {
    ctx: Context,
    drawing_next: Arc<AtomicBool>,
}

/// Marks tree operations whose result the current frame is about to draw.
#[derive(Clone, Debug)]
pub struct BeforeDraw(Arc<AtomicBool>);

impl BeforeDraw {
    /// Runs `operation` without asking egui for another frame.
    pub fn run<T>(&self, operation: impl FnOnce() -> T) -> T {
        self.0.store(true, Ordering::Relaxed);
        let value = operation();
        self.0.store(false, Ordering::Relaxed);
        value
    }
}

impl EguiHost {
    /// Binds a tree's updates to `ctx`.
    #[must_use]
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx,
            drawing_next: Arc::default(),
        }
    }

    /// Returns the marker for operations performed before drawing a frame.
    #[must_use]
    pub fn before_draw(&self) -> BeforeDraw {
        BeforeDraw(Arc::clone(&self.drawing_next))
    }
}

struct Repaint(Context);

impl Wake for Repaint {
    fn wake(self: Arc<Self>) {
        self.0.request_repaint();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.request_repaint();
    }
}

impl Host for EguiHost {
    fn commit(&mut self, _commit: &Commit<'_>) {
        if !self.drawing_next.load(Ordering::Relaxed) {
            self.ctx.request_repaint();
        }
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::new(Repaint(self.ctx.clone())))
    }
}

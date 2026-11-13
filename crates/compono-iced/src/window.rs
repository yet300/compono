use compono::Phase;
use iced::{Subscription, Task, window};

/// A change in how present a window is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presence {
    /// The window that changed.
    pub window: window::Id,
    /// The phase the change implies.
    pub phase: Phase,
}

impl Presence {
    /// Asks whether an unfocused window is minimized, producing `Hidden` if so.
    ///
    /// iced reports no minimize event. The answer can only ever lower the
    /// phase, so a late reply cannot undo a newer focus event.
    pub fn check_minimized(self) -> Task<Phase> {
        if self.phase != Phase::Visible {
            return Task::none();
        }
        window::is_minimized(self.window).then(|minimized| {
            if minimized == Some(true) {
                Task::done(Phase::Hidden)
            } else {
                Task::none()
            }
        })
    }
}

/// Reports window focus changes, for [`Tree::set_phase`](compono::Tree::set_phase).
pub fn presence() -> Subscription<Presence> {
    window::events().filter_map(|(window, event)| {
        let phase = match event {
            window::Event::Focused => Phase::Focused,
            window::Event::Opened { .. } | window::Event::Unfocused => Phase::Visible,
            _ => return None,
        };
        Some(Presence { window, phase })
    })
}

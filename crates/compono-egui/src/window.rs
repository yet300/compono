use compono::Phase;
use egui::Context;

/// Returns how present the window is, for [`Tree::set_phase`](compono::Tree::set_phase).
#[must_use]
pub fn window_phase(ctx: &Context) -> Phase {
    ctx.input(|input| {
        if input.viewport().minimized == Some(true) {
            Phase::Hidden
        } else if input.focused {
            Phase::Focused
        } else {
            Phase::Visible
        }
    })
}

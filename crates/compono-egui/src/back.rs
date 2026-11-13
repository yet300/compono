use compono_desktop::{Convention, Key, Modifiers};
use egui::{Context, Event, PointerButton};

fn key(key: egui::Key) -> Key {
    match key {
        egui::Key::Escape => Key::Escape,
        egui::Key::ArrowLeft => Key::ArrowLeft,
        egui::Key::OpenBracket => Key::BracketLeft,
        egui::Key::BrowserBack => Key::BrowserBack,
        _ => Key::Other,
    }
}

const fn modifiers(modifiers: egui::Modifiers) -> Modifiers {
    Modifiers {
        ctrl: modifiers.ctrl,
        alt: modifiers.alt,
        shift: modifiers.shift,
        logo: modifiers.mac_cmd,
    }
}

/// Returns whether the user asked for Back during this frame.
///
/// Escape, a mouse Back button and the system's own shortcut count, as
/// [`Convention::CURRENT`] describes. Answer it with
/// [`Outbox::back`](crate::Outbox::back).
#[must_use]
pub fn back_requested(ctx: &Context) -> bool {
    ctx.input(|input| {
        input.events.iter().any(|event| match *event {
            Event::Key {
                key: pressed,
                pressed: true,
                repeat: false,
                modifiers: held,
                ..
            } => Convention::CURRENT.is_back(key(pressed), modifiers(held)),
            Event::PointerButton {
                button: PointerButton::Extra1,
                pressed: true,
                ..
            } => true,
            _ => false,
        })
    })
}

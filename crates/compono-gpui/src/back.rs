use compono_desktop::{Convention, Key, Modifiers};
use gpui::{Keystroke, MouseButton, NavigationDirection};

fn key(key: &str) -> Key {
    match key {
        "escape" => Key::Escape,
        "left" => Key::ArrowLeft,
        "[" => Key::BracketLeft,
        _ => Key::Other,
    }
}

/// Returns whether a key press means Back under [`Convention::CURRENT`].
///
/// Answer it with [`Tree::back`](compono::Tree::back) inside [`update`](crate::update).
#[must_use]
pub fn is_back_key(keystroke: &Keystroke) -> bool {
    let held = keystroke.modifiers;
    let modifiers = Modifiers {
        ctrl: held.control,
        alt: held.alt,
        shift: held.shift,
        logo: held.platform,
    };
    Convention::CURRENT.is_back(key(&keystroke.key), modifiers)
}

/// Returns whether a mouse button is the Back button.
#[must_use]
pub const fn is_back_button(button: MouseButton) -> bool {
    matches!(button, MouseButton::Navigate(NavigationDirection::Back))
}

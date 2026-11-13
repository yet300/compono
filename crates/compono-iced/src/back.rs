use compono_desktop::{Convention, Key, Modifiers};
use iced::keyboard::{self, key::Named};
use iced::{Event, Subscription, event, mouse};

/// The user asked for Back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackRequest;

fn key(key: &keyboard::Key) -> Key {
    match key {
        keyboard::Key::Named(Named::Escape) => Key::Escape,
        keyboard::Key::Named(Named::ArrowLeft) => Key::ArrowLeft,
        keyboard::Key::Named(Named::BrowserBack) => Key::BrowserBack,
        keyboard::Key::Character(text) if text.as_str() == "[" => Key::BracketLeft,
        _ => Key::Other,
    }
}

fn modifiers(modifiers: keyboard::Modifiers) -> Modifiers {
    Modifiers {
        ctrl: modifiers.control(),
        alt: modifiers.alt(),
        shift: modifiers.shift(),
        logo: modifiers.logo(),
    }
}

/// Returns whether `event` means Back under [`Convention::CURRENT`].
#[must_use]
pub fn is_back(event: &Event) -> bool {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: pressed,
            modifiers: held,
            repeat: false,
            ..
        }) => Convention::CURRENT.is_back(key(pressed), modifiers(*held)),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Back)) => true,
        _ => false,
    }
}

/// Reports every input that means Back and that no widget handled.
///
/// Answer it with [`Tree::back`](compono::Tree::back).
pub fn back_requests() -> Subscription<BackRequest> {
    event::listen_with(|event, status, _window| {
        (status == event::Status::Ignored && is_back(&event)).then_some(BackRequest)
    })
}

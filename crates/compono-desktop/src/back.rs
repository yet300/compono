//! Which inputs mean Back on each desktop system.

/// A key or button that can mean Back, in toolkit-neutral form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Key {
    /// The Escape key.
    Escape,
    /// The left arrow key.
    ArrowLeft,
    /// The `[` key.
    BracketLeft,
    /// The Back key found on some keyboards.
    BrowserBack,
    /// The Back button of a mouse, often its fourth button.
    MouseBack,
    /// Any other input, which never means Back.
    Other,
}

/// Modifier keys held during an input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each modifier is an independent key"
)]
pub struct Modifiers {
    /// Control.
    pub ctrl: bool,
    /// Alt, or Option on macOS.
    pub alt: bool,
    /// Shift.
    pub shift: bool,
    /// Command on macOS, the Windows key elsewhere.
    pub logo: bool,
}

impl Modifiers {
    const fn none(self) -> bool {
        !(self.ctrl || self.alt || self.shift || self.logo)
    }

    const fn only_alt(self) -> bool {
        self.alt && !(self.ctrl || self.shift || self.logo)
    }

    const fn only_logo(self) -> bool {
        self.logo && !(self.ctrl || self.alt || self.shift)
    }
}

/// The Back shortcuts of one family of systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Convention {
    /// macOS: Command-[ and Command-Left, as in Finder and Safari.
    Mac,
    /// Windows and Linux: Alt-Left, as in file managers and browsers.
    Other,
}

impl Convention {
    /// The convention of the system this program was built for.
    pub const CURRENT: Self = if cfg!(target_os = "macos") {
        Self::Mac
    } else {
        Self::Other
    };

    /// Returns whether `key` pressed with `modifiers` means Back.
    ///
    /// Escape, a keyboard Back key and a mouse Back button mean Back everywhere.
    #[must_use]
    pub const fn is_back(self, key: Key, modifiers: Modifiers) -> bool {
        match key {
            Key::Escape => modifiers.none(),
            Key::BrowserBack | Key::MouseBack => true,
            Key::ArrowLeft => match self {
                Self::Mac => modifiers.only_logo(),
                Self::Other => modifiers.only_alt(),
            },
            Key::BracketLeft => matches!(self, Self::Mac) && modifiers.only_logo(),
            Key::Other => false,
        }
    }
}

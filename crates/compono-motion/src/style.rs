//! What a UI applies to one drawn entry.

/// The size of the area a router draws in, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    /// Creates a size from a width and a height in logical pixels.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// What a UI applies to one drawn entry.
///
/// A UI translates the entry by `offset_x × width` and `offset_y × height`, scales
/// it about its centre, sets its opacity, clips it to `corner_radius`, and then
/// paints black over it at `dim`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerStyle {
    /// Horizontal offset as a share of the router's width. Positive moves right.
    pub offset_x: f32,
    /// Vertical offset as a share of the router's height. Positive moves down.
    pub offset_y: f32,
    /// Scale about the entry's centre.
    pub scale: f32,
    /// Opacity, `0..=1`.
    pub opacity: f32,
    /// Corner radius in logical pixels.
    pub corner_radius: f32,
    /// Opacity of a black scrim over the entry, `0..=1`.
    pub dim: f32,
}

impl LayerStyle {
    /// An entry at rest: in place, full size, opaque and undimmed.
    pub const REST: Self = Self {
        offset_x: 0.0,
        offset_y: 0.0,
        scale: 1.0,
        opacity: 1.0,
        corner_radius: 0.0,
        dim: 0.0,
    };
}

impl Default for LayerStyle {
    fn default() -> Self {
        Self::REST
    }
}

/// The role of one entry in a transition, with Decompose's names.
///
/// A push moves the new entry with `EnterFront` and the covered one with
/// `ExitBack`. A pop moves the leaving entry with `ExitFront` and the revealed one
/// with `EnterBack`. The front entry is drawn above the back one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    EnterFront,
    ExitFront,
    EnterBack,
    ExitBack,
}

impl Direction {
    /// Whether the entry is arriving rather than leaving.
    #[must_use]
    pub const fn is_enter(self) -> bool {
        matches!(self, Self::EnterFront | Self::EnterBack)
    }

    /// Whether the entry is drawn above the other one.
    #[must_use]
    pub const fn is_front(self) -> bool {
        matches!(self, Self::EnterFront | Self::ExitFront)
    }
}

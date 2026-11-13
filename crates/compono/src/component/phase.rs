/// How present a component currently is, ordered from least to most active.
///
/// Creation is the constructor and destruction is `Drop`, so these three values
/// are the whole lifecycle. A child's effective phase never exceeds its parent's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Alive but not on screen.
    #[default]
    Hidden,
    /// On screen but not the primary target of input.
    Visible,
    /// On screen and receiving input.
    Focused,
}

use core::fmt;

/// Why a call into a [`NativeTree`](crate::NativeTree) did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeError {
    /// The call came from a thread that does not own the tree.
    WrongThread,
    /// The tree is running, and this call came from inside that run.
    Busy,
    /// The tree or the addressed component no longer exists, or has another type.
    Gone,
    /// The tree's state could not be saved.
    Save,
    /// The tree's message queue was poisoned by a panic on another thread.
    QueueClosed,
    /// A panic in an earlier call on this tree was caught, so the tree is
    /// half-changed and runs nothing more ([`Tree::is_poisoned`](compono::Tree::is_poisoned)).
    Poisoned,
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WrongThread => "called from a thread that does not own the tree",
            Self::Busy => "called while the tree is running",
            Self::Gone => "the tree or component no longer exists",
            Self::Save => "the tree could not be saved",
            Self::QueueClosed => "the tree's message queue is closed",
            Self::Poisoned => "an earlier call on the tree panicked",
        })
    }
}

impl NativeError {
    /// Returns whether this error means a bug in the caller rather than normal use.
    ///
    /// A native UI meets [`NativeError::Gone`] in normal use, for example when a
    /// tap reaches a component that was just removed, and can ignore it. Every
    /// other error means the caller broke the contract: it called from another
    /// thread or from inside a running call, a component could not be encoded,
    /// or another thread panicked. Exported functions should not hide these; a
    /// debug build can stop at them.
    #[must_use]
    pub const fn is_misuse(self) -> bool {
        !matches!(self, Self::Gone)
    }
}

impl core::error::Error for NativeError {}

#[cfg(test)]
mod tests {
    use super::NativeError;

    #[test]
    fn only_a_missing_tree_or_component_is_normal_use() {
        assert!(!NativeError::Gone.is_misuse());
        for error in [
            NativeError::WrongThread,
            NativeError::Busy,
            NativeError::Save,
            NativeError::QueueClosed,
            NativeError::Poisoned,
        ] {
            assert!(error.is_misuse(), "{error:?}");
        }
    }
}

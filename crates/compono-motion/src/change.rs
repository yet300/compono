//! What changed in a stack.

/// What changed in a stack, from the keys of its entries before and after.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StackChange {
    /// The active entry is the same.
    None,
    /// A push, a replace, or an older entry brought to the front.
    Forward { from: u64, to: u64 },
    /// A pop: the old active entry is gone and the new one was below it.
    Backward { from: u64, to: u64 },
}

impl StackChange {
    /// Compares a stack before and after a change.
    ///
    /// `before` holds the entry keys before the change, oldest first, and `after`
    /// those after it. The change is backward only when `before_active` is gone
    /// from `after` and `after_active` was in `before`, below it. Anything else
    /// that changes the active entry is forward: a push, `replace`, `reset`, and
    /// `bring_to_front`, which keeps the old entry parked. This follows
    /// Decompose's `AbstractStackAnimation`.
    #[must_use]
    pub fn between(
        before: &[u64],
        before_active: u64,
        after: impl Iterator<Item = u64>,
        after_active: u64,
    ) -> Self {
        if before_active == after_active {
            return Self::None;
        }
        let mut old_remains = false;
        for key in after {
            if key == before_active {
                old_remains = true;
                break;
            }
        }
        let new_was_below = before
            .iter()
            .any(|&key| key == after_active && key != before_active);
        if !old_remains && new_was_below {
            Self::Backward {
                from: before_active,
                to: after_active,
            }
        } else {
            Self::Forward {
                from: before_active,
                to: after_active,
            }
        }
    }
}

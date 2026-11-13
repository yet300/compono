//! Every change a `Stack` makes, as keys before and after.

use compono_motion::StackChange;

fn between(before: &[u64], after: &[u64]) -> StackChange {
    let before_active = before.last().copied().unwrap_or_default();
    let after_active = after.last().copied().unwrap_or_default();
    StackChange::between(before, before_active, after.iter().copied(), after_active)
}

#[test]
fn an_unchanged_stack_is_no_change() {
    assert_eq!(between(&[0, 1], &[0, 1]), StackChange::None);
    // A change below the active entry moves nothing.
    assert_eq!(between(&[0, 1, 2], &[0, 2]), StackChange::None);
}

#[test]
fn a_push_is_forward() {
    assert_eq!(
        between(&[0], &[0, 1]),
        StackChange::Forward { from: 0, to: 1 }
    );
}

#[test]
fn a_pop_is_backward() {
    assert_eq!(
        between(&[0, 1], &[0]),
        StackChange::Backward { from: 1, to: 0 }
    );
}

#[test]
fn popping_several_entries_is_backward() {
    assert_eq!(
        between(&[0, 1, 2], &[0]),
        StackChange::Backward { from: 2, to: 0 }
    );
}

#[test]
fn a_replace_is_forward() {
    assert_eq!(
        between(&[0, 1], &[0, 2]),
        StackChange::Forward { from: 1, to: 2 }
    );
}

#[test]
fn a_reset_is_forward() {
    assert_eq!(
        between(&[0, 1], &[2]),
        StackChange::Forward { from: 1, to: 2 }
    );
}

#[test]
fn bringing_an_older_entry_to_front_is_forward() {
    assert_eq!(
        between(&[0, 1, 2], &[1, 2, 0]),
        StackChange::Forward { from: 2, to: 0 }
    );
}

#[test]
fn push_to_front_is_forward() {
    // The route of entry 0 is pushed again: 0 is removed and 3 is a new visit.
    assert_eq!(
        between(&[0, 1, 2], &[1, 2, 3]),
        StackChange::Forward { from: 2, to: 3 }
    );
}

#[test]
fn a_pop_after_bring_to_front_is_backward() {
    assert_eq!(
        between(&[1, 2, 0], &[1, 2]),
        StackChange::Backward { from: 0, to: 2 }
    );
}

#[test]
fn an_entry_never_seen_before_is_forward_even_with_a_lower_key() {
    // Keys wrap after u64::MAX, so a lower key is not an older visit.
    assert_eq!(
        between(&[u64::MAX], &[0]),
        StackChange::Forward {
            from: u64::MAX,
            to: 0
        }
    );
}

use compono::Phase;

/// The address that names no component, for a function that returns an address
/// or nothing.
///
/// It is `u64::MAX` as an `i64`, which [`NodeId::to_bits`](compono::NodeId::to_bits)
/// never returns.
pub const NO_NODE: i64 = -1;

const _: () = assert!(NO_NODE.cast_unsigned() == u64::MAX);

/// Encodes a phase as 0 for hidden, 1 for visible and 2 for focused.
#[must_use]
pub const fn phase_code(phase: Phase) -> u8 {
    match phase {
        Phase::Hidden => 0,
        Phase::Visible => 1,
        Phase::Focused => 2,
    }
}

/// Decodes [`phase_code`].
#[must_use]
pub const fn phase_from_code(code: u8) -> Option<Phase> {
    match code {
        0 => Some(Phase::Hidden),
        1 => Some(Phase::Visible),
        2 => Some(Phase::Focused),
        _ => None,
    }
}

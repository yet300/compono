/// Which way a stack moves when its active entry changes.
enum StackDirection: Equatable {
    case forward
    case back
}

/// Returns the direction of a change from `shown` to `active`.
///
/// The move is back only when the shown entry is gone from the stack and the new
/// active entry was below it: either it is `below`, the entry Back would have
/// revealed, or it is an older visit. Everything else is forward: a push, a
/// `replace`, and a `bringToFront` that leaves the shown entry parked.
func stackDirection(shown: Int64, active: Int64, below: Int64?, shownRemains: Bool) -> StackDirection {
    guard !shownRemains else { return .forward }
    return active == below || active < shown ? .back : .forward
}

package compono.android

/** Which way a stack moves when its active entry changes. */
internal enum class StackDirection { Forward, Back }

/**
 * Returns the direction of a change from [shown] to [active].
 *
 * The move is back only when the shown entry is gone from the stack and the new
 * active entry was below it: either it is [below], the entry Back would have
 * revealed, or it is an older visit. Everything else is forward: a push, a
 * `replace`, and a `bring_to_front` that leaves the shown entry parked.
 */
internal fun stackDirection(shown: Long, active: Long, below: Long?, shownRemains: Boolean): StackDirection =
    when {
        shownRemains -> StackDirection.Forward
        active == below || active < shown -> StackDirection.Back
        else -> StackDirection.Forward
    }

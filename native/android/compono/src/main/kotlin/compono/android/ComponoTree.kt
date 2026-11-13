package compono.android

import androidx.activity.BackEventCompat

import android.os.Handler
import android.os.Looper
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.tween
import androidx.compose.runtime.MutableIntState
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.util.concurrent.atomic.AtomicBoolean

/** What an application's generated bindings provide to the library. */
interface TreeBridge {
    fun pump()

    /** Sets the phase: 0 for hidden, 1 for visible and 2 for focused. */
    fun setPhase(code: Int)

    fun back(): Boolean

    /** Starts a Back gesture and returns its target's address, or -1. */
    fun backStart(): Long

    fun backCancel()

    fun backCommit(): Boolean

    fun contains(node: Long): Boolean

    /** Encodes navigation and persistent state. */
    fun save(): ByteArray
}

private const val MINIMUM_PRUNE_SIZE = 256
private const val SETTLE_MILLIS = 200

/** A Back gesture in progress. Progress stays in the UI and never crosses into Rust. */
@Stable
class BackGestureState internal constructor(
    /** The component that handles Back if the gesture completes. */
    val target: Long,
) {
    /** How far the gesture has gone, from 0 to 1. */
    var progress by mutableFloatStateOf(0f)
        internal set

    /** How far a committed gesture has animated its entry away, from 0 to 1. */
    var finish by mutableFloatStateOf(0f)
        internal set

    /** The edge the gesture started from. */
    internal var edge by mutableStateOf(SwipeEdge.Left)

    /** How far the touch has moved down since the gesture started, in pixels. */
    internal var touchTravel by mutableFloatStateOf(0f)

    private var touchStart: Float? = null

    internal fun touch(y: Float) {
        val start = touchStart ?: y.also { touchStart = it }
        touchTravel = y - start
    }

    /** Whether the finger is up and the gesture is animating to its end. */
    internal var settlingTo: Boolean? = null
}

/**
 * Connects a Rust component tree to Compose.
 *
 * A composable calls [observe] for each component it reads, so a commit recomposes
 * only the composables of components that changed. Every function except
 * [requestPump] must be called on the main thread.
 */
@Stable
class ComponoTree(private val post: (Runnable) -> Unit = mainThreadPost()) : ComponoListener {
    /** Whether a Back press would be handled. */
    var canBack by mutableStateOf(false)
        private set

    /** The Back gesture in progress, if any. */
    var gesture by mutableStateOf<BackGestureState?>(null)
        private set

    private var bridge: TreeBridge? = null
    private val stacks = HashMap<Long, Int>()
    private var committedTo: Long? = null
    private val versions = HashMap<Long, MutableIntState>()
    private var pruneAt = MINIMUM_PRUNE_SIZE
    private val pumpPending = AtomicBoolean(false)

    internal val trackedNodes: Int get() = versions.size

    /** Connects the tree's bindings, which may already have committed. */
    fun attach(bridge: TreeBridge) {
        this.bridge = bridge
    }

    /** Makes the calling composable depend on one component. */
    fun observe(node: Long) {
        versions.getOrPut(node) { mutableIntStateOf(0) }.intValue
    }

    /** Receives a commit. Call from the bindings' listener. */
    override fun commit(dirty: LongArray, canBack: Boolean) {
        for (node in dirty) {
            versions[node]?.let { it.intValue += 1 }
        }
        if (this.canBack != canBack) {
            this.canBack = canBack
        }
        pruneIfLarge()
    }

    /** Asks for one pump on the main thread. Any number of requests become one. */
    override fun schedulePump() = requestPump()

    fun requestPump() {
        if (pumpPending.compareAndSet(false, true)) {
            post(
                Runnable {
                    pumpPending.set(false)
                    bridge?.pump()
                },
            )
        }
    }

    /** Delivers a Back press and returns whether it was handled. */
    fun back(): Boolean = bridge?.back() ?: false

    /** Sets the phase: 0 for hidden, 1 for visible and 2 for focused. */
    fun setPhase(code: Int) {
        bridge?.setPhase(code)
    }

    /** Starts a Back gesture. Returns whether anything would handle it. */
    fun beginGesture(): Boolean {
        val bridge = bridge ?: return false
        val previous = gesture
        if (previous != null) {
            // A press while the last gesture animates to its end finishes that
            // gesture at once, so the press is not lost.
            val commit = previous.settlingTo ?: return false
            finish(previous, commit)
        }
        committedTo = null
        val target = bridge.backStart()
        if (target == -1L) return false
        gesture = BackGestureState(target)
        return true
    }

    /**
     * Moves the gesture. [swipeEdge] and [touchY] are AndroidX `BackEventCompat`'s:
     * `EDGE_LEFT`, `EDGE_RIGHT` or `EDGE_NONE`, and the touch's position in pixels.
     */
    fun updateGesture(progress: Float, swipeEdge: Int = BackEventCompat.EDGE_LEFT, touchY: Float = 0f) {
        val current = gesture ?: return
        current.progress = progress.coerceIn(0f, 1f)
        current.edge = when (swipeEdge) {
            BackEventCompat.EDGE_LEFT -> SwipeEdge.Left
            BackEventCompat.EDGE_RIGHT -> SwipeEdge.Right
            else -> SwipeEdge.None
        }
        current.touch(touchY)
    }

    /** Ends the gesture, animating a stack to whichever side it ended on. */
    suspend fun settleGesture(commit: Boolean) {
        val current = gesture ?: return
        current.settlingTo = commit
        try {
            if (current.target in stacks) {
                // A commit animates the entry away from where the gesture left it; a
                // cancel brings the gesture back to its start.
                if (commit) {
                    animate(0f, 1f, animationSpec = tween(SETTLE_MILLIS)) { value, _ -> current.finish = value }
                } else {
                    animate(current.progress, 0f, animationSpec = tween(SETTLE_MILLIS)) { value, _ ->
                        current.progress = value
                    }
                }
            }
        } finally {
            // The animation is cancelled when its composable leaves, as when the
            // Activity is recreated. The tree outlives it, so the gesture must end.
            if (gesture === current) finish(current, commit)
        }
    }

    private fun finish(current: BackGestureState, commit: Boolean) {
        gesture = null
        if (commit) {
            committedTo = current.target
            bridge?.backCommit()
        } else {
            bridge?.backCancel()
        }
    }

    /** Returns whether the component at [node] still exists. */
    internal fun containsNode(node: Long): Boolean = bridge?.contains(node) ?: false

    internal fun registerStack(owner: Long) {
        stacks[owner] = (stacks[owner] ?: 0) + 1
    }

    internal fun unregisterStack(owner: Long) {
        val remaining = (stacks[owner] ?: 1) - 1
        if (remaining > 0) stacks[owner] = remaining else stacks.remove(owner)
    }

    /** Returns whether a gesture just completed on this stack. */
    internal fun wasCommitted(owner: Long): Boolean = committedTo == owner

    internal fun clearCommitted() {
        committedTo = null
    }

    /** Encodes navigation and persistent state. */
    fun save(): ByteArray = bridge?.save() ?: ByteArray(0)

    private fun pruneIfLarge() {
        val bridge = bridge ?: return
        if (versions.size <= pruneAt) return
        versions.keys.retainAll { bridge.contains(it) }
        pruneAt = maxOf(MINIMUM_PRUNE_SIZE, versions.size * 2)
    }
}

private fun mainThreadPost(): (Runnable) -> Unit {
    val handler = Handler(Looper.getMainLooper())
    return { handler.post(it) }
}

package compono.android

import androidx.activity.compose.LocalActivity
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch

/**
 * The phase to set after a lifecycle event, or null to leave it.
 *
 * An Activity recreated for a configuration change is paused and stopped on the
 * way out, but the tree outlives it and nothing is hidden, so the phase stays.
 */
internal fun phaseUpdate(state: Lifecycle.State, changingConfigurations: Boolean): Int? =
    if (changingConfigurations) null else phaseCode(state)

/** The phase code for a lifecycle state: hidden, visible or focused. */
internal fun phaseCode(state: Lifecycle.State): Int = when {
    state.isAtLeast(Lifecycle.State.RESUMED) -> 2
    state.isAtLeast(Lifecycle.State.STARTED) -> 1
    else -> 0
}

/**
 * Follows the Activity's lifecycle and routes system Back into the tree.
 *
 * Back is predictive: a stack that handles it follows the gesture, showing the entry
 * beneath, and the three-button Back plays the same animation.
 */
@Composable
fun ComponoHost(tree: ComponoTree, content: @Composable () -> Unit) {
    val owner = LocalLifecycleOwner.current
    val activity = LocalActivity.current
    DisposableEffect(owner, tree) {
        val observer = LifecycleEventObserver { _, _ ->
            val changing = activity?.isChangingConfigurations == true
            phaseUpdate(owner.lifecycle.currentState, changing)?.let(tree::setPhase)
        }
        owner.lifecycle.addObserver(observer)
        onDispose { owner.lifecycle.removeObserver(observer) }
    }
    val scope = rememberCoroutineScope()
    PredictiveBackHandler(enabled = tree.canBack) { events ->
        val started = tree.beginGesture()
        try {
            events.collect { if (started) tree.updateGesture(it.progress, it.swipeEdge, it.touchY) }
            if (started) scope.launch { tree.settleGesture(commit = true) }
        } catch (cancelled: CancellationException) {
            if (started) scope.launch { tree.settleGesture(commit = false) }
            throw cancelled
        }
    }
    CompositionLocalProvider(LocalComponoTree provides tree, content = content)
}

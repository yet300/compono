package compono.android

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp

/** How a stack moves between entries. */
enum class StackAnimation {
    /** The new entry slides over the old one, and Back slides it off. */
    Slide,
    Fade,
    None,
}

private const val TRANSITION_MILLIS = 280

/** How far a covered entry sits to the left, as a share of the width. */
private const val PARALLAX = 0.3f

private class StackMotion<C : Any>(first: C) {
    var shown by mutableStateOf(first)
    var outgoing by mutableStateOf<C?>(null)
    var forward by mutableStateOf(true)
    var progress by mutableFloatStateOf(0f)
    var generation by mutableIntStateOf(0)
}

private class Layer<C>(val child: C, val shift: () -> Float, val alpha: () -> Float = { 1f })

/**
 * Draws the active entry of a stack, with transitions and an interactive Back.
 *
 * A component is destroyed as soon as it leaves the stack. The composable on its way
 * out is the one already composed, so it needs nothing further from the tree.
 *
 * Pass [tree], [owner] and [previous] to follow Back gestures aimed at the component
 * that owns this stack. [previous] is the entry that Back would reveal.
 */
@Composable
fun <C : Any> ChildStack(
    active: C,
    key: (C) -> Long,
    modifier: Modifier = Modifier,
    animation: StackAnimation = StackAnimation.Slide,
    tree: ComponoTree? = null,
    owner: Long? = null,
    previous: C? = null,
    background: Color = Color.White,
    content: @Composable (C) -> Unit,
) {
    val motion = remember { StackMotion(active) }
    if (tree != null && owner != null) {
        DisposableEffect(tree, owner) {
            tree.registerStack(owner)
            onDispose { tree.unregisterStack(owner) }
        }
    }
    if (key(active) != key(motion.shown)) {
        val committed = tree != null && owner != null && tree.wasCommitted(owner)
        if (committed || animation == StackAnimation.None) {
            motion.outgoing = null
            if (committed) SideEffect { tree?.clearCommitted() }
        } else {
            motion.forward = key(active) >= key(motion.shown)
            motion.outgoing = motion.shown
            motion.progress = 0f
            motion.generation += 1
        }
    }
    motion.shown = active
    LaunchedEffect(motion.generation) {
        if (motion.generation > 0) {
            animate(0f, 1f, animationSpec = tween(TRANSITION_MILLIS)) { value, _ -> motion.progress = value }
            motion.outgoing = null
            motion.progress = 0f
        }
    }
    val gesture = tree?.gesture?.takeIf { it.target == owner }
    val outgoing = motion.outgoing
    val layers: List<Layer<C>> = when {
        gesture != null -> buildList {
            if (previous != null) add(Layer(previous, { -PARALLAX * (1f - gesture.progress) }))
            add(Layer(active, { gesture.progress }))
        }
        outgoing == null -> listOf(Layer(active, { 0f }))
        animation == StackAnimation.Fade -> listOf(
            Layer(outgoing, { 0f }, { 1f - motion.progress }),
            Layer(active, { 0f }, { motion.progress }),
        )
        motion.forward -> listOf(
            Layer(outgoing, { -PARALLAX * motion.progress }),
            Layer(active, { 1f - motion.progress }),
        )
        else -> listOf(
            Layer(active, { -PARALLAX * (1f - motion.progress) }),
            Layer(outgoing, { motion.progress }),
        )
    }
    BoxWithConstraints(modifier.clipToBounds()) {
        val width = constraints.maxWidth.toFloat()
        for (layer in layers) {
            key(key(layer.child)) {
                Box(
                    Modifier
                        .fillMaxSize()
                        .graphicsLayer {
                            translationX = layer.shift() * width
                            alpha = layer.alpha()
                        }
                        .background(background),
                ) {
                    content(layer.child)
                }
            }
        }
    }
}

/** Draws the entry of a slot, if one is open, fading it in and out. */
@Composable
fun <C : Any> ChildSlot(active: C?, key: (C) -> Long, content: @Composable (C) -> Unit) {
    val last = remember { mutableStateOf(active) }
    if (active != null) last.value = active
    AnimatedVisibility(active != null, enter = fadeIn(), exit = fadeOut()) {
        last.value?.let { child ->
            key(key(child)) {
                content(child)
            }
        }
    }
}

/** Draws the selected page, sliding between pages by their order. */
@Composable
fun <C : Any> ChildPages(
    active: C?,
    key: (C) -> Long,
    modifier: Modifier = Modifier,
    background: Color = Color.White,
    content: @Composable (C) -> Unit,
) {
    if (active != null) {
        ChildStack(active, key, modifier, background = background, content = content)
    }
}

/** Draws the panels on screen side by side. */
@Composable
fun <C> ChildPanels(
    panels: List<C>,
    key: (C) -> Long,
    modifier: Modifier = Modifier,
    content: @Composable (C) -> Unit,
) {
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        for (panel in panels) {
            key(key(panel)) {
                Box(Modifier.weight(1f).fillMaxHeight()) {
                    content(panel)
                }
            }
        }
    }
}

/** Returns the rows on screen, or null before the first layout. */
internal fun LazyListState.visibleRange(): IntRange? {
    val visible = layoutInfo.visibleItemsInfo
    val first = visible.firstOrNull() ?: return null
    return first.index..visible.last().index
}

/** How many spare rows a list keeps alive on each side of the rows on screen. */
const val COMPONO_OVERSCAN = 10

/** Rows from [start] up to, but not including, [end]. */
internal data class RowWindow(val start: Int, val end: Int)

/**
 * Returns the window to report, or null while [current] still covers the rows on
 * screen. The window is the visible rows plus [overscan] rows on each side.
 */
internal fun nextWindow(visible: RowWindow, current: RowWindow?, overscan: Int, count: Int): RowWindow? {
    val end = minOf(visible.end, count)
    val start = minOf(visible.start, end)
    if (current != null && current.start <= start && end <= current.end && current.end <= count) {
        return null
    }
    val window = RowWindow(maxOf(start - overscan, 0), minOf(end + overscan, count))
    return if (window == current) null else window
}

/**
 * A long list that reports the rows on screen to its owner.
 *
 * It reports the rows on screen plus [overscan] spare rows on each side, so a fast
 * scroll reaches rows whose components already exist, and reports again only when
 * the rows on screen leave that window.
 */
@Composable
fun ChildList(
    count: Int,
    onWindow: (first: Int, last: Int) -> Unit,
    modifier: Modifier = Modifier,
    overscan: Int = COMPONO_OVERSCAN,
    row: @Composable (Int) -> Unit,
) {
    val state = rememberLazyListState()
    val report = rememberUpdatedState(onWindow)
    val limits = rememberUpdatedState(count to overscan)
    LaunchedEffect(state) {
        var reported: RowWindow? = null
        snapshotFlow { state.visibleRange() }.collect { range ->
            if (range != null) {
                val (rows, spare) = limits.value
                val window = nextWindow(RowWindow(range.first, range.last + 1), reported, spare, rows)
                if (window != null) {
                    reported = window
                    report.value(window.start, window.end)
                }
            }
        }
    }
    LazyColumn(modifier, state) {
        items(count, key = { it }) { index -> row(index) }
    }
}

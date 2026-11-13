package compono.showcase.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember

/**
 * Returns a child's component object for [key], or `null` when its parent refuses
 * the child, created once and closed when it leaves the composition.
 *
 * The showcase's accessors return `null` for a child of another kind.
 */
@Composable
fun <T : AutoCloseable> rememberChild(key: Any, create: () -> T?): T? {
    val component = remember(key) { create() }
    DisposableEffect(component) { onDispose { component?.close() } }
    return component
}

package compono.android

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.NonRestartableComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf

/** The tree of the nearest [ComponoHost]. */
val LocalComponoTree = staticCompositionLocalOf<ComponoTree?> { null }

/**
 * Makes the calling composable recompose when the component at [node] changes.
 *
 * It has no recompose scope of its own, so the read belongs to the caller.
 */
@Composable
@NonRestartableComposable
fun observe(node: Long) {
    LocalComponoTree.current?.observe(node)
}

/**
 * Returns a component object for [key], created once and closed when it leaves the
 * composition.
 *
 * Generated Kotlin bindings free a Rust object only on `close()`, so a component
 * taken from its parent inside a composable goes through this:
 *
 * ```kotlin
 * val counter = rememberComponent(child.node) { counters.counter(child) }
 * CounterContent(counter)
 * ```
 */
@Composable
fun <T : AutoCloseable> rememberComponent(key: Any, create: () -> T): T {
    val component = remember(key) { create() }
    DisposableEffect(component) { onDispose { component.close() } }
    return component
}

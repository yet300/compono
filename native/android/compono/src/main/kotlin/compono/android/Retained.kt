package compono.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider

/**
 * What the bindings' listener receives. The names match a BoltFFI listener, so an
 * application passes the tree on with `object : Listener, ComponoListener by it {}`.
 */
interface ComponoListener {
    /** One finished change: the components that changed, and whether Back is handled. */
    fun commit(dirty: LongArray, canBack: Boolean)

    /** Asks for one pump on the main thread. Called from any thread. */
    fun schedulePump()
}

/** Builds a [TreeBridge] from the bindings' methods, usually given as references. */
fun treeBridge(
    pump: () -> Unit,
    setPhase: (Int) -> Unit,
    back: () -> Boolean,
    backStart: () -> Long,
    backCancel: () -> Unit,
    backCommit: () -> Boolean,
    contains: (Long) -> Boolean,
    save: () -> ByteArray,
): TreeBridge = object : TreeBridge {
    override fun pump() = pump()
    override fun setPhase(code: Int) = setPhase(code)
    override fun back() = back()
    override fun backStart() = backStart()
    override fun backCancel() = backCancel()
    override fun backCommit() = backCommit()
    override fun contains(node: Long) = contains(node)
    override fun save() = save()
}

/** A tree and the application's bindings, kept while their Activity is. */
class ComponoRoot<T : AutoCloseable> internal constructor(
    /** The tree the composables observe. */
    val tree: ComponoTree,
    /** The application's bindings, for reading models and sending actions. */
    val app: T,
)

internal class RetainedRoot : ViewModel() {
    var root: ComponoRoot<*>? = null

    override fun onCleared() {
        // The Activity is finishing for good, so the Rust tree is freed.
        root?.app?.close()
        root = null
    }
}

private const val STATE = "state"

/**
 * Returns the tree of this Activity, creating it on first use.
 *
 * The tree outlives configuration changes, as Decompose's `retainedComponent` does,
 * and is closed when the Activity finishes. Its state is saved in the Activity's
 * saved state, so it also survives the process being killed. Call it in `onCreate`,
 * once per [key].
 *
 * ```kotlin
 * val root = retainedTree(
 *     create = { listener, saved -> ShowcaseApp(object : Listener, ComponoListener by listener {}, saved) },
 *     bridge = { app -> treeBridge(app::pump, app::setPhase, app::back, app::backStart,
 *         app::backCancel, app::backCommit, app::contains, app::save) },
 * )
 * setContent { ComponoHost(root.tree) { RootScreen(root.app) } }
 * ```
 */
fun <T : AutoCloseable> ComponentActivity.retainedTree(
    key: String = "compono",
    create: (listener: ComponoListener, saved: ByteArray) -> T,
    bridge: (T) -> TreeBridge,
): ComponoRoot<T> {
    require(savedStateRegistry.getSavedStateProvider(key) == null) {
        "A tree is already retained under the key $key"
    }
    val factory = object : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <V : ViewModel> create(modelClass: Class<V>): V = RetainedRoot() as V
    }
    val holder = ViewModelProvider(this, factory)[key, RetainedRoot::class.java]
    @Suppress("UNCHECKED_CAST")
    val root = holder.root as ComponoRoot<T>? ?: run {
        val saved = savedStateRegistry.consumeRestoredStateForKey(key)?.getByteArray(STATE) ?: ByteArray(0)
        val tree = ComponoTree()
        val app = create(tree, saved)
        tree.attach(bridge(app))
        ComponoRoot(tree, app).also { holder.root = it }
    }
    savedStateRegistry.registerSavedStateProvider(key) {
        Bundle().apply { putByteArray(STATE, root.tree.save()) }
    }
    return root
}

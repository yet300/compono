package compono.android

import androidx.compose.runtime.MonotonicFrameClock
import androidx.compose.runtime.snapshots.Snapshot
import androidx.lifecycle.Lifecycle
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private class FakeBridge : TreeBridge {
    var pumps = 0
    var live = emptySet<Long>()

    override fun pump() {
        pumps += 1
    }

    override fun setPhase(code: Int) {}

    override fun back() = true

    var target = 7L
    var cancels = 0
    var commits = 0

    override fun backStart() = target

    override fun backCancel() {
        cancels += 1
    }

    override fun backCommit(): Boolean {
        commits += 1
        return true
    }

    override fun contains(node: Long) = node in live

    override fun save() = ByteArray(0)
}

class ComponoTreeTest {
    private val queue = ArrayDeque<Runnable>()
    private val tree = ComponoTree(post = { queue.addLast(it) })

    private fun drain() {
        while (queue.isNotEmpty()) queue.removeFirst().run()
    }

    private fun readsOf(body: () -> Unit): Int {
        var reads = 0
        Snapshot.observe(readObserver = { reads += 1 }, block = body)
        return reads
    }

    @Test
    fun observingAComponentReadsItsVersion() {
        assertEquals(1, readsOf { tree.observe(1) })
    }

    @Test
    fun manyPumpRequestsBecomeOne() {
        val bridge = FakeBridge()
        tree.attach(bridge)
        repeat(100) { tree.requestPump() }
        assertEquals(1, queue.size)
        drain()
        assertEquals(1, bridge.pumps)
        tree.requestPump()
        drain()
        assertEquals(2, bridge.pumps)
    }

    @Test
    fun destroyedComponentsAreForgotten() {
        val bridge = FakeBridge()
        bridge.live = setOf(1L)
        tree.attach(bridge)
        for (node in 1L..300L) tree.observe(node)
        assertEquals(300, tree.trackedNodes)
        tree.commit(LongArray(0), canBack = false)
        assertEquals(1, tree.trackedNodes)
    }

    @Test
    fun aGestureWithNoStackEndsAtOnceOnEitherSide() = runBlocking {
        val bridge = FakeBridge()
        tree.attach(bridge)
        assertTrue(tree.beginGesture())
        assertEquals(7L, tree.gesture?.target)
        tree.settleGesture(commit = false)
        assertNull(tree.gesture)
        assertEquals(1, bridge.cancels)
        assertTrue(tree.beginGesture())
        tree.settleGesture(commit = true)
        assertEquals(1, bridge.commits)
    }

    /** A frame clock that never ticks, as when the Activity is being recreated. */
    private object Stopped : MonotonicFrameClock {
        override suspend fun <R> withFrameNanos(onFrame: (Long) -> R): R = awaitCancellation()
    }

    @Test
    fun aGestureWhoseAnimationIsCancelledStillEnds() = runBlocking {
        val bridge = FakeBridge()
        tree.attach(bridge)
        tree.registerStack(7)
        assertTrue(tree.beginGesture())
        val settling = launch(Stopped) { tree.settleGesture(commit = true) }
        yield()
        settling.cancelAndJoin()
        assertNull(tree.gesture)
        assertEquals(1, bridge.commits)
        assertTrue(tree.beginGesture())
    }

    @Test
    fun backDuringASettlingGestureEndsItAndStartsAnother() = runBlocking {
        val bridge = FakeBridge()
        tree.attach(bridge)
        tree.registerStack(7)
        assertTrue(tree.beginGesture())
        val settling = launch(Stopped) { tree.settleGesture(commit = true) }
        yield()
        assertTrue("a press while settling is not dropped", tree.beginGesture())
        assertEquals("the settling gesture committed first", 1, bridge.commits)
        assertEquals(0, bridge.cancels)
        settling.cancelAndJoin()
        assertEquals("the old animation ends nothing again", 1, bridge.commits)
        assertEquals(7L, tree.gesture?.target)
    }

    @Test
    fun aGestureNeedsAHandlerAndProgressStaysInRange() {
        val bridge = FakeBridge()
        tree.attach(bridge)
        bridge.target = -1
        assertFalse(tree.beginGesture())
        bridge.target = 7
        assertTrue(tree.beginGesture())
        tree.updateGesture(3f)
        assertEquals(1f, tree.gesture?.progress)
        tree.updateGesture(-1f)
        assertEquals(0f, tree.gesture?.progress)
    }

    @Test
    fun aConfigurationChangeLeavesThePhaseAlone() {
        assertNull("stopping for a rotation is not hiding", phaseUpdate(Lifecycle.State.CREATED, changingConfigurations = true))
        assertNull(phaseUpdate(Lifecycle.State.STARTED, changingConfigurations = true))
        assertEquals(0, phaseUpdate(Lifecycle.State.CREATED, changingConfigurations = false))
        assertEquals(2, phaseUpdate(Lifecycle.State.RESUMED, changingConfigurations = false))
    }

    @Test
    fun lifecycleStatesMapToPhases() {
        assertEquals(0, phaseCode(Lifecycle.State.CREATED))
        assertEquals(1, phaseCode(Lifecycle.State.STARTED))
        assertEquals(2, phaseCode(Lifecycle.State.RESUMED))
    }
}

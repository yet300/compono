package compono.android

import org.junit.Assert.assertEquals
import org.junit.Test

/** Keys are visits, numbered from 0 as a Rust `Stack` numbers them. */
class StackDirectionTest {
    @Test
    fun aPushMovesForward() {
        // [0] -> [0, 1]: the old entry stays parked.
        assertEquals(StackDirection.Forward, stackDirection(shown = 0, active = 1, below = null, shownRemains = true))
    }

    @Test
    fun aPopMovesBack() {
        // [0, 1] -> [0]: the top entry is destroyed.
        assertEquals(StackDirection.Back, stackDirection(shown = 1, active = 0, below = 0, shownRemains = false))
    }

    @Test
    fun poppingSeveralEntriesMovesBack() {
        // [0, 1, 2] -> [0]
        assertEquals(StackDirection.Back, stackDirection(shown = 2, active = 0, below = 1, shownRemains = false))
    }

    @Test
    fun bringingAnOlderEntryToFrontMovesForward() {
        // [0, 1, 2] -> [1, 2, 0]: the shown entry stays parked.
        assertEquals(StackDirection.Forward, stackDirection(shown = 2, active = 0, below = 1, shownRemains = true))
        assertEquals(StackDirection.Forward, stackDirection(shown = 2, active = 1, below = 1, shownRemains = true))
    }

    @Test
    fun aReplaceMovesForward() {
        // [0, 1] -> [0, 2]: the old top is destroyed, the new one is a new visit.
        assertEquals(StackDirection.Forward, stackDirection(shown = 1, active = 2, below = 0, shownRemains = false))
    }

    @Test
    fun aPopAfterBringToFrontMovesBack() {
        // [0, 1, 2] -> bring 0 to front -> [1, 2, 0] -> pop -> [1, 2].
        assertEquals(StackDirection.Back, stackDirection(shown = 0, active = 2, below = 2, shownRemains = false))
    }
}

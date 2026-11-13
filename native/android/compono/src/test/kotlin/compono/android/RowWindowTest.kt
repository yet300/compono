package compono.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RowWindowTest {
    @Test
    fun aFirstReportAddsSpareRowsOnEachSide() {
        assertEquals(RowWindow(10, 40), nextWindow(RowWindow(20, 30), null, 10, 1000))
    }

    @Test
    fun spareRowsStopAtTheEndsOfTheList() {
        assertEquals(RowWindow(0, 12), nextWindow(RowWindow(0, 5), null, 10, 12))
        assertEquals(RowWindow(985, 1000), nextWindow(RowWindow(995, 1000), null, 10, 1000))
    }

    @Test
    fun scrollingInsideTheWindowReportsNothing() {
        assertNull(nextWindow(RowWindow(15, 25), RowWindow(10, 40), 10, 1000))
    }

    @Test
    fun leavingTheWindowMovesItWithTheSpareRows() {
        assertEquals(RowWindow(25, 55), nextWindow(RowWindow(35, 45), RowWindow(10, 40), 10, 1000))
    }
}

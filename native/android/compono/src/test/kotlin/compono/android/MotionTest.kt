package compono.android

import androidx.compose.animation.core.TargetBasedAnimation
import androidx.compose.animation.core.VectorConverter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/** One row of `crates/compono-motion/golden/motion.csv`. */
private data class Row(
    val preset: String,
    val preference: String,
    val section: String,
    val role: String,
    val x: Float,
    val style: LayerStyle,
)

private val golden: List<Row> by lazy {
    File("../../../crates/compono-motion/golden/motion.csv").readLines().drop(1).map { line ->
        val c = line.split(',')
        val n = c.drop(4).map(String::toFloat)
        Row(c[0], c[1], c[2], c[3], n[0], LayerStyle(n[1], n[2], n[3], n[4], n[5], n[6]))
    }
}

private const val WIDTH = 400f
private const val HEIGHT = 800f
private const val TOLERANCE = 1e-3f

private fun assertClose(expected: LayerStyle, actual: LayerStyle, what: String) {
    val pairs = listOf(
        expected.offsetX to actual.offsetX,
        expected.offsetY to actual.offsetY,
        expected.scale to actual.scale,
        expected.opacity to actual.opacity,
        expected.cornerRadius to actual.cornerRadius,
        expected.dim to actual.dim,
    )
    for ((e, a) in pairs) {
        assertEquals("$what: expected $expected, got $actual", e, a, TOLERANCE)
    }
}

private fun preference(name: String) = when (name) {
    "full" -> MotionPreference.Full
    "reduced" -> MotionPreference.Reduced
    else -> MotionPreference.Off
}

private fun role(name: String) = when (name) {
    "enter_front" -> Role.EnterFront
    "exit_front" -> Role.ExitFront
    "enter_back" -> Role.EnterBack
    else -> Role.ExitBack
}

class MotionTest {
    private val rows = golden.filter { it.preset == "material" }

    @Test
    fun theGoldenTableIsRead() {
        assertTrue(rows.size > 100)
    }

    @Test
    fun pushAndPopMatchTheMaterialPreset() {
        val moves = rows.filter { it.section == "push" || it.section == "pop" }
        for ((key, group) in moves.groupBy { Triple(it.preference, it.section, it.x) }) {
            val (pref, section, seconds) = key
            val animation = StackAnimation.Platform.forPreference(preference(pref))
            val run = TargetBasedAnimation(animation.spec, Float.VectorConverter, 0f, 1f)
            val nanos = (seconds * 1e9).toLong()
            val what = "$pref $section at $seconds s"
            if (group.all { it.style == LayerStyle() } && (seconds > 0f || run.isFinishedFromNanos(nanos))) {
                // The table shows a finished transition at rest; Compose has finished too.
                assertTrue(what, run.isFinishedFromNanos(nanos))
                continue
            }
            val progress = run.getValueFromNanos(nanos)
            for (row in group) {
                assertClose(row.style, layer(animation.transition, role(row.role), progress, WIDTH), "$what ${row.role}")
            }
        }
    }

    @Test
    fun predictiveBackMatchesTheMaterialPreset() {
        val gestures = rows.filter { it.section.startsWith("back") }
        assertTrue(gestures.isNotEmpty())
        for (row in gestures) {
            val back = StackAnimation.Platform.forPreference(preference(row.preference)).back
            val (progress, edge, touch, finish) = when (row.section) {
                "back" -> listOf(row.x, SwipeEdge.Left, 0f, 0f)
                "back_right" -> listOf(row.x, SwipeEdge.Right, 0f, 0f)
                "back_finish" -> listOf(0.5f, SwipeEdge.Left, 0f, row.x)
                else -> listOf(0.5f, SwipeEdge.Left, row.x, 0f)
            }
            val (under, over) = backFrame(
                back,
                progress as Float,
                edge as SwipeEdge,
                touch as Float,
                finish as Float,
                WIDTH,
                HEIGHT,
            )
            val actual = if (row.role == "under") under else over
            assertClose(row.style, actual, "${row.preference} ${row.section} ${row.x} ${row.role}")
        }
    }

    @Test
    fun removeAnimationsMeansNoMotion() {
        assertEquals(MotionPreference.Off, motionPreference(0f))
        assertEquals(MotionPreference.Full, motionPreference(1f))
        assertEquals(MotionPreference.Full, motionPreference(0.5f))
    }
}

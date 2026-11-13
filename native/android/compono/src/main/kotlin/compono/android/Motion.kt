package compono.android

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

// The formulas here follow the Rust crate `compono-motion`. Tests compare them with
// its golden table, `crates/compono-motion/golden/motion.csv`.

/** How the entries of a stack move during a transition. */
sealed interface StackTransition {
    /** The new entry slides over; the covered one moves [parallax] of the width, shaded by [dim]. */
    data class Slide(val parallax: Float = 0.3f, val dim: Float = 0.12f) : StackTransition

    /** Entering entries fade in, leaving ones fade out. */
    data object Fade : StackTransition

    /** The front entry scales from [front] to 1 and fades in; the back one scales to [back]. */
    data class Scale(val front: Float = 1.15f, val back: Float = 0.95f) : StackTransition

    /** Material shared axis X: both entries shift by [shift] and fade through at 35 %. */
    data class SharedAxisX(val shift: Dp = 30.dp) : StackTransition

    /** Nothing moves. */
    data object None : StackTransition
}

/** How a stack follows a Back gesture. */
sealed interface BackAnimation {
    /**
     * Android's in-app predictive Back, Material style: the top entry shrinks to 90 %,
     * moves toward the far edge, follows the touch and rounds its corners up to
     * [cornerRadius]; the entry under it is shaded by 25 %.
     */
    data class Predictive(val cornerRadius: Dp = 16.dp) : BackAnimation

    /** The top entry follows the finger; the one under it moves in from [parallax]. */
    data class Swipe(val parallax: Float = 0.3f, val dim: Float = 0.12f) : BackAnimation

    /** The top entry fades with the gesture: reduced motion. */
    data object Fade : BackAnimation

    /** Nothing follows the gesture. */
    data object None : BackAnimation
}

/** What the user asked of motion. */
enum class MotionPreference {
    Full,

    /** Spatial transitions become a 150 ms fade; a Back gesture only fades. */
    Reduced,

    /** No motion. */
    Off,
}

/** Android's "Remove animations" sets the animator duration scale to 0, which means [MotionPreference.Off]. */
fun motionPreference(animatorDurationScale: Float): MotionPreference =
    if (animatorDurationScale == 0f) MotionPreference.Off else MotionPreference.Full

/** Material 3 standard easing, `MotionTokens.EasingStandardCubicBezier`. */
internal val StandardEasing = CubicBezierEasing(0.2f, 0f, 0f, 1f)

/** How a stack moves: its transition, timing and Back gesture. */
class StackAnimation(
    val transition: StackTransition = StackTransition.SharedAxisX(),
    /** Material 3 standard default spatial spring: damping 0.9, stiffness 700. */
    val spec: FiniteAnimationSpec<Float> = spring(dampingRatio = 0.9f, stiffness = 700f),
    val back: BackAnimation = BackAnimation.Predictive(),
) {
    /** This animation changed for [preference]. */
    fun forPreference(preference: MotionPreference): StackAnimation = when (preference) {
        MotionPreference.Full -> this
        MotionPreference.Reduced -> StackAnimation(
            transition = StackTransition.Fade,
            spec = if (transition.isSpatial) tween(REDUCED_MILLIS, easing = StandardEasing) else spec,
            back = if (back == BackAnimation.None) back else BackAnimation.Fade,
        )
        MotionPreference.Off -> None
    }

    companion object {
        /** The Material look: shared axis X on the spatial spring, with predictive Back. */
        val Platform = StackAnimation()

        /** The new entry slides over the old one, and Back slides it off. */
        val Slide = StackAnimation(
            transition = StackTransition.Slide(),
            back = BackAnimation.Swipe(),
        )

        /** Entries fade into each other over 150 ms, and Back fades the top entry. */
        val Fade = StackAnimation(
            transition = StackTransition.Fade,
            spec = tween(REDUCED_MILLIS, easing = StandardEasing),
            back = BackAnimation.Fade,
        )

        /** No motion. */
        val None = StackAnimation(StackTransition.None, tween(0), BackAnimation.None)
    }
}

private const val REDUCED_MILLIS = 150

private val StackTransition.isSpatial: Boolean
    get() = this != StackTransition.Fade && this != StackTransition.None

/** The role of an entry in a transition, as `compono_motion::Direction`. */
internal enum class Role {
    EnterFront,
    ExitFront,
    EnterBack,
    ExitBack,
    ;

    val isEnter: Boolean get() = this == EnterFront || this == EnterBack
    val isFront: Boolean get() = this == EnterFront || this == ExitFront
}

/** What a stack applies to one entry. Offsets are shares of the stack's size; [cornerRadius] is in dp. */
internal data class LayerStyle(
    val offsetX: Float = 0f,
    val offsetY: Float = 0f,
    val scale: Float = 1f,
    val opacity: Float = 1f,
    val cornerRadius: Float = 0f,
    val dim: Float = 0f,
)

private const val FADE_THROUGH = 0.35f

private fun lerp(from: Float, to: Float, t: Float) = from + (to - from) * t

/** The style of an entry in [role] at [progress], in a stack [widthDp] wide. */
internal fun layer(transition: StackTransition, role: Role, progress: Float, widthDp: Float): LayerStyle {
    val away = if (role.isEnter) 1f - progress else progress
    return when (transition) {
        is StackTransition.Slide ->
            if (role.isFront) {
                LayerStyle(offsetX = away)
            } else {
                LayerStyle(offsetX = -transition.parallax * away, dim = (transition.dim * away).coerceIn(0f, 1f))
            }
        StackTransition.Fade -> LayerStyle(opacity = (1f - away).coerceIn(0f, 1f))
        is StackTransition.Scale ->
            if (role.isFront) {
                LayerStyle(scale = lerp(1f, transition.front, away), opacity = (1f - away).coerceIn(0f, 1f))
            } else {
                LayerStyle(scale = lerp(1f, transition.back, away))
            }
        is StackTransition.SharedAxisX -> {
            val shift = if (widthDp > 0f) transition.shift.value / widthDp else 0f
            val opacity = if (role.isEnter) {
                ((progress - FADE_THROUGH) / (1f - FADE_THROUGH)).coerceIn(0f, 1f)
            } else {
                (1f - progress / FADE_THROUGH).coerceIn(0f, 1f)
            }
            LayerStyle(offsetX = (if (role.isFront) shift else -shift) * away, opacity = opacity)
        }
        StackTransition.None -> if (role.isEnter) LayerStyle() else LayerStyle(opacity = 0f)
    }
}

/** Where a Back gesture started, as AndroidX `BackEventCompat.swipeEdge` reports it. */
internal enum class SwipeEdge { Left, Right, None }

private const val PREDICTIVE_SCALE = 0.9f
private const val PREDICTIVE_GAP = 8f
private const val PREDICTIVE_SCRIM = 0.25f

/**
 * The entry under the gesture and the one it removes, at [progress]. [touchY] is the
 * touch's travel down since the gesture started, in dp; [finish] runs from 0 to 1
 * after a commit.
 */
internal fun backFrame(
    back: BackAnimation,
    progress: Float,
    edge: SwipeEdge,
    touchY: Float,
    finish: Float,
    widthDp: Float,
    heightDp: Float,
): Pair<LayerStyle, LayerStyle> {
    val p = progress.coerceIn(0f, 1f)
    val f = finish.coerceIn(0f, 1f)
    val travel = p + (1f - p) * f
    return when (back) {
        is BackAnimation.Swipe -> LayerStyle(
            offsetX = -back.parallax * (1f - travel),
            dim = (back.dim * (1f - travel)).coerceIn(0f, 1f),
        ) to LayerStyle(offsetX = travel)
        is BackAnimation.Predictive -> {
            val scale = lerp(1f, PREDICTIVE_SCALE, p)
            val gap = if (widthDp > 0f) PREDICTIVE_GAP / widthDp else 0f
            val toward = (1f - scale) / 2f - gap * p
            val offsetX = when (edge) {
                SwipeEdge.Left -> toward
                SwipeEdge.Right -> -toward
                SwipeEdge.None -> 0f
            }
            val offsetY = if (heightDp > 0f) {
                val limit = (heightDp / 20f - PREDICTIVE_GAP).coerceAtLeast(0f)
                touchY.coerceIn(-limit, limit) * (3f * p).coerceAtMost(1f) / heightDp
            } else {
                0f
            }
            LayerStyle(dim = PREDICTIVE_SCRIM * (1f - f)) to LayerStyle(
                offsetX = offsetX,
                offsetY = offsetY,
                scale = scale,
                opacity = 1f - f,
                cornerRadius = back.cornerRadius.value * p,
            )
        }
        BackAnimation.Fade -> LayerStyle() to LayerStyle(opacity = 1f - travel)
        BackAnimation.None -> LayerStyle() to LayerStyle()
    }
}

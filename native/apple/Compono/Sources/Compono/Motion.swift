import SwiftUI

// The formulas here follow the Rust crate `compono-motion`. Tests compare them with
// its golden table, `crates/compono-motion/golden/motion.csv`.

/// How the entries of a stack move during a transition.
public enum StackTransition: Sendable, Equatable {
    /// The new entry slides over; the covered one moves `parallax` of the width to
    /// the left under a black shade of up to `dim`.
    case slide(parallax: CGFloat, dim: CGFloat)
    /// Entering entries fade in and leaving ones fade out.
    case fade
    /// The front entry scales from `front` to 1 and fades in; the back one scales
    /// from 1 to `back`.
    case scale(front: CGFloat, back: CGFloat)
    /// Nothing moves.
    case none
}

/// How a stack follows a Back gesture.
public enum BackGestureStyle: Sendable, Equatable {
    /// The top entry follows the finger; the one under it moves in from `parallax`
    /// of the width to the left, shaded by up to `dim`.
    case swipe(parallax: CGFloat, dim: CGFloat)
    /// The top entry fades with the gesture: reduced motion.
    case fade
    /// Nothing follows the gesture.
    case none
}

/// A `UINavigationController` push: response 0.29 s, critically damped. Apple does
/// not document it; three pushes measured frame by frame in the iOS 26 simulator
/// fit 0.28 to 0.296 s. It is `Spring::CUPERTINO` in `compono-motion`.
let platformSpring = Spring(response: 0.29, dampingRatio: 1)

/// How a stack moves: its transition, its timing and its Back gesture.
public struct StackAnimation: Sendable, Equatable {
    public var transition: StackTransition
    public var animation: Animation
    public var back: BackGestureStyle

    public init(transition: StackTransition, animation: Animation, back: BackGestureStyle) {
        self.transition = transition
        self.animation = animation
        self.back = back
    }

    /// The iOS look: the new screen slides over, the covered one moves 30 % to the
    /// left under a 12 % shade, on UIKit's push spring, with the swipe-back.
    /// The spring, the 30 % and the 12 % are measured on `UINavigationController`
    /// pushes in the iOS 26 simulator; Apple does not document them.
    public static let platform = StackAnimation(
        transition: .slide(parallax: 0.3, dim: 0.12),
        animation: .spring(platformSpring),
        back: .swipe(parallax: 0.3, dim: 0.12)
    )

    /// The same as ``platform``.
    public static let slide = platform

    /// Entries fade into each other over 150 ms, and Back slides the top entry off.
    public static let fade = StackAnimation(
        transition: .fade,
        animation: .timingCurve(0.2, 0, 0, 1, duration: 0.15),
        back: .swipe(parallax: 0.3, dim: 0.12)
    )

    /// Entries change at once. Back still slides the top entry off.
    public static let none = StackAnimation(
        transition: .none,
        animation: .linear(duration: 0),
        back: .swipe(parallax: 0.3, dim: 0.12)
    )

    /// This animation with reduced motion: a transition that moves entries becomes a
    /// 150 ms fade, and a Back gesture fades the top entry instead of moving it.
    public func reduced() -> StackAnimation {
        var reduced = self
        switch transition {
        case .slide, .scale:
            reduced.transition = .fade
            reduced.animation = .timingCurve(0.2, 0, 0, 1, duration: 0.15)
        case .fade, .none:
            break
        }
        if back != .none {
            reduced.back = .fade
        }
        return reduced
    }
}

/// The role of an entry in a transition, as `compono_motion::Direction`.
enum LayerRole {
    case enterFront, exitFront, enterBack, exitBack

    var isEnter: Bool { self == .enterFront || self == .enterBack }
    var isFront: Bool { self == .enterFront || self == .exitFront }
}

/// What a stack applies to one entry. `offsetX` is a share of the width.
struct LayerStyle: Equatable {
    var offsetX: CGFloat = 0
    var scale: CGFloat = 1
    var opacity: CGFloat = 1
    var dim: CGFloat = 0

    static let rest = LayerStyle()
}

private func clamp01(_ value: CGFloat) -> CGFloat { min(max(value, 0), 1) }

/// The style of an entry in `role` at `progress`, where 0 starts the transition.
func layerStyle(_ transition: StackTransition, role: LayerRole, progress: CGFloat) -> LayerStyle {
    let away = role.isEnter ? 1 - progress : progress
    switch transition {
    case let .slide(parallax, dim):
        return role.isFront
            ? LayerStyle(offsetX: away)
            : LayerStyle(offsetX: -parallax * away, dim: clamp01(dim * away))
    case .fade:
        return LayerStyle(opacity: clamp01(1 - away))
    case let .scale(front, back):
        return role.isFront
            ? LayerStyle(scale: 1 + (front - 1) * away, opacity: clamp01(1 - away))
            : LayerStyle(scale: 1 + (back - 1) * away)
    case .none:
        return role.isEnter ? .rest : LayerStyle(opacity: 0)
    }
}

/// The entry under a Back gesture and the one it removes, at `progress`. `finish`
/// runs from 0 to 1 after a commit.
func backStyles(_ back: BackGestureStyle, progress: CGFloat, finish: CGFloat = 0) -> (under: LayerStyle, over: LayerStyle) {
    let p = clamp01(progress)
    let travel = p + (1 - p) * clamp01(finish)
    switch back {
    case let .swipe(parallax, dim):
        return (LayerStyle(offsetX: -parallax * (1 - travel), dim: clamp01(dim * (1 - travel))), LayerStyle(offsetX: travel))
    case .fade:
        return (.rest, LayerStyle(opacity: 1 - travel))
    case .none:
        return (.rest, .rest)
    }
}

/// The initial velocity of a spring that settles a gesture from `progress` to
/// `target`, given the finger's speed in progress per second. SwiftUI measures it
/// as a share of the distance per second.
func settleVelocity(progress: CGFloat, target: CGFloat, speed: CGFloat) -> CGFloat {
    let distance = target - progress
    guard abs(distance) > 0.0001 else { return 0 }
    return speed / distance
}

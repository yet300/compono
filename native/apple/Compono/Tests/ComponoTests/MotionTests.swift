import SwiftUI
import XCTest

@testable import Compono

/// One row of `crates/compono-motion/golden/motion.csv`.
private struct Row {
    let preset, preference, section, role: String
    let x: Double
    let offsetX, offsetY, scale, opacity, cornerRadius, dim: CGFloat

    var style: LayerStyle { LayerStyle(offsetX: offsetX, scale: scale, opacity: opacity, dim: dim) }
    var atRest: Bool { style == .rest && offsetY == 0 && cornerRadius == 0 }
}

private func golden() throws -> [Row] {
    var root = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 { root.deleteLastPathComponent() }
    let file = root.appendingPathComponent("crates/compono-motion/golden/motion.csv")
    let text = try String(contentsOf: file, encoding: .utf8)
    return text.split(separator: "\n").dropFirst().map { line in
        let c = line.split(separator: ",").map(String.init)
        let n = c.dropFirst(4).map { Double($0) ?? .nan }
        return Row(
            preset: c[0], preference: c[1], section: c[2], role: c[3], x: n[0],
            offsetX: n[1], offsetY: n[2], scale: n[3], opacity: n[4], cornerRadius: n[5], dim: n[6]
        )
    }
}

private func role(_ name: String) -> LayerRole {
    switch name {
    case "enter_front": .enterFront
    case "exit_front": .exitFront
    case "enter_back": .enterBack
    default: .exitBack
    }
}

private func assertClose(_ expected: LayerStyle, _ actual: LayerStyle, _ what: String, line: UInt = #line) {
    let pairs = [
        (expected.offsetX, actual.offsetX), (expected.scale, actual.scale),
        (expected.opacity, actual.opacity), (expected.dim, actual.dim),
    ]
    for (e, a) in pairs {
        XCTAssertEqual(e, a, accuracy: 1e-3, "\(what): expected \(expected), got \(actual)", line: line)
    }
}

final class MotionTests: XCTestCase {
    private var rows: [Row] = []

    override func setUpWithError() throws {
        rows = try golden().filter { $0.preset == "cupertino" && $0.preference != "off" }
        XCTAssertGreaterThan(rows.count, 100)
    }

    /// The progress of each preference's timing, through SwiftUI's own curves.
    private func progress(_ preference: String, at seconds: Double) -> CGFloat {
        if preference == "reduced" {
            let curve = UnitCurve.bezier(startControlPoint: UnitPoint(x: 0.2, y: 0), endControlPoint: UnitPoint(x: 0, y: 1))
            return CGFloat(curve.value(at: min(seconds / 0.15, 1)))
        }
        return CGFloat(platformSpring.value(target: 1.0, initialVelocity: 0, time: seconds))
    }

    func testPushAndPopMatchTheCupertinoPreset() {
        let moves = rows.filter { $0.section == "push" || $0.section == "pop" }
        let groups = Dictionary(grouping: moves) { "\($0.preference) \($0.section) \($0.x)" }
        for (what, group) in groups {
            guard let first = group.first else { continue }
            let animation = first.preference == "reduced" ? StackAnimation.platform.reduced() : .platform
            let p = progress(first.preference, at: first.x)
            if first.x > 0, group.allSatisfy(\.atRest) {
                // The table shows a finished transition at rest; SwiftUI's is done too.
                XCTAssertEqual(p, 1, accuracy: 1e-3, what)
                continue
            }
            for row in group {
                assertClose(row.style, layerStyle(animation.transition, role: role(row.role), progress: p), "\(what) \(row.role)")
            }
        }
    }

    func testTheSwipeBackMatchesTheCupertinoPreset() {
        let gestures = rows.filter { $0.section.hasPrefix("back") }
        XCTAssertFalse(gestures.isEmpty)
        for row in gestures {
            let back = row.preference == "reduced" ? StackAnimation.platform.reduced().back : StackAnimation.platform.back
            // A swipe ignores the edge and the touch's height.
            let (progress, finish): (CGFloat, CGFloat) = switch row.section {
            case "back", "back_right": (row.x, 0)
            case "back_finish": (0.5, row.x)
            default: (0.5, 0)
            }
            let styles = backStyles(back, progress: progress, finish: finish)
            assertClose(row.style, row.role == "under" ? styles.under : styles.over, "\(row.preference) \(row.section) \(row.x) \(row.role)")
        }
    }

    func testReducedMotionFadesInPlace() {
        let reduced = StackAnimation.platform.reduced()
        XCTAssertEqual(reduced.transition, .fade)
        XCTAssertEqual(reduced.back, .fade)
        XCTAssertEqual(StackAnimation.fade.reduced().transition, .fade)
        XCTAssertEqual(StackAnimation.none.reduced().transition, .none)
    }

    func testTheReleaseVelocityIsAShareOfTheDistanceLeft() {
        // Released at 0.6 moving 2 widths per second: 0.4 to go, so 5 per second.
        XCTAssertEqual(settleVelocity(progress: 0.6, target: 1, speed: 2), 5, accuracy: 1e-6)
        // Cancelled at 0.5 while moving back at 1 width per second.
        XCTAssertEqual(settleVelocity(progress: 0.5, target: 0, speed: -1), 2, accuracy: 1e-6)
        XCTAssertEqual(settleVelocity(progress: 1, target: 1, speed: 3), 0)
    }
}

import Foundation
import Observation
import XCTest

@testable import Compono

@MainActor
private final class FakeBridge: TreeBridge {
    var pumps = 0
    var phases: [Int32] = []
    var live = Set<Int64>()

    func pump() { pumps += 1 }
    func setPhase(code: Int32) { phases.append(code) }
    func back() -> Bool { true }
    var gestureTarget: Int64 = 7
    var cancels = 0
    var commits = 0
    func backStart() -> Int64 { gestureTarget }
    func backCancel() { cancels += 1 }
    func backCommit() -> Bool { commits += 1; return true }
    func contains(node: Int64) -> Bool { live.contains(node) }
    func save() -> Data { Data([1, 2]) }
}

/// Bindings that keep their listener alive, as generated bindings do: they own the
/// Rust tree, which holds the listener.
@MainActor
private final class OwningBridge: TreeBridge {
    let listener: ComponoTree
    init(listener: ComponoTree) { self.listener = listener }
    func pump() {}
    func setPhase(code: Int32) {}
    func back() -> Bool { false }
    func backStart() -> Int64 { -1 }
    func backCancel() {}
    func backCommit() -> Bool { false }
    func contains(node: Int64) -> Bool { false }
    func save() -> Data { Data() }
}

@MainActor
private func fires(_ body: () -> Void, after change: () -> Void) -> Bool {
    final class Flag: @unchecked Sendable { var fired = false }
    let flag = Flag()
    withObservationTracking(body) { flag.fired = true }
    change()
    return flag.fired
}

@MainActor
final class ComponoTreeTests: XCTestCase {
    func testAChangeReachesOnlyViewsOfThatComponent() {
        let tree = ComponoTree()
        XCTAssertTrue(fires({ tree.observe(1) }, after: { tree.commit(dirty: [1], canBack: false) }))
        XCTAssertFalse(fires({ tree.observe(1) }, after: { tree.commit(dirty: [2], canBack: false) }))
    }

    func testCanBackIsObservableAndUnchangedValuesAreQuiet() {
        let tree = ComponoTree()
        XCTAssertTrue(fires({ _ = tree.canBack }, after: { tree.commit(dirty: [], canBack: true) }))
        XCTAssertFalse(fires({ _ = tree.canBack }, after: { tree.commit(dirty: [], canBack: true) }))
    }

    func testManyPumpRequestsBecomeOne() async {
        let tree = ComponoTree()
        let bridge = FakeBridge()
        tree.attach(bridge)
        for _ in 0..<100 { tree.requestPump() }
        await Task.yield()
        let done = expectation(description: "main queue drained")
        DispatchQueue.main.async { done.fulfill() }
        await fulfillment(of: [done], timeout: 5)
        XCTAssertEqual(bridge.pumps, 1)
        tree.requestPump()
        let again = expectation(description: "main queue drained again")
        DispatchQueue.main.async { again.fulfill() }
        await fulfillment(of: [again], timeout: 5)
        XCTAssertEqual(bridge.pumps, 2)
    }

    func testDestroyedComponentsAreForgotten() {
        let tree = ComponoTree()
        let bridge = FakeBridge()
        tree.attach(bridge)
        bridge.live = [1]
        for node in 1...300 { tree.observe(Int64(node)) }
        XCTAssertEqual(tree.trackedNodes, 300)
        tree.commit(dirty: [], canBack: false)
        XCTAssertEqual(tree.trackedNodes, 1)
    }

    func testAGestureWithNoStackEndsAtOnceOnEitherSide() {
        let tree = ComponoTree()
        let bridge = FakeBridge()
        tree.attach(bridge)
        XCTAssertTrue(tree.beginGesture())
        XCTAssertEqual(tree.gesture?.target, 7)
        tree.finishGesture(commit: false)
        XCTAssertNil(tree.gesture)
        XCTAssertEqual(bridge.cancels, 1)
        XCTAssertTrue(tree.beginGesture())
        tree.finishGesture(commit: true)
        XCTAssertEqual(bridge.commits, 1)
    }

    func testAGestureNeedsAHandlerAndProgressStaysInRange() {
        let tree = ComponoTree()
        let bridge = FakeBridge()
        tree.attach(bridge)
        bridge.gestureTarget = -1
        XCTAssertFalse(tree.beginGesture())
        bridge.gestureTarget = 7
        XCTAssertTrue(tree.beginGesture())
        tree.updateGesture(progress: 3)
        XCTAssertEqual(tree.gesture?.progress, 1)
        tree.updateGesture(progress: -1)
        XCTAssertEqual(tree.gesture?.progress, 0)
    }

    func testBackDuringASettlingGestureEndsItAndStartsAnother() {
        let tree = ComponoTree()
        let bridge = FakeBridge()
        tree.attach(bridge)
        tree.registerStack(7)
        var finishAnimation: (@MainActor () -> Void)?
        tree.settle = { _, done in finishAnimation = done }
        XCTAssertTrue(tree.beginGesture())
        tree.finishGesture(commit: true)
        XCTAssertNotNil(tree.gesture, "a gesture on a stack animates to its end")
        XCTAssertTrue(tree.beginGesture(), "a swipe while settling is not dropped")
        XCTAssertEqual(bridge.commits, 1, "the settling gesture committed first")
        finishAnimation?()
        XCTAssertEqual(bridge.commits, 1, "the old animation ends nothing again")
        XCTAssertEqual(tree.gesture?.target, 7, "the new gesture is still in progress")
    }

    func testReleasingTheRootFreesTheTreeAndItsBindings() {
        weak var tree: ComponoTree?
        weak var app: OwningBridge?
        do {
            let root = ComponoRoot(stateFile: nil) { tree, _ in OwningBridge(listener: tree) }
            tree = root.tree
            app = root.app
        }
        XCTAssertNil(app, "the bindings outlived their root")
        XCTAssertNil(tree, "the tree outlived its root")
    }

    func testACompletedGestureIsReportedToItsStackOnce() {
        let tree = ComponoTree()
        let bridge = FakeBridge()
        tree.attach(bridge)
        withExtendedLifetime(bridge) {
            _ = tree.beginGesture()
            tree.finishGesture(commit: true)
            XCTAssertTrue(tree.takeCommitted(7))
            XCTAssertFalse(tree.takeCommitted(7), "a later change is an ordinary transition")
        }
    }

    func testTheWindowCoversEveryRowOnScreenWithSpareRowsAndRepeatsNothing() {
        let tracker = WindowTracker()
        tracker.configure(count: 100, overscan: 2)
        var windows: [Range<Int>] = []
        tracker.report = { windows.append($0) }
        tracker.appear(3)
        tracker.appear(5)
        tracker.flush()
        tracker.flush()
        tracker.disappear(3)
        tracker.flush()
        tracker.appear(20)
        tracker.flush()
        XCTAssertEqual(windows, [1..<8, 3..<23], "rows inside the window report nothing")
    }
}

@MainActor
final class ComponoRootTests: XCTestCase {
    func testARootReadsTheStateItSavedAndConnectsItsBindings() throws {
        let file = FileManager.default.temporaryDirectory.appending(path: "compono-root-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: file) }
        var given: [Data] = []
        let first = ComponoRoot(stateFile: file) { _, saved in
            given.append(saved)
            return FakeBridge()
        }
        XCTAssertEqual(given, [Data()], "a first start has no saved state")
        first.persist(Data([7, 8]))
        _ = ComponoRoot(stateFile: file) { _, saved in
            given.append(saved)
            return FakeBridge()
        }
        XCTAssertEqual(given.last, Data([7, 8]))
        first.tree.setPhase(2)
        XCTAssertEqual(first.app.phases, [2], "the tree reaches the bindings it was given")
    }

    func testTheTreeIsTheBindingsListener() {
        let root = ComponoRoot(stateFile: nil) { _, _ in FakeBridge() }
        root.tree.commit(dirty: [], canBack: true)
        XCTAssertTrue(root.tree.canBack)
    }
}

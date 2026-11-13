import XCTest

@testable import Compono

/// Keys are visits, numbered from 0 as a Rust `Stack` numbers them.
final class StackDirectionTests: XCTestCase {
    func testAPushMovesForward() {
        // [0] -> [0, 1]: the old entry stays parked.
        XCTAssertEqual(stackDirection(shown: 0, active: 1, below: nil, shownRemains: true), .forward)
    }

    func testAPopMovesBack() {
        // [0, 1] -> [0]: the top entry is destroyed.
        XCTAssertEqual(stackDirection(shown: 1, active: 0, below: 0, shownRemains: false), .back)
    }

    func testPoppingSeveralEntriesMovesBack() {
        // [0, 1, 2] -> [0]
        XCTAssertEqual(stackDirection(shown: 2, active: 0, below: 1, shownRemains: false), .back)
    }

    func testBringingAnOlderEntryToFrontMovesForward() {
        // [0, 1, 2] -> [1, 2, 0]: the shown entry stays parked.
        XCTAssertEqual(stackDirection(shown: 2, active: 0, below: 1, shownRemains: true), .forward)
        XCTAssertEqual(stackDirection(shown: 2, active: 1, below: 1, shownRemains: true), .forward)
    }

    func testAReplaceMovesForward() {
        // [0, 1] -> [0, 2]: the old top is destroyed, the new one is a new visit.
        XCTAssertEqual(stackDirection(shown: 1, active: 2, below: 0, shownRemains: false), .forward)
    }

    func testAPopAfterBringToFrontMovesBack() {
        // [0, 1, 2] -> bring 0 to front -> [1, 2, 0] -> pop -> [1, 2].
        XCTAssertEqual(stackDirection(shown: 0, active: 2, below: 2, shownRemains: false), .back)
    }
}

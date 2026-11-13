import XCTest

@testable import Compono

final class RowWindowTests: XCTestCase {
    func testAFirstReportAddsSpareRowsOnEachSide() {
        XCTAssertEqual(nextWindow(visible: 20..<30, current: nil, overscan: 10, count: 1000), 10..<40)
    }

    func testSpareRowsStopAtTheEndsOfTheList() {
        XCTAssertEqual(nextWindow(visible: 0..<5, current: nil, overscan: 10, count: 12), 0..<12)
        XCTAssertEqual(nextWindow(visible: 995..<1000, current: nil, overscan: 10, count: 1000), 985..<1000)
    }

    func testScrollingInsideTheWindowReportsNothing() {
        XCTAssertNil(nextWindow(visible: 15..<25, current: 10..<40, overscan: 10, count: 1000))
    }

    func testLeavingTheWindowMovesItWithTheSpareRows() {
        XCTAssertEqual(nextWindow(visible: 35..<45, current: 10..<40, overscan: 10, count: 1000), 25..<55)
    }
}

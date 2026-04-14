import AppKit
import XCTest
@testable import Trace

final class CapturePanelControllerTests: XCTestCase {
    func testDefaultPanelCollectionBehaviorSupportsCurrentFullScreenSpace() {
        let behavior = CapturePanelController.defaultPanelCollectionBehavior

        XCTAssertTrue(behavior.contains(.canJoinAllSpaces))
        XCTAssertTrue(behavior.contains(.fullScreenAuxiliary))
        XCTAssertTrue(behavior.contains(.moveToActiveSpace))
    }
}

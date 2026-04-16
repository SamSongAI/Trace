import AppKit
import XCTest
@testable import Trace

final class CapturePanelControllerTests: XCTestCase {
    func testDefaultPanelCollectionBehaviorSupportsCurrentFullScreenSpace() {
        let behavior = CapturePanelController.defaultPanelCollectionBehavior

        // .moveToActiveSpace and .canJoinAllSpaces are mutually exclusive;
        // combining them causes -[NSWindow _validateCollectionBehavior:] to
        // throw an exception on macOS 13+.  We use .moveToActiveSpace so the
        // panel follows the user to their active Space.
        XCTAssertTrue(behavior.contains(.moveToActiveSpace))
        XCTAssertTrue(behavior.contains(.fullScreenAuxiliary))
        XCTAssertFalse(behavior.contains(.canJoinAllSpaces),
                       ".canJoinAllSpaces conflicts with .moveToActiveSpace")
    }
}

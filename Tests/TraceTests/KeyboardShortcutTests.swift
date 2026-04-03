import Carbon
import XCTest
@testable import Trace

final class KeyboardShortcutTests: XCTestCase {
    func testDisplayLabelUsesExpectedGlyphs() {
        let shortcut = KeyboardShortcut(
            keyCode: UInt32(kVK_ANSI_N),
            modifiers: UInt32(cmdKey | shiftKey)
        )

        XCTAssertEqual(shortcut.displayLabel, "⇧⌘N")
    }

    func testSanitizedModifiersDropUnsupportedBits() {
        let noisyModifiers = UInt32(cmdKey | shiftKey | alphaLock)
        let shortcut = KeyboardShortcut(
            keyCode: UInt32(kVK_ANSI_N),
            modifiers: noisyModifiers
        )

        XCTAssertEqual(shortcut.modifiers, UInt32(cmdKey | shiftKey))
    }

    func testDisplayLabelSupportsShiftTabShortcut() {
        let shortcut = KeyboardShortcut(
            keyCode: UInt32(kVK_Tab),
            modifiers: UInt32(shiftKey)
        )

        XCTAssertEqual(shortcut.displayLabel, "⇧Tab")
    }

    func testReservedSectionSwitchDetection() {
        let reserved = KeyboardShortcut(
            keyCode: UInt32(kVK_ANSI_9),
            modifiers: UInt32(cmdKey | shiftKey)
        )
        let free = KeyboardShortcut(
            keyCode: UInt32(kVK_ANSI_9),
            modifiers: UInt32(optionKey)
        )

        XCTAssertTrue(reserved.isReservedSectionSwitch)
        XCTAssertFalse(free.isReservedSectionSwitch)
    }
}

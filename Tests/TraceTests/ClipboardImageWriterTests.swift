import AppKit
import Foundation
import XCTest
@testable import Trace

final class ClipboardImageWriterTests: XCTestCase {
    func testSaveFromPasteboardImageWritesPngAndReturnsMarkdownLink() throws {
        let tempDir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let fixedDate = makeDate(year: 2026, month: 3, day: 3, hour: 23, minute: 50)
        let settings = ClipboardTestSettings(
            vaultPath: tempDir.path,
            dailyFolderName: "Daily",
            hasValidVaultPath: true
        )
        let writer = ClipboardImageWriter(settings: settings)

        let markdown = try writer.saveFromPasteboardImage(makeTestImage(), now: fixedDate)
        XCTAssertTrue(markdown.hasPrefix("![image](assets/"))
        XCTAssertTrue(markdown.hasSuffix(".png)"))

        let relativePath = extractRelativePath(fromMarkdownImage: markdown)
        let targetURL = tempDir
            .appendingPathComponent("Daily", isDirectory: true)
            .appendingPathComponent(relativePath, isDirectory: false)
        XCTAssertTrue(FileManager.default.fileExists(atPath: targetURL.path))
    }

    func testSaveFromPasteboardImageThrowsWhenVaultIsInvalid() {
        let settings = ClipboardTestSettings(
            vaultPath: "",
            dailyFolderName: "Daily",
            hasValidVaultPath: false
        )
        let writer = ClipboardImageWriter(settings: settings)

        XCTAssertThrowsError(try writer.saveFromPasteboardImage(makeTestImage())) { error in
            XCTAssertEqual(error as? ClipboardImageWriterError, .invalidVaultPath)
        }
    }

    private func extractRelativePath(fromMarkdownImage markdown: String) -> String {
        guard let openParen = markdown.firstIndex(of: "("),
              let closeParen = markdown.firstIndex(of: ")"),
              openParen < closeParen else {
            XCTFail("Invalid markdown image syntax: \(markdown)")
            return ""
        }

        return String(markdown[markdown.index(after: openParen)..<closeParen])
    }

    private func makeDate(year: Int, month: Int, day: Int, hour: Int, minute: Int) -> Date {
        var components = DateComponents()
        components.calendar = Calendar(identifier: .gregorian)
        components.timeZone = TimeZone(secondsFromGMT: 0)
        components.year = year
        components.month = month
        components.day = day
        components.hour = hour
        components.minute = minute
        components.second = 0
        return components.date!
    }

    private func makeTestImage() -> NSImage {
        let width = 4
        let height = 4
        let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: width,
            pixelsHigh: height,
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        )!

        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
        NSColor.systemBlue.setFill()
        NSBezierPath(rect: NSRect(x: 0, y: 0, width: width, height: height)).fill()
        NSGraphicsContext.restoreGraphicsState()

        let image = NSImage(size: NSSize(width: width, height: height))
        image.addRepresentation(rep)
        return image
    }
}

private struct ClipboardTestSettings: ClipboardImageWritingSettingsProviding {
    let vaultPath: String
    let dailyFolderName: String
    let imageAssetsFolderName: String = "assets"
    let hasValidVaultPath: Bool
}

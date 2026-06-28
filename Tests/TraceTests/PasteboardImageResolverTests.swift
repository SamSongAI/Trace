import AppKit
import XCTest
@testable import Trace

final class PasteboardImageResolverTests: XCTestCase {
    func testResolveFromCustomPasteboardTypeUsingPNGData() throws {
        let item = NSPasteboardItem()
        item.setData(try makePNGData(), forType: NSPasteboard.PasteboardType("com.tencent.wechat.image"))

        let image = PasteboardImageResolver.resolve(from: item)

        XCTAssertNotNil(image)
        XCTAssertEqual(image?.size.width, 6)
        XCTAssertEqual(image?.size.height, 6)
    }

    func testResolveFromQtImageTypeUsingTIFFData() throws {
        let item = NSPasteboardItem()
        item.setData(try makeTIFFData(), forType: NSPasteboard.PasteboardType("com.trolltech.anymime.application--x-qt-image"))

        let image = PasteboardImageResolver.resolve(
            from: item,
            preferredType: NSPasteboard.PasteboardType("com.trolltech.anymime.application--x-qt-image")
        )

        XCTAssertNotNil(image)
        XCTAssertEqual(image?.size.width, 6)
        XCTAssertEqual(image?.size.height, 6)
    }

    func testResolveSkipsPlainTextPayloads() {
        let item = NSPasteboardItem()
        item.setString("not an image", forType: .string)

        let image = PasteboardImageResolver.resolve(from: item)

        XCTAssertNil(image)
    }

    // MARK: - resolveAll tests

    func testResolveAllFromMultipleItems() throws {
        let item1 = NSPasteboardItem()
        item1.setData(try makePNGData(), forType: .png)
        let item2 = NSPasteboardItem()
        item2.setData(try makePNGData(), forType: .png)

        let images = PasteboardImageResolver.resolveAll(from: [item1, item2])

        XCTAssertEqual(images.count, 2)
    }

    func testResolveAllFromSingleItem() throws {
        let item = NSPasteboardItem()
        item.setData(try makePNGData(), forType: .png)

        let images = PasteboardImageResolver.resolveAll(from: [item])

        XCTAssertEqual(images.count, 1)
    }

    func testResolveAllFromEmptyItemsArray() {
        let images = PasteboardImageResolver.resolveAll(from: [NSPasteboardItem]())

        XCTAssertTrue(images.isEmpty)
    }

    private func makePNGData() throws -> Data {
        let rep = try makeBitmapImageRep()
        return try XCTUnwrap(rep.representation(using: .png, properties: [:]))
    }

    private func makeTIFFData() throws -> Data {
        let rep = try makeBitmapImageRep()
        return try XCTUnwrap(rep.tiffRepresentation)
    }

    private func makeBitmapImageRep() throws -> NSBitmapImageRep {
        let width = 6
        let height = 6
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
        NSColor.systemOrange.setFill()
        NSBezierPath(rect: NSRect(x: 0, y: 0, width: width, height: height)).fill()
        NSGraphicsContext.restoreGraphicsState()
        return rep
    }
}

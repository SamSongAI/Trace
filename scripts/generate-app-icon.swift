#!/usr/bin/env swift

import AppKit
import Foundation

struct IconSpec {
    let fileName: String
    let pixels: Int
}

private let appIconCanvasInsetRatio: CGFloat = 11.0 / 128.0
private let horizontalInsetRatio: CGFloat = 0.05
private let topInsetRatio: CGFloat = 0.05
private let bottomInsetRatio: CGFloat = 0.05
private let contentBoundsPaddingRatio: CGFloat = 0.02
private let alphaThreshold: UInt8 = 5
private let backgroundDistanceThreshold: Int = 26
private let cornerRadiusRatio: CGFloat = 0.2
private let appBackgroundColor = NSColor(
    srgbRed: 0.08,
    green: 0.08,
    blue: 0.08,
    alpha: 1
)

let args = CommandLine.arguments

guard args.count == 3 else {
    fputs("usage: generate-app-icon.swift <image-path> <iconset-dir>\n", stderr)
    exit(1)
}

let imageURL = URL(fileURLWithPath: args[1])
let iconsetURL = URL(fileURLWithPath: args[2], isDirectory: true)

guard let sourceImage = NSImage(contentsOf: imageURL) else {
    fputs("failed to load image at \(imageURL.path)\n", stderr)
    exit(1)
}

let specs: [IconSpec] = [
    .init(fileName: "icon_16x16.png", pixels: 16),
    .init(fileName: "icon_16x16@2x.png", pixels: 32),
    .init(fileName: "icon_32x32.png", pixels: 32),
    .init(fileName: "icon_32x32@2x.png", pixels: 64),
    .init(fileName: "icon_128x128.png", pixels: 128),
    .init(fileName: "icon_128x128@2x.png", pixels: 256),
    .init(fileName: "icon_256x256.png", pixels: 256),
    .init(fileName: "icon_256x256@2x.png", pixels: 512),
    .init(fileName: "icon_512x512.png", pixels: 512),
    .init(fileName: "icon_512x512@2x.png", pixels: 1024)
]

try FileManager.default.createDirectory(at: iconsetURL, withIntermediateDirectories: true)

func makeBitmap(size: Int) -> NSBitmapImageRep? {
    NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: size,
        pixelsHigh: size,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )
}

func makeSourceBitmap(from image: NSImage) -> NSBitmapImageRep? {
    let width = max(Int(image.size.width), 1)
    let height = max(Int(image.size.height), 1)

    guard let bitmap = NSBitmapImageRep(
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
    ) else {
        return nil
    }

    bitmap.size = NSSize(width: width, height: height)

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    NSColor.clear.setFill()
    NSBezierPath(rect: NSRect(x: 0, y: 0, width: width, height: height)).fill()
    image.draw(
        in: NSRect(x: 0, y: 0, width: width, height: height),
        from: NSRect(origin: .zero, size: image.size),
        operation: .sourceOver,
        fraction: 1
    )
    NSGraphicsContext.restoreGraphicsState()

    return bitmap
}

struct PixelSample {
    let red: UInt8
    let green: UInt8
    let blue: UInt8
    let alpha: UInt8
}

func pixelSample(
    bitmap: NSBitmapImageRep,
    x: Int,
    y: Int
) -> PixelSample? {
    guard let data = bitmap.bitmapData,
          x >= 0, y >= 0,
          x < bitmap.pixelsWide,
          y < bitmap.pixelsHigh else {
        return nil
    }

    let bytesPerRow = bitmap.bytesPerRow
    let bytesPerPixel = max(bitmap.bitsPerPixel / 8, 4)
    let offset = y * bytesPerRow + x * bytesPerPixel

    return PixelSample(
        red: data[offset],
        green: data[offset + 1],
        blue: data[offset + 2],
        alpha: data[offset + 3]
    )
}

func averageCornerBackground(bitmap: NSBitmapImageRep) -> PixelSample {
    let maxX = max(bitmap.pixelsWide - 1, 0)
    let maxY = max(bitmap.pixelsHigh - 1, 0)
    let inset = min(max(min(bitmap.pixelsWide, bitmap.pixelsHigh) / 32, 1), 24)

    let points = [
        (x: inset, y: inset),
        (x: max(maxX - inset, 0), y: inset),
        (x: inset, y: max(maxY - inset, 0)),
        (x: max(maxX - inset, 0), y: max(maxY - inset, 0))
    ]

    var redTotal = 0
    var greenTotal = 0
    var blueTotal = 0
    var alphaTotal = 0
    var sampleCount = 0

    for point in points {
        guard let sample = pixelSample(bitmap: bitmap, x: point.x, y: point.y) else {
            continue
        }

        redTotal += Int(sample.red)
        greenTotal += Int(sample.green)
        blueTotal += Int(sample.blue)
        alphaTotal += Int(sample.alpha)
        sampleCount += 1
    }

    guard sampleCount > 0 else {
        return PixelSample(red: 0, green: 0, blue: 0, alpha: 0)
    }

    return PixelSample(
        red: UInt8(redTotal / sampleCount),
        green: UInt8(greenTotal / sampleCount),
        blue: UInt8(blueTotal / sampleCount),
        alpha: UInt8(alphaTotal / sampleCount)
    )
}

func isBackgroundPixel(
    sample: PixelSample,
    background: PixelSample
) -> Bool {
    if sample.alpha <= alphaThreshold {
        return true
    }

    let redDistance = abs(Int(sample.red) - Int(background.red))
    let greenDistance = abs(Int(sample.green) - Int(background.green))
    let blueDistance = abs(Int(sample.blue) - Int(background.blue))
    let alphaDistance = abs(Int(sample.alpha) - Int(background.alpha))

    return redDistance <= backgroundDistanceThreshold
        && greenDistance <= backgroundDistanceThreshold
        && blueDistance <= backgroundDistanceThreshold
        && alphaDistance <= backgroundDistanceThreshold
}

func contentRect(for bitmap: NSBitmapImageRep, imageSize: NSSize) -> NSRect {
    guard let data = bitmap.bitmapData else {
        return NSRect(origin: .zero, size: imageSize)
    }

    let background = averageCornerBackground(bitmap: bitmap)
    let bytesPerRow = bitmap.bytesPerRow
    let bytesPerPixel = max(bitmap.bitsPerPixel / 8, 4)
    let width = bitmap.pixelsWide
    let height = bitmap.pixelsHigh

    var minX = width
    var minY = height
    var maxX = -1
    var maxY = -1

    for y in 0..<height {
        for x in 0..<width {
            let offset = y * bytesPerRow + x * bytesPerPixel
            let red = data[offset]
            let green = data[offset + 1]
            let blue = data[offset + 2]
            let alpha = data[offset + 3]

            if isBackgroundPixel(
                sample: PixelSample(red: red, green: green, blue: blue, alpha: alpha),
                background: background
            ) {
                continue
            }

            minX = min(minX, x)
            minY = min(minY, y)
            maxX = max(maxX, x)
            maxY = max(maxY, y)
        }
    }

    guard maxX >= minX, maxY >= minY else {
        return NSRect(origin: .zero, size: imageSize)
    }

    let scaleX = imageSize.width / CGFloat(width)
    let scaleY = imageSize.height / CGFloat(height)

    let rect = NSRect(
        x: CGFloat(minX) * scaleX,
        y: CGFloat(minY) * scaleY,
        width: CGFloat(maxX - minX + 1) * scaleX,
        height: CGFloat(maxY - minY + 1) * scaleY
    )

    let padding = max(rect.width, rect.height) * contentBoundsPaddingRatio
    let expandedRect = rect.insetBy(dx: -padding, dy: -padding)
    let imageBounds = NSRect(origin: .zero, size: imageSize)

    return expandedRect.intersection(imageBounds)
}

func fittedRect(for sourceRect: NSRect, in backgroundRect: NSRect) -> NSRect {
    let leftInset = backgroundRect.width * horizontalInsetRatio
    let rightInset = backgroundRect.width * horizontalInsetRatio
    let topInset = backgroundRect.height * topInsetRatio
    let bottomInset = backgroundRect.height * bottomInsetRatio
    let availableWidth = backgroundRect.width - leftInset - rightInset
    let availableHeight = backgroundRect.height - topInset - bottomInset
    let scale = min(availableWidth / sourceRect.width, availableHeight / sourceRect.height)
    let targetWidth = sourceRect.width * scale
    let targetHeight = sourceRect.height * scale

    return NSRect(
        x: backgroundRect.minX + leftInset + (availableWidth - targetWidth) / 2,
        y: backgroundRect.minY + bottomInset + (availableHeight - targetHeight) / 2,
        width: targetWidth,
        height: targetHeight
    )
}

guard let sourceBitmap = makeSourceBitmap(from: sourceImage) else {
    fputs("failed to rasterize image at \(imageURL.path)\n", stderr)
    exit(1)
}

let sourceContentRect = contentRect(for: sourceBitmap, imageSize: sourceImage.size)

for spec in specs {
    guard let bitmap = makeBitmap(size: spec.pixels) else {
        fputs("failed to create bitmap for \(spec.fileName)\n", stderr)
        exit(1)
    }

    bitmap.size = NSSize(width: spec.pixels, height: spec.pixels)

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    NSColor.clear.setFill()
    NSBezierPath(rect: NSRect(x: 0, y: 0, width: spec.pixels, height: spec.pixels)).fill()

    let canvasRect = NSRect(x: 0, y: 0, width: spec.pixels, height: spec.pixels)
    let canvasInset = CGFloat(spec.pixels) * appIconCanvasInsetRatio
    let backgroundRect = canvasRect.insetBy(dx: canvasInset, dy: canvasInset)
    let cornerRadius = backgroundRect.width * cornerRadiusRatio
    let backgroundPath = NSBezierPath(
        roundedRect: backgroundRect,
        xRadius: cornerRadius,
        yRadius: cornerRadius
    )
    appBackgroundColor.setFill()
    backgroundPath.fill()
    backgroundPath.addClip()

    let destinationRect = fittedRect(for: sourceContentRect, in: backgroundRect)
    sourceImage.draw(
        in: destinationRect,
        from: sourceContentRect,
        operation: .sourceOver,
        fraction: 1
    )
    NSGraphicsContext.restoreGraphicsState()

    guard let pngData = bitmap.representation(using: .png, properties: [:]) else {
        fputs("failed to encode png for \(spec.fileName)\n", stderr)
        exit(1)
    }

    try pngData.write(to: iconsetURL.appendingPathComponent(spec.fileName))
}

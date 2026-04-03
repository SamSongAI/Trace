import AppKit
import Foundation

private enum BrandLogoVariant {
    case darkBackground
    case lightBackground

    var resourceName: String {
        switch self {
        case .darkBackground:
            return "trace-logo-light"
        case .lightBackground:
            return "trace-logo-dark"
        }
    }
}

enum BrandAssets {
    static let displayName = "Trace"
    static let slogan = "Thought is leverage, Leave a trace."

    static func menuBarLogo(size: CGFloat = 18) -> NSImage? {
        makeMenuBarTemplateLogo(size: size)
    }

    static func headerLogo(
        for preset: AppThemePreset,
        size: CGFloat = 18
    ) -> NSImage? {
        themedLogo(variant: logoVariant(for: preset), size: size)
            ?? makeResizedBrandImage(size: size)
    }

    static func appIcon(size: CGFloat = 128) -> NSImage? {
        if let bundledIcon = bundledTraceIcon() {
            return bundledIcon
        }

        return makeResizedBrandImage(size: size)
    }

    private static func makeResizedBrandImage(size: CGFloat) -> NSImage? {
        let source = bundledTraceIcon()
            ?? moduleLogo()

        guard let source else {
            return nil
        }

        return resizedImage(from: source, size: size)
    }

    private static func themedLogo(
        variant: BrandLogoVariant,
        size: CGFloat
    ) -> NSImage? {
        guard let source = bundledPNG(named: variant.resourceName) else {
            return nil
        }

        return resizedImage(from: source, size: size)
    }

    private static func makeMenuBarTemplateLogo(size: CGFloat) -> NSImage? {
        let targetSize = NSSize(width: size, height: size)
        let image = NSImage(size: targetSize)
        image.lockFocus()

        NSColor.clear.setFill()
        NSBezierPath(rect: NSRect(origin: .zero, size: targetSize)).fill()

        NSColor.black.setFill()

        let topBarWidth = size * 0.78
        let topBarHeight = max(size * 0.18, 3)
        let stemWidth = max(size * 0.2, 3.6)
        let stemHeight = size * 0.5
        let topBarX = (size - topBarWidth) / 2
        let topBarY = size * 0.66
        let stemX = (size - stemWidth) / 2
        let stemY = size * 0.16

        NSBezierPath(
            roundedRect: NSRect(
                x: topBarX,
                y: topBarY,
                width: topBarWidth,
                height: topBarHeight
            ),
            xRadius: 1,
            yRadius: 1
        ).fill()

        NSBezierPath(
            roundedRect: NSRect(
                x: stemX,
                y: stemY,
                width: stemWidth,
                height: stemHeight
            ),
            xRadius: 1,
            yRadius: 1
        ).fill()

        image.unlockFocus()
        image.isTemplate = true
        return image
    }

    private static func bundledTraceIcon() -> NSImage? {
        // NSImage(named:) checks CFBundleIconFile in Info.plist automatically
        if let named = NSImage(named: "Trace") {
            return named
        }

        let candidates: [URL?] = [
            Bundle.main.url(forResource: "Trace", withExtension: "icns"),
            Bundle.main.resourceURL?.appendingPathComponent("Trace.icns"),
            Bundle.module.url(forResource: "Trace", withExtension: "icns")
        ]

        for candidate in candidates {
            guard let candidate,
                  let image = NSImage(contentsOf: candidate) else {
                continue
            }

            return image
        }

        return nil
    }

    private static func bundledPNG(named name: String) -> NSImage? {
        let candidates: [URL?] = [
            Bundle.module.url(forResource: name, withExtension: "png"),
            Bundle.main.url(forResource: name, withExtension: "png"),
            Bundle.main.resourceURL?.appendingPathComponent("\(name).png")
        ]

        for candidate in candidates {
            guard let candidate,
                  let image = NSImage(contentsOf: candidate) else {
                continue
            }

            return image
        }

        return nil
    }

    private static func moduleLogo() -> NSImage? {
        if let svgURL = Bundle.module.url(forResource: "trace-app-icon", withExtension: "svg"),
           let svgImage = NSImage(contentsOf: svgURL) {
            return svgImage
        }

        if let pngURL = Bundle.module.url(forResource: "logo", withExtension: "png"),
           let pngImage = NSImage(contentsOf: pngURL) {
            return pngImage
        }

        guard let svgURL = Bundle.module.url(forResource: "lava-mark", withExtension: "svg"),
              let svgImage = NSImage(contentsOf: svgURL) else {
            return nil
        }

        return svgImage
    }

    private static func logoVariant(for appearance: NSAppearance) -> BrandLogoVariant {
        let bestMatch = appearance.bestMatch(from: [.darkAqua, .aqua])
        return bestMatch == .darkAqua ? .darkBackground : .lightBackground
    }

    private static func logoVariant(for preset: AppThemePreset) -> BrandLogoVariant {
        switch preset {
        case .dark:
            return .darkBackground
        case .light, .paper, .dune:
            return .lightBackground
        }
    }

    private static func resizedImage(from source: NSImage, size: CGFloat) -> NSImage {
        let targetSize = NSSize(width: size, height: size)
        let image = NSImage(size: targetSize)
        image.lockFocus()
        source.draw(
            in: NSRect(origin: .zero, size: targetSize),
            from: NSRect(origin: .zero, size: source.size),
            operation: .copy,
            fraction: 1
        )
        image.unlockFocus()
        image.isTemplate = false
        return image
    }
}

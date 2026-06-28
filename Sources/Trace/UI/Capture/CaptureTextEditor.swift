import AppKit
import CryptoKit
import SwiftUI
import UniformTypeIdentifiers

struct CaptureTextEditor: NSViewRepresentable {
    struct Theme {
        let textColor: NSColor
        let placeholderColor: NSColor
        let insertionPointColor: NSColor
        let editorFont: NSFont
    }

    @Binding var text: String
    @Binding var isFocused: Bool
    let placeholder: String
    let theme: Theme
    let onPasteImages: (([NSImage]) -> [String])?

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text, isFocused: $isFocused, onPasteImages: onPasteImages)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.hasVerticalScroller = false
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.contentInsets = NSEdgeInsets(top: 0, left: 0, bottom: 0, right: 0)

        let textView = PlaceholderTextView()
        textView.delegate = context.coordinator
        textView.drawsBackground = false
        textView.isRichText = false
        textView.importsGraphics = false
        textView.allowsUndo = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticDataDetectionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.textContainerInset = NSSize(width: 16, height: 12)
        textView.textContainer?.lineFragmentPadding = 0
        textView.textContainer?.containerSize = NSSize(width: 0, height: CGFloat.greatestFiniteMagnitude)
        textView.textContainer?.widthTracksTextView = true
        textView.font = theme.editorFont
        textView.textColor = theme.textColor
        textView.insertionPointColor = theme.insertionPointColor
        textView.placeholder = placeholder
        textView.placeholderColor = theme.placeholderColor
        textView.placeholderFont = theme.editorFont
        textView.string = text
        textView.onPasteImages = { images, textView in
            context.coordinator.handlePastedImages(images, in: textView)
        }
        registerForDraggedTypes(in: textView)

        scrollView.documentView = textView
        return scrollView
    }

    func updateNSView(_ nsView: NSScrollView, context: Context) {
        guard let textView = nsView.documentView as? PlaceholderTextView else { return }

        context.coordinator.syncTextViewIfNeeded(textView, with: text)

        if textView.placeholder != placeholder {
            textView.placeholder = placeholder
        }
        textView.placeholderColor = theme.placeholderColor
        textView.textColor = theme.textColor
        textView.insertionPointColor = theme.insertionPointColor
        textView.font = theme.editorFont
        textView.placeholderFont = theme.editorFont


        if isFocused,
           nsView.window?.firstResponder !== textView,
           !textView.isComposingMarkedText {
            DispatchQueue.main.async { [weak textView] in
                guard let textView else { return }
                guard !textView.isComposingMarkedText else { return }
                textView.window?.makeFirstResponder(textView)
            }
        }
    }

    private func registerForDraggedTypes(in textView: NSView) {
        textView.registerForDraggedTypes([
            .png,
            .tiff,
            .pdf,
            NSPasteboard.PasteboardType.fileURL
        ])
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        private let text: Binding<String>
        private let isFocused: Binding<Bool>
        private let onPasteImages: (([NSImage]) -> [String])?
        private var isApplyingExternalText = false
        private var pendingExternalText: String?

        init(
            text: Binding<String>,
            isFocused: Binding<Bool>,
            onPasteImages: (([NSImage]) -> [String])?
        ) {
            self.text = text
            self.isFocused = isFocused
            self.onPasteImages = onPasteImages
        }

        func syncTextViewIfNeeded(_ textView: NSTextView, with updatedText: String) {
            guard textView.string != updatedText else {
                pendingExternalText = nil
                return
            }

            // Keep IME composition stable unless we are explicitly clearing text.
            if textView.isComposingMarkedText, !updatedText.isEmpty {
                pendingExternalText = updatedText
                return
            }

            applyExternalText(updatedText, to: textView)
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            guard !isApplyingExternalText else { return }

            if text.wrappedValue != textView.string {
                text.wrappedValue = textView.string
            }

            guard let pendingExternalText,
                  !textView.isComposingMarkedText,
                  textView.string != pendingExternalText else { return }
            applyExternalText(pendingExternalText, to: textView)
        }

        func textDidBeginEditing(_ notification: Notification) {
            isFocused.wrappedValue = true
        }

        func textDidEndEditing(_ notification: Notification) {
            if let textView = notification.object as? NSTextView,
               let pendingExternalText,
               !textView.isComposingMarkedText,
               textView.string != pendingExternalText {
                applyExternalText(pendingExternalText, to: textView)
            }
            isFocused.wrappedValue = false
        }

        func handlePastedImages(_ images: [NSImage], in textView: NSTextView) {
            guard !images.isEmpty else {
                NSSound.beep()
                return
            }

            let markdowns: [String]
            if let onPasteImages {
                markdowns = onPasteImages(images)
            } else {
                NSSound.beep()
                return
            }

            guard !markdowns.isEmpty else {
                NSSound.beep()
                return
            }

            let insertion = markdowns.map { $0 + "\n" }.joined()
            textView.insertText(insertion, replacementRange: textView.selectedRange())

            if text.wrappedValue != textView.string {
                text.wrappedValue = textView.string
            }
        }

        private func applyExternalText(_ updatedText: String, to textView: NSTextView) {
            let previousSelection = textView.selectedRange()
            let textLength = (updatedText as NSString).length
            let clampedLocation = min(previousSelection.location, textLength)
            let maxLength = max(textLength - clampedLocation, 0)
            let clampedSelection = NSRange(
                location: clampedLocation,
                length: min(previousSelection.length, maxLength)
            )

            isApplyingExternalText = true
            textView.string = updatedText
            textView.setSelectedRange(clampedSelection)
            isApplyingExternalText = false
            pendingExternalText = nil
        }
    }
}

private enum CaptureLoraFonts {
    static let editor: NSFont = NSFont(name: "Lora-Regular", size: 15)
        ?? NSFont(name: "Lora", size: 15)
        ?? NSFont.systemFont(ofSize: 15, weight: .regular)
}

private final class PlaceholderTextView: NSTextView {
    var placeholder: String = "" {
        didSet { needsDisplay = true }
    }

    var placeholderColor: NSColor = .placeholderTextColor {
        didSet { needsDisplay = true }
    }

    var placeholderFont: NSFont = NSFont.systemFont(ofSize: 15) {
        didSet { needsDisplay = true }
    }

    var onPasteImages: (([NSImage], NSTextView) -> Void)?

    override var string: String {
        didSet { needsDisplay = true }
    }

    override func didChangeText() {
        super.didChangeText()
        needsDisplay = true
    }

    override func paste(_ sender: Any?) {
        if handleImagePaste(from: NSPasteboard.general) {
            return
        }

        super.paste(sender)
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if modifiers == .command, event.charactersIgnoringModifiers?.lowercased() == "v" {
            paste(nil)
            return true
        }
        return super.performKeyEquivalent(with: event)
    }

    override func readSelection(from pboard: NSPasteboard) -> Bool {
        if handleImagePaste(from: pboard) {
            return true
        }

        return super.readSelection(from: pboard)
    }

    override func readSelection(from pboard: NSPasteboard, type pasteboardType: NSPasteboard.PasteboardType) -> Bool {
        if handleImagePaste(from: pboard, preferredType: pasteboardType) {
            return true
        }

        return super.readSelection(from: pboard, type: pasteboardType)
    }

    // MARK: - Drag and Drop

    override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation {
        let pasteboard = sender.draggingPasteboard
        let hasImages = PasteboardImageResolver.resolveAll(from: pasteboard).count > 0
        return hasImages ? .copy : NSDragOperation()
    }

    override func draggingUpdated(_ sender: NSDraggingInfo) -> NSDragOperation {
        let pasteboard = sender.draggingPasteboard
        let hasImages = PasteboardImageResolver.resolveAll(from: pasteboard).count > 0
        return hasImages ? .copy : NSDragOperation()
    }

    override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
        let pasteboard = sender.draggingPasteboard
        let images = PasteboardImageResolver.resolveAll(from: pasteboard)
        guard !images.isEmpty else { return false }
        onPasteImages?(images, self)
        return true
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)

        guard string.isEmpty, !placeholder.isEmpty else { return }

        let linePadding = textContainer?.lineFragmentPadding ?? 0
        let x = textContainerInset.width + linePadding
        let y = textContainerInset.height
        let placeholderRect = NSRect(
            x: x,
            y: y,
            width: bounds.width - x - textContainerInset.width,
            height: bounds.height - y - textContainerInset.height
        )

        let paragraphStyle = NSMutableParagraphStyle()
        paragraphStyle.lineBreakMode = .byWordWrapping

        let attributes: [NSAttributedString.Key: Any] = [
            .font: placeholderFont,
            .foregroundColor: placeholderColor,
            .paragraphStyle: paragraphStyle
        ]
        (placeholder as NSString).draw(in: placeholderRect, withAttributes: attributes)
    }

    private func handleImagePaste(
        from pasteboard: NSPasteboard,
        preferredType: NSPasteboard.PasteboardType? = nil
    ) -> Bool {
        let images: [NSImage]
        if let preferredType,
           let items = pasteboard.pasteboardItems {
            images = items.compactMap { item in
                if item.types.contains(preferredType) {
                    return PasteboardImageResolver.resolve(from: item, preferredType: preferredType)
                }
                return nil
            }
        } else {
            images = PasteboardImageResolver.resolveAll(from: pasteboard)
        }

        guard !images.isEmpty else {
            return false
        }

        onPasteImages?(images, self)
        return true
    }
}

enum PasteboardImageResolver {
    private static let standardImageTypes: [NSPasteboard.PasteboardType] = [
        .png,
        .tiff,
        .pdf
    ]

    private static let skippedTextLikeTypes: Set<String> = [
        NSPasteboard.PasteboardType.string.rawValue,
        NSPasteboard.PasteboardType.html.rawValue,
        NSPasteboard.PasteboardType.rtf.rawValue,
        NSPasteboard.PasteboardType.rtfd.rawValue,
        NSPasteboard.PasteboardType.tabularText.rawValue,
        NSPasteboard.PasteboardType.fileURL.rawValue
    ]

    static func resolve(from pasteboard: NSPasteboard) -> NSImage? {
        resolveAll(from: pasteboard).first
    }

    static func resolveAll(from pasteboard: NSPasteboard) -> [NSImage] {
        var images: [NSImage] = []
        var seen = Set<String>()

        // 1. readObjects for NSImage — may return multiple
        if let nsImages = pasteboard.readObjects(forClasses: [NSImage.self], options: nil) as? [NSImage] {
            for image in nsImages {
                let key = imageKey(image)
                if !seen.contains(key) {
                    seen.insert(key)
                    images.append(image)
                }
            }
        }

        // 2. Per-item resolution for custom pasteboard types
        if let items = pasteboard.pasteboardItems {
            for item in items {
                if let image = resolve(from: item) {
                    let key = imageKey(image)
                    if !seen.contains(key) {
                        seen.insert(key)
                        images.append(image)
                    }
                }
            }
        }

        // 3. File URLs — may be multiple image files
        if images.isEmpty {
            let fileURLOptions: [NSPasteboard.ReadingOptionKey: Any] = [
                .urlReadingFileURLsOnly: true
            ]
            if let urls = pasteboard.readObjects(
                forClasses: [NSURL.self],
                options: fileURLOptions
            ) as? [URL] {
                for url in urls {
                    if let image = NSImage(contentsOf: url) {
                        let key = imageKey(image)
                        if !seen.contains(key) {
                            seen.insert(key)
                            images.append(image)
                        }
                    }
                }
            }
        }

        // 4. Direct pasteboard-level type check (handles macOS screenshot clipboard)
        if images.isEmpty {
            for type in standardImageTypes {
                if let data = pasteboard.data(forType: type),
                   !data.isEmpty,
                   let image = NSImage(data: data) {
                    images.append(image)
                    break
                }
            }
        }

        // 5. Fallback: single image via NSImage(pasteboard:)
        if images.isEmpty, let image = NSImage(pasteboard: pasteboard) {
            images.append(image)
        }

        // 6. HTML image extraction (handles WeChat articles, web pages)
        if images.isEmpty {
            if let htmlImages = resolveImagesFromHTML(in: pasteboard) {
                images.append(contentsOf: htmlImages)
            }
        }

        return images
    }

    private static func imageKey(_ image: NSImage) -> String {
        if let tiff = image.tiffRepresentation {
            return SHA256.hash(data: tiff).compactMap { String(format: "%02x", $0) }.joined()
        }
        return UUID().uuidString
    }

    static func resolve(from items: [NSPasteboardItem]) -> NSImage? {
        for item in items {
            if let image = resolve(from: item) {
                return image
            }
        }

        return nil
    }

    static func resolveAll(from items: [NSPasteboardItem]) -> [NSImage] {
        items.compactMap { resolve(from: $0) }
    }

    static func resolve(from item: NSPasteboardItem) -> NSImage? {
        resolve(from: item, preferredType: nil)
    }

    static func resolve(
        from item: NSPasteboardItem,
        preferredType: NSPasteboard.PasteboardType?
    ) -> NSImage? {
        for type in prioritizedTypes(from: item.types, preferredType: preferredType) {
            guard let data = item.data(forType: type),
                  !data.isEmpty,
                  let image = NSImage(data: data) else {
                continue
            }

            return image
        }

        return nil
    }

    private static func prioritizedTypes(
        from types: [NSPasteboard.PasteboardType],
        preferredType: NSPasteboard.PasteboardType?
    ) -> [NSPasteboard.PasteboardType] {
        var orderedTypes: [NSPasteboard.PasteboardType] = []

        if let preferredType, types.contains(preferredType) {
            orderedTypes.append(preferredType)
        }

        let preferred = types.filter { type in
            !orderedTypes.contains(type) && (standardImageTypes.contains(type) || conformsToImage(type))
        }

        let fallback = types.filter { type in
            !orderedTypes.contains(type) && !preferred.contains(type) && !skippedTextLikeTypes.contains(type.rawValue)
        }

        orderedTypes.append(contentsOf: preferred)
        orderedTypes.append(contentsOf: fallback)
        return orderedTypes
    }

    private static func conformsToImage(_ type: NSPasteboard.PasteboardType) -> Bool {
        guard let utType = UTType(type.rawValue) else {
            return false
        }

        return utType.conforms(to: .image)
    }

    private static func resolveImagesFromHTML(in pasteboard: NSPasteboard) -> [NSImage]? {
        let htmlType = NSPasteboard.PasteboardType.html
        guard let htmlData = pasteboard.data(forType: htmlType),
              !htmlData.isEmpty,
              let html = String(data: htmlData, encoding: .utf8) ?? String(data: htmlData, encoding: .unicode) else {
            return nil
        }

        let imgURLs = extractImageURLs(from: html)
        guard !imgURLs.isEmpty else { return nil }

        var images: [NSImage] = []
        for urlString in imgURLs {
            guard let url = URL(string: urlString) else { continue }
            if let data = try? Data(contentsOf: url), let image = NSImage(data: data) {
                images.append(image)
            }
        }
        return images.isEmpty ? nil : images
    }

    private static func extractImageURLs(from html: String) -> [String] {
        let pattern = #"<img[^>]+src\s*=\s*["']([^"']+)["']"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return []
        }
        let range = NSRange(html.startIndex..., in: html)
        let matches = regex.matches(in: html, options: [], range: range)
        return matches.compactMap { match in
            guard let urlRange = Range(match.range(at: 1), in: html) else { return nil }
            return String(html[urlRange])
        }
    }
}

extension CaptureTextEditor.Theme {
    static let obsidianLight = CaptureTextEditor.Theme(
        textColor: NSColor(srgbRed: 31.0 / 255.0, green: 35.0 / 255.0, blue: 40.0 / 255.0, alpha: 1),
        placeholderColor: NSColor(srgbRed: 167.0 / 255.0, green: 159.0 / 255.0, blue: 188.0 / 255.0, alpha: 1),
        insertionPointColor: NSColor(srgbRed: 31.0 / 255.0, green: 35.0 / 255.0, blue: 40.0 / 255.0, alpha: 1),
        editorFont: CaptureLoraFonts.editor
    )

    static let obsidianDark = CaptureTextEditor.Theme(
        textColor: NSColor(srgbRed: 236.0 / 255.0, green: 233.0 / 255.0, blue: 246.0 / 255.0, alpha: 1),
        placeholderColor: NSColor(srgbRed: 141.0 / 255.0, green: 132.0 / 255.0, blue: 166.0 / 255.0, alpha: 1),
        insertionPointColor: NSColor(srgbRed: 160.0 / 255.0, green: 121.0 / 255.0, blue: 255.0 / 255.0, alpha: 1),
        editorFont: CaptureLoraFonts.editor
    )
}

private extension NSTextView {
    var isComposingMarkedText: Bool {
        let marked = markedRange()
        return marked.location != NSNotFound
    }
}

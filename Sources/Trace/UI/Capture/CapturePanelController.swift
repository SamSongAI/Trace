import AppKit
import Combine
import Carbon
import SwiftUI

final class CapturePanel: NSPanel {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { true }
}

final class CapturePanelController: NSObject, NSWindowDelegate {
    private let settings: AppSettings
    private let writer: DailyNoteWriter
    private lazy var clipboardImageWriter = ClipboardImageWriter(settings: settings)

    private let viewModel = CaptureViewModel()
    private let minimumPanelSize = NSSize(width: 360, height: 240)
    private var panel: CapturePanel?
    private var localKeyMonitor: Any?
    private var previousFrontmostApplication: NSRunningApplication?
    private var cancellables: Set<AnyCancellable> = []

    init(settings: AppSettings, writer: DailyNoteWriter) {
        self.settings = settings
        self.writer = writer
        super.init()
        bindPinnedBehavior()
        bindSectionState()
    }

    func toggle() {
        if panel?.isVisible == true {
            hide()
        } else {
            show()
        }
    }

    func presentFromGlobalHotKey() {
        if let panel, panel.isVisible {
            NSApp.activate(ignoringOtherApps: true)
            panel.makeKeyAndOrderFront(nil)
            NotificationCenter.default.post(name: .traceFocusInput, object: nil)
            return
        }

        show()
    }

    func show() {
        let panel = panel ?? buildPanel()
        self.panel = panel

        rememberFrontmostApplicationBeforeShowing()
        viewModel.selectedSection = settings.defaultSection
        viewModel.selectedThread = settings.defaultThread
        applySavedFrameIfNeeded(on: panel)

        NSApp.activate(ignoringOtherApps: true)
        panel.makeKeyAndOrderFront(nil)
        NotificationCenter.default.post(name: .traceFocusInput, object: nil)
    }

    func hide(restoreDelay: TimeInterval = 0) {
        guard let panel else { return }
        settings.savePanelFrame(panel.frame)
        viewModel.pinned = false
        panel.orderOut(nil)
        restorePreviousFrontmostApplicationIfNeeded(after: restoreDelay)
    }

    private func buildPanel() -> CapturePanel {
        let initialFrame = NSRect(x: 0, y: 0, width: 440, height: 520)
        let panel = CapturePanel(
            contentRect: initialFrame,
            styleMask: [.fullSizeContentView, .resizable],
            backing: .buffered,
            defer: false
        )

        panel.titleVisibility = .hidden
        panel.titlebarAppearsTransparent = true
        panel.isMovableByWindowBackground = true
        panel.isFloatingPanel = true
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.hidesOnDeactivate = !viewModel.pinned
        panel.delegate = self

        panel.standardWindowButton(.closeButton)?.isHidden = true
        panel.standardWindowButton(.miniaturizeButton)?.isHidden = true
        panel.standardWindowButton(.zoomButton)?.isHidden = true

        let rootView = CaptureView(
            viewModel: viewModel,
            settings: settings,
            onPasteImage: { [weak self] image in
                self?.markdownForPastedImage(image)
            }
        )

        panel.contentView = NSHostingView(rootView: rootView)
        setupLocalKeyMonitor()

        return panel
    }

    private func setupLocalKeyMonitor() {
        if let localKeyMonitor {
            NSEvent.removeMonitor(localKeyMonitor)
            self.localKeyMonitor = nil
        }

        localKeyMonitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown]) { [weak self] event -> NSEvent? in
            guard let self,
                  let panel,
                  panel.isVisible,
                  NSApp.keyWindow === panel else {
                return event
            }

            if event.keyCode == UInt16(kVK_Escape) {
                hide()
                return nil
            }

            let modifierFlags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)

            if matchesShortcut(
                event,
                keyCode: settings.modeToggleKeyCode,
                modifiers: settings.modeToggleModifiers
            ) {
                settings.noteWriteMode = settings.noteWriteMode.next()
                NotificationCenter.default.post(name: .traceFocusInput, object: nil)
                return nil
            }

            if matchesShortcut(
                event,
                keyCode: settings.appendNoteKeyCode,
                modifiers: settings.appendNoteModifiers
            ) {
                saveCurrentNote(mode: .appendToLatestEntry)
                return nil
            }

            if matchesShortcut(
                event,
                keyCode: settings.sendNoteKeyCode,
                modifiers: settings.sendNoteModifiers
            ) {
                saveCurrentNote(mode: .createNewEntry)
                return nil
            }

            if modifierFlags.contains(.command), let key = event.charactersIgnoringModifiers {
                if key.lowercased() == "p", modifierFlags == .command {
                    viewModel.pinned.toggle()
                    return nil
                }
                // Thread mode: Command+number switches threads
                if settings.noteWriteMode == .thread {
                    if let thread = selectThread(forShortcutKey: key) {
                        viewModel.selectedThread = thread
                        return nil
                    }
                } else {
                    // Daily mode: Command+number switches sections
                    if let section = selectSection(forShortcutKey: key) {
                        viewModel.selectedSection = section
                        return nil
                    }
                }
                return event
            }

            return event
        }
    }

    private func applySavedFrameIfNeeded(on panel: NSPanel) {
        if let savedFrame = settings.savedPanelFrame() {
            panel.setFrame(validatedFrame(savedFrame), display: false)
            return
        }

        if let visibleFrame = NSScreen.main?.visibleFrame ?? NSScreen.screens.first?.visibleFrame {
            panel.setFrame(frameByCentering(panel.frame, in: visibleFrame), display: false)
        } else {
            panel.center()
        }
    }

    private func rememberFrontmostApplicationBeforeShowing() {
        guard let frontmostApplication = NSWorkspace.shared.frontmostApplication else {
            previousFrontmostApplication = nil
            return
        }

        if frontmostApplication.processIdentifier == ProcessInfo.processInfo.processIdentifier {
            previousFrontmostApplication = nil
            return
        }

        previousFrontmostApplication = frontmostApplication
    }

    private func restorePreviousFrontmostApplicationIfNeeded(after delay: TimeInterval = 0) {
        guard let previousFrontmostApplication else { return }
        self.previousFrontmostApplication = nil

        let restore = {
            _ = previousFrontmostApplication.activate(options: [.activateIgnoringOtherApps])
        }

        if delay > 0 {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: restore)
        } else {
            restore()
        }
    }

    private func validatedFrame(_ frame: NSRect) -> NSRect {
        var validatedFrame = frame.standardized
        validatedFrame.size.width = max(validatedFrame.size.width, minimumPanelSize.width)
        validatedFrame.size.height = max(validatedFrame.size.height, minimumPanelSize.height)

        let visibleFrames = NSScreen.screens.map(\.visibleFrame)
        guard !visibleFrames.isEmpty else { return validatedFrame }

        if let bestVisibleFrame = bestVisibleFrame(for: validatedFrame, in: visibleFrames) {
            return frameByClamping(validatedFrame, to: bestVisibleFrame)
        }

        let fallbackVisibleFrame = NSScreen.main?.visibleFrame ?? visibleFrames[0]
        return frameByCentering(validatedFrame, in: fallbackVisibleFrame)
    }

    private func bestVisibleFrame(for frame: NSRect, in visibleFrames: [NSRect]) -> NSRect? {
        var bestVisibleFrame: NSRect?
        var bestIntersectionArea: CGFloat = 0

        for visibleFrame in visibleFrames {
            let area = intersectionArea(frame, visibleFrame)
            if area > bestIntersectionArea {
                bestIntersectionArea = area
                bestVisibleFrame = visibleFrame
            }
        }

        return bestIntersectionArea > 0 ? bestVisibleFrame : nil
    }

    private func frameByClamping(_ frame: NSRect, to bounds: NSRect) -> NSRect {
        var clampedFrame = frame
        clampedFrame.size.width = min(clampedFrame.size.width, bounds.width)
        clampedFrame.size.height = min(clampedFrame.size.height, bounds.height)

        clampedFrame.origin.x = min(max(clampedFrame.origin.x, bounds.minX), bounds.maxX - clampedFrame.width)
        clampedFrame.origin.y = min(max(clampedFrame.origin.y, bounds.minY), bounds.maxY - clampedFrame.height)

        return clampedFrame
    }

    private func frameByCentering(_ frame: NSRect, in bounds: NSRect) -> NSRect {
        var centeredFrame = frame
        centeredFrame.size.width = min(centeredFrame.size.width, bounds.width)
        centeredFrame.size.height = min(centeredFrame.size.height, bounds.height)

        centeredFrame.origin.x = bounds.midX - centeredFrame.width / 2
        centeredFrame.origin.y = bounds.midY - centeredFrame.height / 2

        return centeredFrame
    }

    private func intersectionArea(_ lhs: NSRect, _ rhs: NSRect) -> CGFloat {
        let intersection = lhs.intersection(rhs)
        guard !intersection.isNull else { return 0 }
        return intersection.width * intersection.height
    }

    private func saveCurrentNote(mode: DailyNoteSaveMode = .createNewEntry) {
        // Flush any in-progress IME composition before reading text.
        panel?.makeFirstResponder(nil)

        let trimmedText = viewModel.text.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !trimmedText.isEmpty else {
            viewModel.showToast(L10n.emptyNotSaved)
            return
        }

        // Validate thread selection for thread mode
        if settings.noteWriteMode == .thread && viewModel.selectedThread == nil {
            viewModel.showToast(L10n.noThreadSelected)
            return
        }

        do {
            let targetSection: NoteSection = settings.noteWriteMode == .dimension ? viewModel.selectedSection : .note
            try writer.save(
                text: trimmedText,
                to: targetSection,
                mode: mode,
                documentTitle: viewModel.fileTitle,
                thread: viewModel.selectedThread
            )
            settings.lastUsedSectionIndex = viewModel.selectedSection.index
            if let thread = viewModel.selectedThread {
                settings.setLastUsedThread(thread)
            }
            if viewModel.pinned {
                viewModel.resetInput()
                NotificationCenter.default.post(name: .traceFocusInput, object: nil)
            } else {
                viewModel.resetInput()
                hide(restoreDelay: 0.08)
            }
        } catch {
            showError(error.localizedDescription)
        }
    }

    private func matchesShortcut(_ event: NSEvent, keyCode: UInt32, modifiers: UInt32) -> Bool {
        KeyboardShortcut(keyCode: keyCode, modifiers: modifiers).matches(event)
    }

    private func showError(_ message: String) {
        guard let panel else { return }
        let alert = NSAlert()
        alert.messageText = L10n.saveFailed
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.beginSheetModal(for: panel)
    }

    private func markdownForPastedImage(_ image: NSImage) -> String? {
        do {
            return try clipboardImageWriter.saveFromPasteboardImage(image)
        } catch {
            showError(error.localizedDescription)
            return nil
        }
    }

    private func bindPinnedBehavior() {
        viewModel.$pinned
            .receive(on: RunLoop.main)
            .sink { [weak self] pinned in
                self?.panel?.hidesOnDeactivate = !pinned
            }
            .store(in: &cancellables)
    }

    private func bindSectionState() {
        settings.$sectionTitles
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self else { return }
                self.viewModel.selectedSection = self.settings.resolvedSection(for: self.viewModel.selectedSection)
            }
            .store(in: &cancellables)
    }

    private func selectSection(forShortcutKey key: String) -> NoteSection? {
        guard let shortcutNumber = Int(key), (1...NoteSection.maximumCount).contains(shortcutNumber) else {
            return nil
        }

        return settings.section(atShortcutIndex: shortcutNumber - 1)
    }

    private func selectThread(forShortcutKey key: String) -> ThreadConfig? {
        guard let shortcutNumber = Int(key), shortcutNumber >= 1 else {
            return nil
        }

        let sortedThreads = settings.threadConfigs.sorted(by: { $0.order < $1.order })
        guard sortedThreads.indices.contains(shortcutNumber - 1) else {
            return nil
        }

        return sortedThreads[shortcutNumber - 1]
    }

    func windowDidMove(_ notification: Notification) {
        guard let panel else { return }
        settings.savePanelFrame(panel.frame)
    }

    func windowDidEndLiveResize(_ notification: Notification) {
        guard let panel else { return }
        settings.savePanelFrame(panel.frame)
    }
}

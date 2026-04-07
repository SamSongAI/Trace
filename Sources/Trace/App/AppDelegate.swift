import AppKit
import Combine
import Foundation
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate {
    let settings = AppSettings.shared

    private lazy var writer = DailyNoteWriter(settings: settings)
    private lazy var panelController = CapturePanelController(
        settings: settings,
        writer: writer
    )
    private let hotKeyManager = GlobalHotKeyManager()

    private var statusItem: NSStatusItem?
    private var settingsWindowController: NSWindowController?
    private var cancellables: Set<AnyCancellable> = []
    private var hasShownHotKeyRegistrationAlert = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        if let icon = BrandAssets.appIcon() {
            NSApp.applicationIconImage = icon
        }
        setupStatusItem()
        bindSettings()
        registerCurrentHotKey()

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(openSettingsFromMenu),
            name: .traceOpenSettings,
            object: nil
        )

        DispatchQueue.main.async { [weak self] in
            self?.presentInitialSurface()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        hotKeyManager.unregister()
    }

    func showCapturePanel() {
        panelController.show()
    }

    func showSettingsWindow() {
        openSettingsWindow()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag {
            showCapturePanel()
        }
        return true
    }

    @objc private func toggleCapturePanel() {
        panelController.toggle()
    }

    @objc private func openSettingsFromMenu() {
        openSettingsWindow()
    }

    @objc private func quitApplication() {
        NSApp.terminate(nil)
    }

    private func bindSettings() {
        settings.$hotKeyCode
            .combineLatest(settings.$hotKeyModifiers)
            .sink { [weak self] _, _ in
                self?.registerCurrentHotKey()
            }
            .store(in: &cancellables)
    }

    private func setupStatusItem() {
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        self.statusItem = item

        if let button = item.button {
            button.image = BrandAssets.menuBarLogo()
            button.imagePosition = .imageOnly
            button.title = ""
        }

        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "New Note", action: #selector(toggleCapturePanel), keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Settings…", action: #selector(openSettingsFromMenu), keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Quit \(BrandAssets.displayName)", action: #selector(quitApplication), keyEquivalent: "q"))

        menu.items.forEach { $0.target = self }
        item.menu = menu
    }

    private func registerCurrentHotKey() {
        let success = hotKeyManager.register(
            keyCode: settings.hotKeyCode,
            modifiers: settings.hotKeyModifiers
        ) { [weak self] in
            DispatchQueue.main.async {
                self?.panelController.presentFromGlobalHotKey()
            }
        }

        if !success {
            NSLog("Failed to register global hotkey")
            presentHotKeyRegistrationFailureAlertIfNeeded()
        }
    }

    private func presentInitialSurface() {
        if !settings.hasValidVaultPath {
            openSettingsWindow()
            return
        }

        showCapturePanel()
    }

    private func openSettingsWindow() {
        NSApp.activate(ignoringOtherApps: true)
        presentFallbackSettingsWindow()
    }

    private func presentFallbackSettingsWindow() {
        if settingsWindowController == nil {
            let rootView = SettingsView(settings: settings)
            let hostingController = NSHostingController(rootView: rootView)
            let window = NSWindow(contentViewController: hostingController)
            window.title = "Trace Settings"
            window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
            window.setContentSize(NSSize(width: 760, height: 760))
            window.isReleasedWhenClosed = false
            window.center()
            settingsWindowController = NSWindowController(window: window)
        }

        settingsWindowController?.showWindow(nil)
        settingsWindowController?.window?.makeKeyAndOrderFront(nil)
    }

    private func presentHotKeyRegistrationFailureAlertIfNeeded() {
        guard !hasShownHotKeyRegistrationAlert else { return }
        hasShownHotKeyRegistrationAlert = true

        DispatchQueue.main.async { [weak self] in
            guard let self else { return }

            let alert = NSAlert()
            alert.messageText = L10n.hotkeyRegistrationFailed
            alert.informativeText = L10n.hotkeyConflictMessage
            alert.alertStyle = .warning
            alert.addButton(withTitle: L10n.openSettings)
            alert.addButton(withTitle: L10n.later)

            NSApp.activate(ignoringOtherApps: true)
            let response = alert.runModal()

            if response == .alertFirstButtonReturn {
                self.openSettingsWindow()
            }
        }
    }
}

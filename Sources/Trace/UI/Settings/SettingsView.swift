import AppKit
import Carbon
import SwiftUI

private typealias SettingsPalette = TraceTheme.SettingsPalette

private enum ShortcutTarget {
    case create
    case send
    case append
    case toggleWriteMode

    var name: String {
        switch self {
        case .create:
            return "创建笔记"
        case .send:
            return "发送笔记"
        case .append:
            return "追加上一条"
        case .toggleWriteMode:
            return "切换写入模式"
        }
    }
}

// MARK: - Section Card (simplified)

private struct SectionCard<Content: View>: View {
    let title: String
    let palette: SettingsPalette
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(title)
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(palette.sectionTitle)

            content
        }
        .padding(18)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(palette.cardBackground)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(palette.cardBorder, lineWidth: 1)
        )
    }
}

// MARK: - Row

private struct SettingRow<Content: View>: View {
    let label: String
    let hint: String?
    let palette: SettingsPalette
    @ViewBuilder var content: Content

    init(
        label: String,
        hint: String? = nil,
        palette: SettingsPalette,
        @ViewBuilder content: () -> Content
    ) {
        self.label = label
        self.hint = hint
        self.palette = palette
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                Text(label)
                    .font(.system(size: 11, weight: .semibold))
                    .textCase(.uppercase)
                    .tracking(0.8)
                    .foregroundStyle(palette.rowLabel)

                if let hint {
                    Text(hint)
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(palette.mutedText)
                }
            }

            content
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

// MARK: - Field modifier

private struct SettingsFieldChrome: ViewModifier {
    let palette: SettingsPalette

    func body(content: Content) -> some View {
        content
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(palette.fieldText)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(palette.fieldBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .stroke(palette.fieldBorder, lineWidth: 1)
            )
    }
}

private extension View {
    func settingsFieldChrome(_ palette: SettingsPalette) -> some View {
        modifier(SettingsFieldChrome(palette: palette))
    }
}

// MARK: - Button Styles

private struct SettingsPrimaryButtonStyle: ButtonStyle {
    let palette: SettingsPalette

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(palette.primaryButtonText)
            .padding(.horizontal, 12)
            .padding(.vertical, 7)
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(palette.accentStrong)
            )
            .opacity(configuration.isPressed ? 0.85 : 1)
    }
}

private struct SettingsSecondaryButtonStyle: ButtonStyle {
    let palette: SettingsPalette

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .medium))
            .foregroundStyle(palette.secondaryButtonText)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(palette.secondaryButtonBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .stroke(palette.secondaryButtonBorder, lineWidth: 1)
            )
    }
}

// MARK: - Theme Tile (compact)

private struct ThemePresetTile: View {
    let preset: AppThemePreset
    let isSelected: Bool
    let action: () -> Void

    private var previewTheme: TraceTheme { preset.theme }
    private var palette: SettingsPalette { previewTheme.settings }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: preset.iconName)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(palette.accent)
                    .frame(width: 28, height: 28)
                    .background(
                        RoundedRectangle(cornerRadius: 7, style: .continuous)
                            .fill(palette.chipBackground)
                    )

                Text(preset.title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(palette.sectionTitle)

                Spacer(minLength: 4)

                HStack(spacing: 4) {
                    ForEach(Array(previewTheme.previewSwatches.enumerated()), id: \.offset) { _, swatch in
                        Circle()
                            .fill(swatch)
                            .frame(width: 10, height: 10)
                    }
                }

                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(isSelected ? palette.accentStrong : palette.cardBorder)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(palette.cardBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .stroke(isSelected ? palette.accent : palette.cardBorder, lineWidth: isSelected ? 1.5 : 1)
            )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Write Mode Tile (compact)

private struct WriteModeTile: View {
    let mode: NoteWriteMode
    let isSelected: Bool
    let palette: SettingsPalette
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: mode.iconName)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(isSelected ? palette.primaryButtonText : palette.accent)
                    .frame(width: 28, height: 28)
                    .background(
                        RoundedRectangle(cornerRadius: 7, style: .continuous)
                            .fill(isSelected ? palette.accentStrong : palette.chipBackground)
                    )

                VStack(alignment: .leading, spacing: 1) {
                    Text(mode.compactTitle)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(palette.sectionTitle)

                    Text(mode.destinationTitle)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(palette.mutedText)
                }

                Spacer(minLength: 4)

                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(isSelected ? palette.accentStrong : palette.cardBorder)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(palette.cardBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .stroke(isSelected ? palette.accent : palette.cardBorder, lineWidth: isSelected ? 1.5 : 1)
            )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Main View

struct SettingsView: View {
    @ObservedObject var settings: AppSettings
    @State private var recordingTarget: ShortcutTarget?
    @State private var keyRecorderMonitor: Any?
    @State private var shortcutRecorderMessage: String?

    private var palette: SettingsPalette {
        settings.appTheme.settings
    }

    private var modeToggleLabel: String {
        KeyboardShortcut(
            keyCode: settings.modeToggleKeyCode,
            modifiers: settings.modeToggleModifiers
        ).displayLabel
    }

    private var shellBackground: some View {
        palette.shellMiddle.ignoresSafeArea()
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                // Theme
                SectionCard(title: "主题", palette: palette) {
                    VStack(spacing: 8) {
                        ForEach(AppThemePreset.allCases) { preset in
                            ThemePresetTile(
                                preset: preset,
                                isSelected: settings.appThemePreset == preset
                            ) {
                                settings.appThemePreset = preset
                            }
                        }
                    }
                }

                // Storage
                SectionCard(title: "保存位置", palette: palette) {
                    VStack(spacing: 12) {
                        SettingRow(label: "写入模式", palette: palette) {
                            HStack(spacing: 8) {
                                ForEach(NoteWriteMode.allCases) { mode in
                                    WriteModeTile(
                                        mode: mode,
                                        isSelected: settings.noteWriteMode == mode,
                                        palette: palette
                                    ) {
                                        settings.noteWriteMode = mode
                                    }
                                }
                            }
                        }

                        if settings.noteWriteMode == .dimension {
                            SettingRow(label: "笔记库", hint: "日记保存的文件夹路径", palette: palette) {
                                VStack(alignment: .leading, spacing: 6) {
                                    HStack(spacing: 8) {
                                        TextField("/Users/you/Daily", text: $settings.vaultPath)
                                            .textFieldStyle(.plain)
                                            .settingsFieldChrome(palette)

                                        Button("选择") {
                                            chooseFolderPath(binding: $settings.vaultPath)
                                        }
                                        .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))
                                    }

                                    if let issue = settings.vaultPathValidationIssue {
                                        Text(issue.message)
                                            .font(.system(size: 11, weight: .medium))
                                            .foregroundStyle(palette.warningText)
                                    }
                                }
                            }

                            SettingRow(label: "文件名格式", palette: palette) {
                                Picker("", selection: dailyFileDateFormatBinding) {
                                    ForEach(DailyFileDateFormat.allCases) { format in
                                        Text(format.title).tag(format)
                                    }
                                }
                                .labelsHidden()
                                .pickerStyle(.menu)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .settingsFieldChrome(palette)
                            }

                            SettingRow(label: "条目格式", palette: palette) {
                                Picker("", selection: $settings.dailyEntryThemePreset) {
                                    ForEach(DailyEntryThemePreset.allCases) { preset in
                                        Text(preset.title).tag(preset)
                                    }
                                }
                                .labelsHidden()
                                .pickerStyle(.menu)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .settingsFieldChrome(palette)
                            }
                        }

                        if settings.noteWriteMode == .file {
                            SettingRow(label: "笔记库", hint: "文档保存的文件夹路径", palette: palette) {
                                VStack(alignment: .leading, spacing: 6) {
                                    HStack(spacing: 8) {
                                        TextField("/Users/you/Documents", text: $settings.inboxVaultPath)
                                            .textFieldStyle(.plain)
                                            .settingsFieldChrome(palette)

                                        Button("选择") {
                                            chooseFolderPath(binding: $settings.inboxVaultPath)
                                        }
                                        .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))
                                    }

                                    if let issue = settings.inboxVaultPathValidationIssue {
                                        Text(issue.message)
                                            .font(.system(size: 11, weight: .medium))
                                            .foregroundStyle(palette.warningText)
                                    }
                                }
                            }
                        }
                    }
                }

                // Modules
                SectionCard(title: "快捷分类", palette: palette) {
                    VStack(alignment: .leading, spacing: 8) {
                        ForEach(settings.sections) { section in
                            SectionTitleRow(
                                section: section,
                                settings: settings,
                                palette: palette
                            )
                        }

                        HStack {
                            Button {
                                settings.addSection()
                            } label: {
                                Label("新增模块", systemImage: "plus")
                            }
                            .buttonStyle(SettingsSecondaryButtonStyle(palette: palette))
                            .disabled(!settings.canAddSection)

                            Spacer()

                            Button("保存") {
                                // Resign first responder to commit all pending drafts
                                NSApp.keyWindow?.makeFirstResponder(nil)
                            }
                            .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))
                        }
                    }
                }

                // Shortcuts
                SectionCard(title: "快捷键", palette: palette) {
                    VStack(spacing: 0) {
                        shortcutRow(for: .create)
                        shortcutRow(for: .send)
                        shortcutRow(for: .append)
                        shortcutRow(for: .toggleWriteMode)

                        Divider().overlay(palette.mutedText.opacity(0.15)).padding(.vertical, 6)

                        fixedShortcutRow("Esc", "关闭面板")
                        fixedShortcutRow("⌘P", "固定面板")
                        fixedShortcutRow("⌘1–9", "切换模块")

                        if let shortcutRecorderMessage {
                            Text(shortcutRecorderMessage)
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(palette.warningText)
                                .padding(.top, 6)
                        }
                    }
                }

                // System
                SectionCard(title: "系统", palette: palette) {
                    HStack {
                        Text("开机自启动")
                            .font(.system(size: 13, weight: .medium))
                            .foregroundStyle(palette.sectionTitle)
                        Spacer()
                        Toggle("", isOn: $settings.launchAtLogin)
                            .labelsHidden()
                            .toggleStyle(.switch)
                            .tint(palette.accent)
                    }
                }
            }
            .padding(.horizontal, 24)
            .padding(.top, 16)
            .padding(.bottom, 28)
        }
        .scrollIndicators(.hidden)
        .background(shellBackground)
        .frame(width: 520, height: 720)
        .onDisappear {
            stopRecording(clearMessage: false)
        }
    }

    // MARK: - Shortcut Recorder

    @ViewBuilder
    private func shortcutRow(for target: ShortcutTarget) -> some View {
        let currentShortcut = shortcut(for: target)
        let isRecording = recordingTarget == target

        HStack(spacing: 0) {
            Text(target.name)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(palette.sectionTitle)
                .frame(width: 100, alignment: .leading)

            Text(isRecording ? "按键录制中…" : currentShortcut.displayLabel)
                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                .foregroundStyle(isRecording ? palette.accent : palette.chipText)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .fill(isRecording ? palette.accent.opacity(0.12) : palette.chipBackground)
                )

            Spacer()

            if isRecording {
                Button("取消") { stopRecording() }
                    .font(.system(size: 11, weight: .medium))
                    .buttonStyle(.plain)
                    .foregroundStyle(palette.mutedText)
            } else {
                Button("修改") { toggleRecording(for: target) }
                    .font(.system(size: 11, weight: .medium))
                    .buttonStyle(.plain)
                    .foregroundStyle(palette.accent)
            }
        }
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private func fixedShortcutRow(_ key: String, _ label: String) -> some View {
        HStack(spacing: 0) {
            Text(label)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(palette.mutedText)
                .frame(width: 100, alignment: .leading)

            Text(key)
                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                .foregroundStyle(palette.mutedText.opacity(0.7))
                .padding(.horizontal, 8)
                .padding(.vertical, 4)

            Spacer()
        }
        .padding(.vertical, 4)
        .padding(.vertical, 3)
    }

    // MARK: - Logic

    private func shortcut(for target: ShortcutTarget) -> KeyboardShortcut {
        switch target {
        case .create:
            return KeyboardShortcut(keyCode: settings.hotKeyCode, modifiers: settings.hotKeyModifiers)
        case .send:
            return KeyboardShortcut(keyCode: settings.sendNoteKeyCode, modifiers: settings.sendNoteModifiers)
        case .append:
            return KeyboardShortcut(keyCode: settings.appendNoteKeyCode, modifiers: settings.appendNoteModifiers)
        case .toggleWriteMode:
            return KeyboardShortcut(keyCode: settings.modeToggleKeyCode, modifiers: settings.modeToggleModifiers)
        }
    }

    private func setShortcut(_ shortcut: KeyboardShortcut, for target: ShortcutTarget) {
        switch target {
        case .create:
            settings.hotKeyCode = shortcut.keyCode
            settings.hotKeyModifiers = shortcut.modifiers
        case .send:
            settings.sendNoteKeyCode = shortcut.keyCode
            settings.sendNoteModifiers = shortcut.modifiers
        case .append:
            settings.appendNoteKeyCode = shortcut.keyCode
            settings.appendNoteModifiers = shortcut.modifiers
        case .toggleWriteMode:
            settings.modeToggleKeyCode = shortcut.keyCode
            settings.modeToggleModifiers = shortcut.modifiers
        }
    }

    private func toggleRecording(for target: ShortcutTarget) {
        shortcutRecorderMessage = nil

        if recordingTarget == target {
            stopRecording()
            return
        }

        recordingTarget = target
        installKeyRecorderMonitorIfNeeded()
    }

    private func stopRecording(clearMessage: Bool = true) {
        if let keyRecorderMonitor {
            NSEvent.removeMonitor(keyRecorderMonitor)
            self.keyRecorderMonitor = nil
        }
        recordingTarget = nil
        if clearMessage {
            shortcutRecorderMessage = nil
        }
    }

    private func installKeyRecorderMonitorIfNeeded() {
        guard keyRecorderMonitor == nil else { return }

        keyRecorderMonitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown]) { event in
            guard let target = recordingTarget else {
                return event
            }

            if event.keyCode == UInt16(kVK_Escape) {
                stopRecording()
                return nil
            }

            let candidate = KeyboardShortcut.from(event: event)
            handleRecordedShortcut(candidate, for: target)
            return nil
        }
    }

    private func handleRecordedShortcut(_ candidate: KeyboardShortcut, for target: ShortcutTarget) {
        if !candidate.hasModifier {
            shortcutRecorderMessage = "需要至少一个修饰键（⌘/⇧/⌥/⌃）"
            NSSound.beep()
            return
        }

        if candidate.keyCode == UInt32(kVK_Escape) {
            shortcutRecorderMessage = "Esc 已用于关闭面板"
            NSSound.beep()
            return
        }

        if target != .create && candidate.isReservedSectionSwitch {
            shortcutRecorderMessage = "⌘1–9 已用于切换模块"
            NSSound.beep()
            return
        }

        if let conflict = conflictingShortcutTarget(for: candidate, excluding: target) {
            shortcutRecorderMessage = "与「\(conflict.name)」冲突"
            NSSound.beep()
            return
        }

        setShortcut(candidate, for: target)
        stopRecording()
    }

    private func conflictingShortcutTarget(for candidate: KeyboardShortcut, excluding current: ShortcutTarget) -> ShortcutTarget? {
        for target in [ShortcutTarget.create, .send, .append, .toggleWriteMode] where target != current {
            if shortcut(for: target) == candidate {
                return target
            }
        }
        return nil
    }

    private var dailyFileDateFormatBinding: Binding<DailyFileDateFormat> {
        Binding {
            DailyFileDateFormat.resolved(fromStored: settings.dailyFileDateFormat)
        } set: { newValue in
            settings.dailyFileDateFormat = newValue.rawValue
        }
    }

    private func chooseFolderPath(binding: Binding<String>) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "选择文件夹"

        if panel.runModal() == .OK {
            binding.wrappedValue = panel.url?.path ?? ""
        }
    }

}

// MARK: - Section Title Row (local-state editing to avoid mid-keystroke normalization)

private struct SectionTitleRow: View {
    let section: NoteSection
    @ObservedObject var settings: AppSettings
    let palette: SettingsPalette

    @State private var draft: String = ""
    @FocusState private var isFocused: Bool

    var body: some View {
        HStack(spacing: 8) {
            Text("\(section.displayIndex)")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(palette.mutedText)
                .frame(width: 18)

            TextField("模块名", text: $draft)
                .textFieldStyle(.plain)
                .settingsFieldChrome(palette)
                .focused($isFocused)
                .onSubmit { commitDraft() }
                .onChange(of: isFocused) { focused in
                    if !focused { commitDraft() }
                }

            Button {
                settings.removeSection(section)
            } label: {
                Image(systemName: "minus.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(settings.canRemoveSection ? palette.warningText : palette.mutedText)
            }
            .buttonStyle(.plain)
            .help("删除模块")
            .disabled(!settings.canRemoveSection)
        }
        .onAppear { draft = settings.title(for: section) }
        .onChange(of: settings.sectionTitles) { _ in
            if !isFocused { draft = settings.title(for: section) }
        }
    }

    private func commitDraft() {
        let trimmed = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            draft = settings.title(for: section)
        } else {
            settings.setTitle(trimmed, for: section)
            draft = settings.title(for: section)
        }
    }
}

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

private struct SettingsShellHeader: View {
    let palette: SettingsPalette
    let currentThemeTitle: String
    let currentModeTitle: String

    var body: some View {
        HStack(alignment: .top, spacing: 18) {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 0) {
                    Text("Trace Control")
                        .font(.system(size: 12, weight: .bold, design: .default))
                        .textCase(.uppercase)
                        .tracking(1.8)
                        .foregroundStyle(palette.headerEyebrow)
                }

                Text(BrandAssets.displayName)
                    .font(.system(size: 30, weight: .bold, design: .default))
                    .foregroundStyle(palette.headerTitle)

                Text(BrandAssets.slogan)
                    .font(.system(size: 16, weight: .semibold, design: .default))
                    .foregroundStyle(palette.headerSubtitle)

                Text("应用主题、默认保存路由和快捷键都在这里统一配置。")
                    .font(.system(size: 13, weight: .medium, design: .default))
                    .foregroundStyle(palette.sectionDescription)
            }

            Spacer(minLength: 12)

            VStack(alignment: .trailing, spacing: 10) {
                SettingsHeaderChip(
                    title: "Theme",
                    value: currentThemeTitle,
                    icon: "swatchpalette",
                    palette: palette
                )

                SettingsHeaderChip(
                    title: "Mode",
                    value: currentModeTitle,
                    icon: "arrow.triangle.branch",
                    palette: palette
                )
            }
        }
        .padding(24)
        .background(
            RoundedRectangle(cornerRadius: 26, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [palette.shellPanel, palette.cardBackground],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
        )
        .overlay(
            RoundedRectangle(cornerRadius: 26, style: .continuous)
                .stroke(palette.shellPanelBorder, lineWidth: 1)
        )
        .overlay(alignment: .topLeading) {
            Capsule(style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [palette.accent, palette.accentStrong],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                )
                .frame(width: 120, height: 5)
                .padding(.top, 16)
                .padding(.leading, 20)
        }
        .shadow(color: palette.cardShadow.opacity(0.75), radius: 26, y: 16)
    }
}

private struct SettingsHeaderChip: View {
    let title: String
    let value: String
    let icon: String
    let palette: SettingsPalette

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(palette.accent)
                .frame(width: 30, height: 30)
                .background(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(palette.chipBackground)
                )

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 10, weight: .bold, design: .default))
                    .textCase(.uppercase)
                    .tracking(1.2)
                    .foregroundStyle(palette.mutedText)

                Text(value)
                    .font(.system(size: 13, weight: .semibold, design: .default))
                    .foregroundStyle(palette.headerTitle)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(palette.cardBackground.opacity(0.94))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(palette.cardBorder, lineWidth: 1)
        )
    }
}

private struct SettingsPrimaryButtonStyle: ButtonStyle {
    let palette: SettingsPalette

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .bold, design: .default))
            .foregroundStyle(palette.primaryButtonText)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [palette.accent, palette.accentStrong],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .shadow(color: palette.accentStrong.opacity(configuration.isPressed ? 0.16 : 0.32), radius: 14, y: 8)
            .opacity(configuration.isPressed ? 0.92 : 1)
            .scaleEffect(configuration.isPressed ? 0.99 : 1)
    }
}

private struct SettingsSecondaryButtonStyle: ButtonStyle {
    let palette: SettingsPalette

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .semibold, design: .default))
            .foregroundStyle(palette.secondaryButtonText)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(palette.secondaryButtonBackground.opacity(configuration.isPressed ? 0.8 : 1))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .stroke(palette.secondaryButtonBorder, lineWidth: 1)
            )
    }
}

private struct SettingsFieldChrome: ViewModifier {
    let palette: SettingsPalette
    let verticalPadding: CGFloat

    func body(content: Content) -> some View {
        content
            .font(.system(size: 13, weight: .medium, design: .default))
            .foregroundStyle(palette.fieldText)
            .padding(.horizontal, 12)
            .padding(.vertical, verticalPadding)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(palette.fieldBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .stroke(palette.fieldBorder, lineWidth: 1)
            )
    }
}

private extension View {
    func settingsFieldChrome(_ palette: SettingsPalette, verticalPadding: CGFloat = 9) -> some View {
        modifier(SettingsFieldChrome(palette: palette, verticalPadding: verticalPadding))
    }
}

private extension AppThemePreset {
    var pickerCaption: String {
        switch self {
        case .light:
            return "冷静浅色"
        case .dark:
            return "沉浸深色"
        case .paper:
            return "纸感阅读"
        case .dune:
            return "暖调写作"
        }
    }
}

private struct ThemePresetTile: View {
    let preset: AppThemePreset
    let isSelected: Bool
    let action: () -> Void

    private var previewTheme: TraceTheme {
        preset.theme
    }

    private var palette: SettingsPalette {
        previewTheme.settings
    }

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 12) {
                HStack(alignment: .center, spacing: 10) {
                    Image(systemName: preset.iconName)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(palette.accent)
                        .frame(width: 34, height: 34)
                        .background(
                            RoundedRectangle(cornerRadius: 11, style: .continuous)
                                .fill(palette.chipBackground)
                        )

                    VStack(alignment: .leading, spacing: 3) {
                        Text(preset.title)
                            .font(.system(size: 15, weight: .bold, design: .default))
                            .foregroundStyle(palette.sectionTitle)

                        Text(preset.pickerCaption)
                            .font(.system(size: 11, weight: .semibold, design: .default))
                            .foregroundStyle(palette.mutedText)
                    }

                    Spacer(minLength: 8)

                    Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(isSelected ? palette.accentStrong : palette.cardBorder)
                }

                HStack(spacing: 8) {
                    ForEach(Array(previewTheme.previewSwatches.enumerated()), id: \.offset) { _, swatch in
                        Capsule(style: .continuous)
                            .fill(swatch)
                            .frame(maxWidth: .infinity)
                            .frame(height: 10)
                    }
                }

                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(previewTheme.capture.panelBackground)
                    .frame(height: 38)
                    .overlay(alignment: .topLeading) {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .fill(previewTheme.capture.chromeBackground)
                            .frame(height: 20)
                            .overlay(
                                HStack(spacing: 6) {
                                    Circle().fill(previewTheme.capture.accent).frame(width: 7, height: 7)
                                    Capsule(style: .continuous)
                                        .fill(previewTheme.capture.surface)
                                        .frame(width: 70, height: 7)
                                }
                                .padding(.horizontal, 10),
                                alignment: .leading
                            )
                    }
                    .overlay {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .stroke(previewTheme.capture.border, lineWidth: 1)
                    }
            }
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [palette.cardBackground, palette.shellPanel.opacity(0.66)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .overlay(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .stroke(isSelected ? palette.accent : palette.cardBorder, lineWidth: isSelected ? 1.5 : 1)
            )
            .shadow(
                color: (isSelected ? palette.accentStrong : palette.cardShadow).opacity(isSelected ? 0.2 : 0.5),
                radius: isSelected ? 14 : 10,
                y: 8
            )
        }
        .buttonStyle(.plain)
    }
}

private struct WriteModeTile: View {
    let mode: NoteWriteMode
    let isSelected: Bool
    let palette: SettingsPalette
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 10) {
                HStack(spacing: 10) {
                    Image(systemName: mode.iconName)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(isSelected ? palette.primaryButtonText : palette.accent)
                        .frame(width: 32, height: 32)
                        .background(
                            RoundedRectangle(cornerRadius: 10, style: .continuous)
                                .fill(isSelected ? palette.accentStrong : palette.chipBackground)
                        )

                    VStack(alignment: .leading, spacing: 2) {
                        Text(mode.compactTitle)
                            .font(.system(size: 14, weight: .bold, design: .default))
                            .foregroundStyle(isSelected ? palette.sectionTitle : palette.sectionTitle)

                        Text(mode.destinationTitle)
                            .font(.system(size: 11, weight: .semibold, design: .default))
                            .foregroundStyle(palette.mutedText)
                    }

                    Spacer(minLength: 8)

                    Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                        .font(.system(size: 15, weight: .semibold))
                        .foregroundStyle(isSelected ? palette.accentStrong : palette.cardBorder)
                }

                Text(mode.summary)
                    .font(.system(size: 12, weight: .medium, design: .default))
                    .foregroundStyle(palette.sectionDescription)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [palette.cardBackground, palette.shellPanel.opacity(0.54)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .overlay(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .stroke(isSelected ? palette.accent : palette.cardBorder, lineWidth: isSelected ? 1.5 : 1)
            )
            .shadow(
                color: (isSelected ? palette.accentStrong : palette.cardShadow).opacity(isSelected ? 0.16 : 0.3),
                radius: isSelected ? 14 : 10,
                y: 8
            )
        }
        .buttonStyle(.plain)
    }
}

private struct SettingsCard<Content: View>: View {
    let eyebrow: String?
    let title: String
    let description: String?
    let palette: SettingsPalette
    @ViewBuilder var content: Content

    init(
        eyebrow: String? = nil,
        title: String,
        description: String?,
        palette: SettingsPalette,
        @ViewBuilder content: () -> Content
    ) {
        self.eyebrow = eyebrow
        self.title = title
        self.description = description
        self.palette = palette
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 6) {
                if let eyebrow {
                    Text(eyebrow)
                        .font(.system(size: 11, weight: .bold, design: .default))
                        .textCase(.uppercase)
                        .tracking(1.4)
                        .foregroundStyle(palette.headerEyebrow)
                }

                Text(title)
                    .font(.system(size: 20, weight: .bold, design: .default))
                    .foregroundStyle(palette.sectionTitle)

                if let description {
                    Text(description)
                        .font(.system(size: 13, weight: .medium, design: .default))
                        .foregroundStyle(palette.sectionDescription)
                }
            }

            content
        }
        .padding(20)
        .background(
            RoundedRectangle(cornerRadius: 22, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [palette.cardBackground, palette.shellPanel.opacity(0.72)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .shadow(color: palette.cardShadow.opacity(0.9), radius: 24, y: 14)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 22, style: .continuous)
                .stroke(palette.cardBorder, lineWidth: 1)
        )
        .overlay(alignment: .topLeading) {
            Capsule(style: .continuous)
                .fill(palette.accent.opacity(0.5))
                .frame(width: 88, height: 4)
                .padding(.top, 16)
                .padding(.leading, 20)
        }
    }
}

private struct SettingRow<Content: View>: View {
    let label: String
    let palette: SettingsPalette
    let alignment: VerticalAlignment
    let labelTopPadding: CGFloat
    @ViewBuilder var content: Content

    init(
        label: String,
        palette: SettingsPalette,
        alignment: VerticalAlignment = .center,
        labelTopPadding: CGFloat = 0,
        @ViewBuilder content: () -> Content
    ) {
        self.label = label
        self.palette = palette
        self.alignment = alignment
        self.labelTopPadding = labelTopPadding
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(label)
                .font(.system(size: 11, weight: .bold, design: .default))
                .textCase(.uppercase)
                .tracking(1.1)
                .foregroundStyle(palette.rowLabel)

            content
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

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

    private var shortcutFixedHint: String {
        "固定：Esc 关闭面板，⌘1…⌘5 切换模块。默认：⇧Tab 临时切换条目 / 文档，可在下方修改。"
    }

    private var shellBackground: some View {
        ZStack {
            LinearGradient(
                colors: [palette.shellTop, palette.shellMiddle, palette.shellBottom],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            RadialGradient(
                colors: [palette.shellPrimaryGlow, .clear],
                center: .topTrailing,
                startRadius: 20,
                endRadius: 320
            )
            .offset(x: 120, y: -120)

            RadialGradient(
                colors: [palette.shellSecondaryGlow, .clear],
                center: .topLeading,
                startRadius: 20,
                endRadius: 280
            )
            .offset(x: -120, y: -140)
        }
        .ignoresSafeArea()
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                SettingsShellHeader(
                    palette: palette,
                    currentThemeTitle: settings.appThemePreset.title,
                    currentModeTitle: settings.noteWriteMode.compactTitle
                )

                SettingsCard(
                    eyebrow: "Appearance",
                    title: "应用主题",
                    description: "在这里切换整个应用的外观主题，捕获面板与设置页会立即同步。",
                    palette: palette
                ) {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack(spacing: 12) {
                            Text("只保留 4 个真正有辨识度的主题。")
                                .font(.system(size: 12, weight: .medium, design: .default))
                                .foregroundStyle(palette.mutedText)

                            Spacer()

                            Label(settings.appThemePreset.title, systemImage: settings.appThemePreset.iconName)
                                .font(.system(size: 12, weight: .semibold, design: .default))
                                .foregroundStyle(palette.chipText)
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                                .background(
                                    Capsule(style: .continuous)
                                        .fill(palette.chipBackground)
                                )
                        }

                        LazyVGrid(
                            columns: [
                                GridItem(.flexible(), spacing: 12, alignment: .top),
                                GridItem(.flexible(), spacing: 12, alignment: .top)
                            ],
                            alignment: .leading,
                            spacing: 12
                        ) {
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
                }

                SettingsCard(
                    eyebrow: "Vault",
                    title: "Vault",
                    description: "设置你的 Obsidian 仓库路径，用于写入 Daily 条目或独立文档。",
                    palette: palette
                ) {
                    SettingRow(label: "Obsidian Vault", palette: palette, alignment: .top, labelTopPadding: 8) {
                        VStack(alignment: .leading, spacing: 8) {
                            TextField("/Users/you/Obsidian", text: $settings.vaultPath)
                                .textFieldStyle(.plain)
                                .settingsFieldChrome(palette)

                            HStack(spacing: 8) {
                                Button("选择文件夹") {
                                    chooseVaultPath()
                                }
                                .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))

                                Text(settings.vaultPath.isEmpty ? "未配置" : settings.vaultPath)
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                                    .lineLimit(1)
                            }

                            if let issue = settings.vaultPathValidationIssue {
                                Text(issue.message)
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.warningText)
                            }
                        }
                    }
                }

                SettingsCard(
                    eyebrow: "Routing",
                    title: "保存目标",
                    description: "在这里定义默认写入模式和 Daily / Inbox 路由。捕获面板不再显示这些控件。",
                    palette: palette
                ) {
                    VStack(spacing: 10) {
                        SettingRow(label: "默认模式", palette: palette) {
                            VStack(alignment: .leading, spacing: 10) {
                                HStack(spacing: 12) {
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

                                HStack(spacing: 8) {
                                    Label("切换快捷键 \(modeToggleLabel)", systemImage: "keyboard")
                                        .font(.system(size: 12, weight: .semibold, design: .default))
                                        .foregroundStyle(palette.chipText)
                                        .padding(.horizontal, 10)
                                        .padding(.vertical, 7)
                                        .background(
                                            Capsule(style: .continuous)
                                                .fill(palette.chipBackground)
                                        )

                                    Text("面板里随时切换，设置会立即同步。")
                                        .font(.system(size: 12, weight: .medium, design: .default))
                                        .foregroundStyle(palette.mutedText)
                                        .fixedSize(horizontal: false, vertical: true)
                                }
                            }
                        }

                        SettingRow(label: "Inbox 目录", palette: palette) {
                            VStack(alignment: .leading, spacing: 6) {
                                TextField("inbox", text: $settings.inboxFolderName)
                                    .textFieldStyle(.plain)
                                    .settingsFieldChrome(palette)

                                Text("独立文档默认目录：{Vault}/\(settings.inboxFolderName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "inbox" : settings.inboxFolderName.trimmingCharacters(in: .whitespacesAndNewlines))")
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                            }
                        }

                        SettingRow(label: "子目录", palette: palette) {
                            TextField("Daily", text: $settings.dailyFolderName)
                                .textFieldStyle(.plain)
                                .settingsFieldChrome(palette)
                        }
                        .disabled(settings.noteWriteMode == .file)
                        .opacity(settings.noteWriteMode == .file ? 0.6 : 1)

                        SettingRow(label: "文件名格式", palette: palette) {
                            VStack(alignment: .leading, spacing: 6) {
                                TextField("yyyy M月d日 EEEE", text: $settings.dailyFileDateFormat)
                                    .textFieldStyle(.plain)
                                    .settingsFieldChrome(palette)

                                Text("示例：yyyy M月d日 EEEE → 2026 2月27日 星期五.md")
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                            }
                        }
                        .disabled(settings.noteWriteMode == .file)
                        .opacity(settings.noteWriteMode == .file ? 0.6 : 1)

                        SettingRow(label: "输出风格", palette: palette) {
                            VStack(alignment: .leading, spacing: 6) {
                                Picker("", selection: $settings.dailyEntryThemePreset) {
                                    ForEach(DailyEntryThemePreset.allCases) { preset in
                                        Text(preset.title).tag(preset)
                                    }
                                }
                                .labelsHidden()
                                .pickerStyle(.menu)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .settingsFieldChrome(palette, verticalPadding: 8)

                                Text("默认推荐：文本 + 时间戳（纯 Markdown）。代码块与引用也是纯 Markdown；其余风格使用 Obsidian 原生 Callout。")
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                            }
                        }
                        .disabled(settings.noteWriteMode == .file)
                        .opacity(settings.noteWriteMode == .file ? 0.6 : 1)

                        SettingRow(label: "分割线", palette: palette) {
                            VStack(alignment: .leading, spacing: 6) {
                                Picker("", selection: $settings.markdownEntrySeparatorStyle) {
                                    ForEach(MarkdownEntrySeparatorStyle.allCases) { style in
                                        Text(style.title).tag(style)
                                    }
                                }
                                .labelsHidden()
                                .pickerStyle(.menu)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .settingsFieldChrome(palette, verticalPadding: 8)

                                Text("仅作用于纯 Markdown 样式：文本 + 时间戳、代码块（经典）、引用（Markdown）。")
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                            }
                        }
                        .disabled(settings.noteWriteMode == .file)
                        .opacity(settings.noteWriteMode == .file ? 0.6 : 1)

                        SettingRow(label: "Markdown", palette: palette) {
                            Text("原生语法可配置项：文本 + 时间戳、代码块、引用、分割线。")
                                .font(.system(size: 12, weight: .medium, design: .default))
                                .foregroundStyle(palette.mutedText)
                        }
                        .disabled(settings.noteWriteMode == .file)
                        .opacity(settings.noteWriteMode == .file ? 0.6 : 1)
                    }
                }

                SettingsCard(
                    eyebrow: "Structure",
                    title: "默认模块",
                    description: "定义捕捉面板底部的五个默认模块。",
                    palette: palette
                ) {
                    SettingRow(label: "模块列表", palette: palette) {
                        VStack(alignment: .leading, spacing: 10) {
                            ForEach(NoteSection.allCases) { section in
                                HStack(spacing: 10) {
                                    Text("\(section.rawValue)")
                                        .font(.system(size: 12, weight: .bold, design: .default))
                                        .foregroundStyle(palette.chipText)
                                        .frame(width: 22, height: 22)
                                        .background(
                                            Circle()
                                                .fill(palette.chipBackground)
                                        )

                                    TextField("模块名称", text: sectionTitleBinding(for: section))
                                        .textFieldStyle(.plain)
                                        .settingsFieldChrome(palette)
                                }
                            }

                            HStack {
                                Text("这些名称会写入 Daily Note 的一级标题（# 标题）。")
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                                Spacer()
                                Button("重置默认") {
                                    settings.resetSectionTitlesToDefault()
                                }
                                .buttonStyle(SettingsSecondaryButtonStyle(palette: palette))
                            }
                        }
                    }
                }

                SettingsCard(
                    eyebrow: "Shortcuts",
                    title: "快捷键",
                    description: "为关键操作自定义快捷键：创建、发送、追加和写入模式切换。",
                    palette: palette
                ) {
                    VStack(spacing: 10) {
                        shortcutRecorderRow(for: .create)
                        shortcutRecorderRow(for: .send)
                        shortcutRecorderRow(for: .append)
                        shortcutRecorderRow(for: .toggleWriteMode)

                        SettingRow(label: "固定操作", palette: palette, alignment: .top, labelTopPadding: 6) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("点击“录制快捷键”后直接按下组合键，按 Esc 可取消录制。")
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                                Text(shortcutFixedHint)
                                    .font(.system(size: 12, weight: .medium, design: .default))
                                    .foregroundStyle(palette.mutedText)
                            }
                        }

                        if let shortcutRecorderMessage {
                            HStack {
                                Spacer()
                                Text(shortcutRecorderMessage)
                                    .font(.system(size: 12, weight: .semibold, design: .default))
                                    .foregroundStyle(palette.warningText)
                            }
                        }

                        HStack {
                            Spacer()
                            Button("恢复默认快捷键") {
                                stopRecording()
                                settings.resetShortcutSettingsToDefault()
                            }
                            .buttonStyle(SettingsSecondaryButtonStyle(palette: palette))
                        }
                    }
                }

                SettingsCard(
                    eyebrow: "System",
                    title: "系统",
                    description: "应用启动行为。",
                    palette: palette
                ) {
                    SettingRow(label: "开机自启动", palette: palette, alignment: .center) {
                        Toggle("", isOn: $settings.launchAtLogin)
                            .labelsHidden()
                            .toggleStyle(.switch)
                            .tint(palette.accent)
                    }
                }
            }
            .padding(28)
        }
        .scrollIndicators(.hidden)
        .background(shellBackground)
        .frame(width: 760, height: 760)
        .onDisappear {
            stopRecording(clearMessage: false)
        }
    }

    @ViewBuilder
    private func shortcutRecorderRow(for target: ShortcutTarget) -> some View {
        let currentShortcut = shortcut(for: target)
        let isRecording = recordingTarget == target

        SettingRow(label: target.name, palette: palette) {
            HStack(spacing: 10) {
                Text(currentShortcut.displayLabel)
                    .font(.system(size: 12, weight: .bold, design: .default))
                    .foregroundStyle(palette.chipText)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .fill(palette.chipBackground)
                    )

                Spacer()

                Button(isRecording ? "按键中…" : "录制快捷键") {
                    toggleRecording(for: target)
                }
                .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))

                if isRecording {
                    Button("取消") {
                        stopRecording()
                    }
                    .buttonStyle(SettingsSecondaryButtonStyle(palette: palette))
                }
            }
        }
    }

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
            shortcutRecorderMessage = "快捷键必须包含至少一个修饰键（⌘ / ⇧ / ⌥ / ⌃）。"
            NSSound.beep()
            return
        }

        if candidate.keyCode == UInt32(kVK_Escape) {
            shortcutRecorderMessage = "Esc 已用于取消录制与关闭面板，不能被覆盖。"
            NSSound.beep()
            return
        }

        if target != .create && candidate.isReservedSectionSwitch {
            shortcutRecorderMessage = "⌘1…⌘5 已用于切换模块，请换一个组合。"
            NSSound.beep()
            return
        }

        if let conflict = conflictingShortcutTarget(for: candidate, excluding: target) {
            shortcutRecorderMessage = "与“\(conflict.name)”冲突，请使用其他组合键。"
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

    private func chooseVaultPath() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "选择 Vault"

        if panel.runModal() == .OK {
            settings.vaultPath = panel.url?.path ?? ""
        }
    }

    private func sectionTitleBinding(for section: NoteSection) -> Binding<String> {
        Binding {
            settings.title(for: section)
        } set: { updatedTitle in
            settings.setTitle(updatedTitle, for: section)
        }
    }
}

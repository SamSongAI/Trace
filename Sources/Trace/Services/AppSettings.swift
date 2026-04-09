import AppKit
import Combine
import Carbon
import Foundation
import ServiceManagement

enum AppLanguage: String, CaseIterable, Identifiable {
    case zh
    case en
    case ja

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .zh: return "中文"
        case .en: return "English"
        case .ja: return "日本語"
        }
    }

    static var systemDefault: AppLanguage {
        let preferred = Locale.preferredLanguages.first ?? ""
        if preferred.hasPrefix("zh") { return .zh }
        if preferred.hasPrefix("ja") { return .ja }
        return .en
    }
}

enum SettingKeys {
    static let language = "trace.language"
    static let vaultPath = "trace.vaultPath"
    static let dailyFolderName = "trace.dailyFolderName"
    static let dailyFileDateFormat = "trace.dailyFileDateFormat"
    static let noteWriteMode = "trace.noteWriteMode"
    static let inboxFolderName = "trace.inboxFolderName"
    static let hotKeyCode = "trace.hotKeyCode"
    static let hotKeyModifiers = "trace.hotKeyModifiers"
    static let sendNoteKeyCode = "trace.sendNoteKeyCode"
    static let sendNoteModifiers = "trace.sendNoteModifiers"
    static let appendNoteKeyCode = "trace.appendNoteKeyCode"
    static let appendNoteModifiers = "trace.appendNoteModifiers"
    static let modeToggleKeyCode = "trace.modeToggleKeyCode"
    static let modeToggleModifiers = "trace.modeToggleModifiers"
    static let launchAtLogin = "trace.launchAtLogin"
    static let panelOriginX = "trace.panel.originX"
    static let panelOriginY = "trace.panel.originY"
    static let panelWidth = "trace.panel.width"
    static let panelHeight = "trace.panel.height"
    static let appThemePreset = "trace.appThemePreset"
    static let sectionTitles = "trace.sectionTitles"
    static let sectionTitlesOrderVersion = "trace.sectionTitlesOrderVersion"
    static let dailyEntryThemePreset = "trace.dailyEntryThemePreset"
    static let markdownEntrySeparatorStyle = "trace.markdownEntrySeparatorStyle"
    static let lastUsedSectionIndex = "trace.lastUsedSectionIndex"
    static let inboxVaultPath = "trace.inboxVaultPath"
    static let threadConfigs = "trace.threadConfigs"
    static let lastUsedThreadId = "trace.lastUsedThreadId"
    static let threadVaultPath = "trace.threadVaultPath"
}

enum LegacySettingKeys {
    static let bundleIdentifier = "com.flashnote.app"
    static let vaultPath = "flashnote.vaultPath"
    static let dailyFolderName = "flashnote.dailyFolderName"
    static let dailyFileDateFormat = "flashnote.dailyFileDateFormat"
    static let noteWriteMode = "flashnote.noteWriteMode"
    static let inboxFolderName = "flashnote.inboxFolderName"
    static let hotKeyCode = "flashnote.hotKeyCode"
    static let hotKeyModifiers = "flashnote.hotKeyModifiers"
    static let sendNoteKeyCode = "flashnote.sendNoteKeyCode"
    static let sendNoteModifiers = "flashnote.sendNoteModifiers"
    static let appendNoteKeyCode = "flashnote.appendNoteKeyCode"
    static let appendNoteModifiers = "flashnote.appendNoteModifiers"
    static let modeToggleKeyCode = "flashnote.modeToggleKeyCode"
    static let modeToggleModifiers = "flashnote.modeToggleModifiers"
    static let launchAtLogin = "flashnote.launchAtLogin"
    static let panelOriginX = "flashnote.panel.originX"
    static let panelOriginY = "flashnote.panel.originY"
    static let panelWidth = "flashnote.panel.width"
    static let panelHeight = "flashnote.panel.height"
    static let appThemePreset = "flashnote.appThemePreset"
    static let captureAppearance = "flashnote.captureAppearance"
    static let sectionTitles = "flashnote.sectionTitles"
    static let sectionTitlesOrderVersion = "flashnote.sectionTitlesOrderVersion"
    static let dailyEntryThemePreset = "flashnote.dailyEntryThemePreset"
    static let markdownEntrySeparatorStyle = "flashnote.markdownEntrySeparatorStyle"
    static let dailyEntryContainerStyle = "flashnote.dailyEntryContainerStyle"
    static let dailyCardTheme = "flashnote.dailyCardTheme"
    static let threadConfigs = "flashnote.threadConfigs"
}

enum NoteWriteMode: String, CaseIterable, Identifiable {
    case dimension
    case file
    case thread

    var id: String { rawValue }

    var title: String {
        switch self {
        case .dimension: return L10n.writeModeDailyTitle
        case .file: return L10n.writeModeDocumentTitle
        case .thread: return L10n.writeModeThreadTitle
        }
    }

    var compactTitle: String {
        switch self {
        case .dimension: return L10n.writeModeDailyCompact
        case .file: return L10n.writeModeDocumentCompact
        case .thread: return L10n.writeModeThreadCompact
        }
    }

    var iconName: String {
        switch self {
        case .dimension:
            return "square.grid.2x2"
        case .file:
            return "doc.text"
        case .thread:
            return "text.bubble"
        }
    }

    var destinationTitle: String {
        switch self {
        case .dimension: return L10n.writeModeDailyDestination
        case .file: return L10n.writeModeDocumentDestination
        case .thread: return L10n.writeModeThreadDestination
        }
    }

    var summary: String {
        switch self {
        case .dimension: return L10n.writeModeDailySummary
        case .file: return L10n.writeModeDocumentSummary
        case .thread: return L10n.writeModeThreadSummary
        }
    }

    var targetSummary: String {
        switch self {
        case .dimension: return L10n.writeModeDailyTarget
        case .file: return L10n.writeModeDocumentTarget
        case .thread: return L10n.writeModeThreadTarget
        }
    }

    func next() -> NoteWriteMode {
        switch self {
        case .dimension: return .file
        case .file: return .thread
        case .thread: return .dimension
        }
    }

    func previous() -> NoteWriteMode {
        switch self {
        case .dimension: return .thread
        case .file: return .dimension
        case .thread: return .file
        }
    }
}

enum DailyFileDateFormat: String, CaseIterable, Identifiable {
    case chineseFull = "yyyy M月d日 EEEE"
    case chineseShort = "yyyy M月d日"
    case isoDate = "yyyy-MM-dd"
    case isoDateTime = "yyyy-MM-dd EEEE"
    case slashDate = "yyyy/MM/dd"

    var id: String { rawValue }

    var title: String {
        let formatter = DateFormatter()
        formatter.locale = Locale.current
        formatter.dateFormat = rawValue
        let example = formatter.string(from: Date())
        return "\(rawValue)  →  \(example)"
    }

    static func resolved(fromStored value: String?) -> DailyFileDateFormat {
        guard let value, let preset = DailyFileDateFormat(rawValue: value) else {
            return .chineseFull
        }
        return preset
    }
}

enum DailyEntryThemePreset: String, CaseIterable, Identifiable {
    case codeBlockClassic
    case plainTextTimestamp
    case markdownQuote

    var id: String { rawValue }

    var title: String {
        switch self {
        case .codeBlockClassic: return L10n.entryCodeBlock
        case .plainTextTimestamp: return L10n.entryPlainText
        case .markdownQuote: return L10n.entryQuote
        }
    }

    /// Resolves a stored rawValue that may reference a removed preset.
    static func resolved(fromStored rawValue: String?) -> DailyEntryThemePreset {
        guard let rawValue, let preset = DailyEntryThemePreset(rawValue: rawValue) else {
            return .codeBlockClassic
        }
        return preset
    }

    fileprivate static func migrated(
        from legacyContainer: LegacyDailyEntryContainerStyle,
        legacyCardTheme: LegacyDailyCardTheme
    ) -> DailyEntryThemePreset {
        // All legacy callout styles map to codeBlockClassic now.
        guard legacyContainer != .calloutCard else {
            return .codeBlockClassic
        }
        return .plainTextTimestamp
    }
}

enum MarkdownEntrySeparatorStyle: String, CaseIterable, Identifiable {
    case none
    case horizontalRule
    case asteriskRule

    var id: String { rawValue }

    var title: String {
        switch self {
        case .none: return L10n.separatorNone
        case .horizontalRule: return L10n.separatorHorizontalRule
        case .asteriskRule: return L10n.separatorAsteriskRule
        }
    }

    var markdown: String? {
        switch self {
        case .none:
            return nil
        case .horizontalRule:
            return "---"
        case .asteriskRule:
            return "***"
        }
    }
}

private enum LegacyDailyEntryContainerStyle: String {
    case codeBlock
    case calloutCard
}

private enum LegacyDailyCardTheme: String {
    case anthropic
    case obsidianPurple
    case slate
}

enum VaultPathValidationIssue: Equatable {
    case empty
    case doesNotExist
    case notDirectory
    case notWritable

    var message: String {
        switch self {
        case .empty: return L10n.vaultEmpty
        case .doesNotExist: return L10n.vaultNotExist
        case .notDirectory: return L10n.vaultNotDirectory
        case .notWritable: return L10n.vaultNotWritable
        }
    }
}

final class AppSettings: ObservableObject {
    static let shared = AppSettings()
    private static let currentSectionTitleOrderVersion = 2
    private static let defaultGlobalHotKeyCode = UInt32(kVK_ANSI_N)
    private static let defaultGlobalHotKeyModifiers = UInt32(cmdKey)
    private static let defaultSendNoteKeyCode = UInt32(kVK_Return)
    private static let defaultSendNoteModifiers = UInt32(cmdKey)
    private static let defaultAppendNoteKeyCode = UInt32(kVK_Return)
    private static let defaultAppendNoteModifiers = UInt32(cmdKey | shiftKey)
    private static let defaultModeToggleKeyCode = UInt32(kVK_Tab)
    private static let defaultModeToggleModifiers = UInt32(shiftKey)

    private let defaults: UserDefaults
    private let fileManager: FileManager

    @Published var language: AppLanguage {
        didSet {
            defaults.set(language.rawValue, forKey: SettingKeys.language)
        }
    }

    @Published var vaultPath: String {
        didSet {
            defaults.set(vaultPath, forKey: SettingKeys.vaultPath)
        }
    }

    @Published var inboxVaultPath: String {
        didSet {
            defaults.set(inboxVaultPath, forKey: SettingKeys.inboxVaultPath)
        }
    }

    @Published var dailyFolderName: String {
        didSet {
            defaults.set(dailyFolderName, forKey: SettingKeys.dailyFolderName)
        }
    }

    @Published var dailyFileDateFormat: String {
        didSet {
            defaults.set(dailyFileDateFormat, forKey: SettingKeys.dailyFileDateFormat)
        }
    }

    @Published var noteWriteMode: NoteWriteMode {
        didSet {
            defaults.set(noteWriteMode.rawValue, forKey: SettingKeys.noteWriteMode)
        }
    }

    @Published var inboxFolderName: String {
        didSet {
            defaults.set(inboxFolderName, forKey: SettingKeys.inboxFolderName)
        }
    }

    @Published var hotKeyCode: UInt32 {
        didSet {
            defaults.set(Int(hotKeyCode), forKey: SettingKeys.hotKeyCode)
        }
    }

    @Published var hotKeyModifiers: UInt32 {
        didSet {
            let normalized = KeyboardShortcut.sanitizedCarbonModifiers(hotKeyModifiers)
            if normalized != hotKeyModifiers {
                hotKeyModifiers = normalized
                return
            }
            defaults.set(Int(hotKeyModifiers), forKey: SettingKeys.hotKeyModifiers)
        }
    }

    @Published var sendNoteKeyCode: UInt32 {
        didSet {
            defaults.set(Int(sendNoteKeyCode), forKey: SettingKeys.sendNoteKeyCode)
        }
    }

    @Published var sendNoteModifiers: UInt32 {
        didSet {
            let normalized = KeyboardShortcut.sanitizedCarbonModifiers(sendNoteModifiers)
            if normalized != sendNoteModifiers {
                sendNoteModifiers = normalized
                return
            }
            defaults.set(Int(sendNoteModifiers), forKey: SettingKeys.sendNoteModifiers)
        }
    }

    @Published var appendNoteKeyCode: UInt32 {
        didSet {
            defaults.set(Int(appendNoteKeyCode), forKey: SettingKeys.appendNoteKeyCode)
        }
    }

    @Published var appendNoteModifiers: UInt32 {
        didSet {
            let normalized = KeyboardShortcut.sanitizedCarbonModifiers(appendNoteModifiers)
            if normalized != appendNoteModifiers {
                appendNoteModifiers = normalized
                return
            }
            defaults.set(Int(appendNoteModifiers), forKey: SettingKeys.appendNoteModifiers)
        }
    }

    @Published var modeToggleKeyCode: UInt32 {
        didSet {
            defaults.set(Int(modeToggleKeyCode), forKey: SettingKeys.modeToggleKeyCode)
        }
    }

    @Published var modeToggleModifiers: UInt32 {
        didSet {
            let normalized = KeyboardShortcut.sanitizedCarbonModifiers(modeToggleModifiers)
            if normalized != modeToggleModifiers {
                modeToggleModifiers = normalized
                return
            }
            defaults.set(Int(modeToggleModifiers), forKey: SettingKeys.modeToggleModifiers)
        }
    }

    @Published var launchAtLogin: Bool {
        didSet {
            defaults.set(launchAtLogin, forKey: SettingKeys.launchAtLogin)
            updateLaunchAtLogin()
        }
    }

    @Published var appThemePreset: AppThemePreset {
        didSet {
            defaults.set(appThemePreset.rawValue, forKey: SettingKeys.appThemePreset)
        }
    }

    @Published var sectionTitles: [String] {
        didSet {
            let normalized = Self.normalizedSectionTitles(sectionTitles)
            if normalized != sectionTitles {
                sectionTitles = normalized
                return
            }
            defaults.set(sectionTitles, forKey: SettingKeys.sectionTitles)
        }
    }

    @Published var dailyEntryThemePreset: DailyEntryThemePreset {
        didSet {
            defaults.set(dailyEntryThemePreset.rawValue, forKey: SettingKeys.dailyEntryThemePreset)
        }
    }

    @Published var markdownEntrySeparatorStyle: MarkdownEntrySeparatorStyle {
        didSet {
            defaults.set(markdownEntrySeparatorStyle.rawValue, forKey: SettingKeys.markdownEntrySeparatorStyle)
        }
    }

    @Published var threadConfigs: [ThreadConfig] {
        didSet {
            if let data = try? JSONEncoder().encode(threadConfigs) {
                defaults.set(data, forKey: SettingKeys.threadConfigs)
            }
        }
    }

    @Published var threadVaultPath: String {
        didSet {
            defaults.set(threadVaultPath, forKey: SettingKeys.threadVaultPath)
        }
    }

    var lastUsedSectionIndex: Int {
        get { defaults.integer(forKey: SettingKeys.lastUsedSectionIndex) }
        set { defaults.set(newValue, forKey: SettingKeys.lastUsedSectionIndex) }
    }

    init(
        defaults: UserDefaults = .standard,
        legacyDefaults: UserDefaults? = nil,
        fileManager: FileManager = .default
    ) {
        self.defaults = defaults
        self.fileManager = fileManager

        Self.migrateLegacyDefaultsIfNeeded(
            into: defaults,
            legacyDefaults: legacyDefaults ?? Self.defaultLegacyDefaults(for: defaults)
        )

        language = AppLanguage(rawValue: defaults.string(forKey: SettingKeys.language) ?? "") ?? .systemDefault
        vaultPath = defaults.string(forKey: SettingKeys.vaultPath) ?? ""
        inboxVaultPath = defaults.string(forKey: SettingKeys.inboxVaultPath) ?? ""
        dailyFolderName = defaults.string(forKey: SettingKeys.dailyFolderName) ?? "Daily"
        dailyFileDateFormat = defaults.string(forKey: SettingKeys.dailyFileDateFormat) ?? "yyyy M月d日 EEEE"
        noteWriteMode = NoteWriteMode(rawValue: defaults.string(forKey: SettingKeys.noteWriteMode) ?? "") ?? .dimension
        inboxFolderName = defaults.string(forKey: SettingKeys.inboxFolderName) ?? "inbox"

        hotKeyCode = {
            let value = defaults.integer(forKey: SettingKeys.hotKeyCode)
            return value == 0 ? Self.defaultGlobalHotKeyCode : UInt32(value)
        }()
        hotKeyModifiers = {
            let value = defaults.integer(forKey: SettingKeys.hotKeyModifiers)
            let candidate = value == 0 ? Self.defaultGlobalHotKeyModifiers : UInt32(value)
            return KeyboardShortcut.sanitizedCarbonModifiers(candidate)
        }()
        sendNoteKeyCode = {
            let value = defaults.integer(forKey: SettingKeys.sendNoteKeyCode)
            return value == 0 ? Self.defaultSendNoteKeyCode : UInt32(value)
        }()
        sendNoteModifiers = {
            let value = defaults.integer(forKey: SettingKeys.sendNoteModifiers)
            let candidate = value == 0 ? Self.defaultSendNoteModifiers : UInt32(value)
            return KeyboardShortcut.sanitizedCarbonModifiers(candidate)
        }()
        appendNoteKeyCode = {
            let value = defaults.integer(forKey: SettingKeys.appendNoteKeyCode)
            return value == 0 ? Self.defaultAppendNoteKeyCode : UInt32(value)
        }()
        appendNoteModifiers = {
            let value = defaults.integer(forKey: SettingKeys.appendNoteModifiers)
            let candidate = value == 0 ? Self.defaultAppendNoteModifiers : UInt32(value)
            return KeyboardShortcut.sanitizedCarbonModifiers(candidate)
        }()
        modeToggleKeyCode = {
            let value = defaults.integer(forKey: SettingKeys.modeToggleKeyCode)
            return value == 0 ? Self.defaultModeToggleKeyCode : UInt32(value)
        }()
        modeToggleModifiers = {
            let value = defaults.integer(forKey: SettingKeys.modeToggleModifiers)
            let candidate = value == 0 ? Self.defaultModeToggleModifiers : UInt32(value)
            return KeyboardShortcut.sanitizedCarbonModifiers(candidate)
        }()

        launchAtLogin = defaults.bool(forKey: SettingKeys.launchAtLogin)
        let storedThemeRawValue = defaults.string(forKey: SettingKeys.appThemePreset)
        if let storedThemePreset = AppThemePreset.resolved(fromStoredRawValue: storedThemeRawValue) {
            appThemePreset = storedThemePreset
            if storedThemeRawValue != storedThemePreset.rawValue {
                defaults.set(storedThemePreset.rawValue, forKey: SettingKeys.appThemePreset)
            }
        } else {
            appThemePreset = .defaultValue
            defaults.set(AppThemePreset.defaultValue.rawValue, forKey: SettingKeys.appThemePreset)
        }
        let storedOrderVersion = defaults.integer(forKey: SettingKeys.sectionTitlesOrderVersion)
        let persistedSectionTitles = defaults.stringArray(forKey: SettingKeys.sectionTitles) ?? []
        let migratedSectionTitles = Self.migratedStoredSectionTitles(
            persistedSectionTitles,
            storedVersion: storedOrderVersion
        )
        sectionTitles = migratedSectionTitles
        markdownEntrySeparatorStyle = MarkdownEntrySeparatorStyle(
            rawValue: defaults.string(forKey: SettingKeys.markdownEntrySeparatorStyle) ?? ""
        ) ?? .horizontalRule
        if persistedSectionTitles != migratedSectionTitles {
            defaults.set(migratedSectionTitles, forKey: SettingKeys.sectionTitles)
        }
        if storedOrderVersion < Self.currentSectionTitleOrderVersion {
            defaults.set(Self.currentSectionTitleOrderVersion, forKey: SettingKeys.sectionTitlesOrderVersion)
        }
        let resolvedPreset = DailyEntryThemePreset.resolved(
            fromStored: defaults.string(forKey: SettingKeys.dailyEntryThemePreset)
        )
        dailyEntryThemePreset = resolvedPreset
        defaults.set(resolvedPreset.rawValue, forKey: SettingKeys.dailyEntryThemePreset)

        // Thread configs
        if let data = defaults.data(forKey: SettingKeys.threadConfigs),
           let configs = try? JSONDecoder().decode([ThreadConfig].self, from: data) {
            threadConfigs = configs
        } else {
            threadConfigs = Self.defaultThreadConfigs
        }
        threadVaultPath = defaults.string(forKey: SettingKeys.threadVaultPath) ?? ""

        normalizePanelShortcutCollisionsIfNeeded()
    }

    private static var defaultThreadConfigs: [ThreadConfig] {
        [
            ThreadConfig(name: "想法", targetFile: "想法.md", order: 0),
            ThreadConfig(name: "读书笔记", targetFile: "读书笔记.md", order: 1),
            ThreadConfig(name: "产品设计", targetFile: "产品设计.md", order: 2),
            ThreadConfig(name: "技术研究", targetFile: "技术研究.md", order: 3)
        ]
    }

    var hasVaultPath: Bool {
        !vaultPath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var vaultPathValidationIssue: VaultPathValidationIssue? {
        let trimmedPath = vaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedPath.isEmpty else { return .empty }

        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: trimmedPath, isDirectory: &isDirectory) else {
            return .doesNotExist
        }

        guard isDirectory.boolValue else {
            return .notDirectory
        }

        guard fileManager.isWritableFile(atPath: trimmedPath) else {
            return .notWritable
        }

        return nil
    }

    var hasValidVaultPath: Bool {
        vaultPathValidationIssue == nil
    }

    var inboxVaultPathValidationIssue: VaultPathValidationIssue? {
        let trimmedPath = inboxVaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedPath.isEmpty else { return .empty }

        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: trimmedPath, isDirectory: &isDirectory) else {
            return .doesNotExist
        }

        guard isDirectory.boolValue else {
            return .notDirectory
        }

        guard fileManager.isWritableFile(atPath: trimmedPath) else {
            return .notWritable
        }

        return nil
    }

    var hasValidInboxVaultPath: Bool {
        inboxVaultPathValidationIssue == nil
    }

    var hasValidThreadVaultPath: Bool {
        threadVaultPathValidationIssue == nil
    }

    var threadVaultPathValidationIssue: VaultPathValidationIssue? {
        let trimmedPath = threadVaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedPath.isEmpty else { return .empty }

        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: trimmedPath, isDirectory: &isDirectory) else {
            return .doesNotExist
        }

        guard isDirectory.boolValue else {
            return .notDirectory
        }

        guard fileManager.isWritableFile(atPath: trimmedPath) else {
            return .notWritable
        }

        return nil
    }

    var defaultThread: ThreadConfig? {
        let savedId = defaults.string(forKey: SettingKeys.lastUsedThreadId)
        if let savedId = savedId,
           let uuid = UUID(uuidString: savedId),
           let config = threadConfigs.first(where: { $0.id == uuid }) {
            return config
        }
        return threadConfigs.first
    }

    func addThread(name: String, targetFile: String, icon: String? = nil) {
        let maxOrder = threadConfigs.map(\.order).max() ?? 0

        // Auto-increment name if duplicate
        var finalName = name
        var sequence = 1
        let baseName = name
        while threadConfigs.contains(where: { $0.name == finalName }) {
            sequence += 1
            finalName = "\(baseName)\(sequence)"
        }

        // Auto-increment filename if duplicate
        var finalTargetFile = targetFile
        var fileSequence = 1
        let baseFileName = targetFile.replacingOccurrences(of: ".md", with: "")
        while threadConfigs.contains(where: { $0.targetFile == finalTargetFile }) {
            fileSequence += 1
            finalTargetFile = "\(baseFileName)\(fileSequence).md"
        }

        let config = ThreadConfig(
            name: finalName,
            targetFile: finalTargetFile,
            icon: icon,
            order: maxOrder + 1
        )
        threadConfigs.append(config)
    }

    func removeThread(_ config: ThreadConfig) {
        threadConfigs.removeAll { $0.id == config.id }
    }

    func updateThread(_ config: ThreadConfig) {
        if let index = threadConfigs.firstIndex(where: { $0.id == config.id }) {
            threadConfigs[index] = config
        }
    }

    func setLastUsedThread(_ config: ThreadConfig?) {
        if let config = config {
            defaults.set(config.id.uuidString, forKey: SettingKeys.lastUsedThreadId)
        } else {
            defaults.removeObject(forKey: SettingKeys.lastUsedThreadId)
        }
    }

    var canAddThread: Bool {
        threadConfigs.count < 9
    }

    var canRemoveThread: Bool {
        threadConfigs.count > 1
    }

    var appTheme: TraceTheme {
        appThemePreset.theme
    }

    var sections: [NoteSection] {
        sectionTitles.enumerated().map { index, title in
            NoteSection(index: index, title: title)
        }
    }

    var canAddSection: Bool {
        sectionTitles.count < NoteSection.maximumCount
    }

    var canRemoveSection: Bool {
        sectionTitles.count > NoteSection.minimumCount
    }

    var defaultSection: NoteSection {
        let saved = lastUsedSectionIndex
        let section = saved > 0 ? NoteSection(index: saved, title: "") : nil
        return resolvedSection(for: section)
    }

    func savedPanelFrame() -> NSRect? {
        guard defaults.object(forKey: SettingKeys.panelOriginX) != nil,
              defaults.object(forKey: SettingKeys.panelOriginY) != nil,
              defaults.object(forKey: SettingKeys.panelWidth) != nil,
              defaults.object(forKey: SettingKeys.panelHeight) != nil else {
            return nil
        }

        let x = defaults.double(forKey: SettingKeys.panelOriginX)
        let y = defaults.double(forKey: SettingKeys.panelOriginY)
        let width = defaults.double(forKey: SettingKeys.panelWidth)
        let height = defaults.double(forKey: SettingKeys.panelHeight)

        return NSRect(x: x, y: y, width: width, height: height)
    }

    func savePanelFrame(_ frame: NSRect) {
        defaults.set(frame.origin.x, forKey: SettingKeys.panelOriginX)
        defaults.set(frame.origin.y, forKey: SettingKeys.panelOriginY)
        defaults.set(frame.size.width, forKey: SettingKeys.panelWidth)
        defaults.set(frame.size.height, forKey: SettingKeys.panelHeight)
    }

    func title(for section: NoteSection) -> String {
        guard sectionTitles.indices.contains(section.index) else { return section.title }
        let stored = sectionTitles[section.index]
        return stored.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            ? NoteSection.defaultTitle(for: section.index)
            : stored
    }

    func header(for section: NoteSection) -> String {
        "# \(title(for: section))"
    }

    func setTitle(_ title: String, for section: NoteSection) {
        guard sectionTitles.indices.contains(section.index) else { return }
        var updated = sectionTitles
        updated[section.index] = title
        sectionTitles = updated
    }

    func addSection() {
        guard canAddSection else { return }
        var updated = sectionTitles
        updated.append(NoteSection.defaultTitle(for: updated.count))
        sectionTitles = updated
    }

    func removeSection(_ section: NoteSection) {
        guard canRemoveSection, sectionTitles.indices.contains(section.index) else { return }
        var updated = sectionTitles
        updated.remove(at: section.index)
        sectionTitles = Self.normalizedSectionTitles(updated)
    }

    func section(atShortcutIndex index: Int) -> NoteSection? {
        guard sections.indices.contains(index) else { return nil }
        return sections[index]
    }

    func resolvedSection(for section: NoteSection?) -> NoteSection {
        let availableSections = sections
        guard !availableSections.isEmpty else {
            return NoteSection(index: 0, title: NoteSection.defaultTitle(for: 0))
        }

        let requestedIndex = section?.index ?? 0
        let resolvedIndex = min(max(requestedIndex, 0), availableSections.count - 1)
        return availableSections[resolvedIndex]
    }

    func resetSectionTitlesToDefault() {
        sectionTitles = Self.defaultSectionTitles
    }

    func resetShortcutSettingsToDefault() {
        hotKeyCode = Self.defaultGlobalHotKeyCode
        hotKeyModifiers = Self.defaultGlobalHotKeyModifiers
        sendNoteKeyCode = Self.defaultSendNoteKeyCode
        sendNoteModifiers = Self.defaultSendNoteModifiers
        appendNoteKeyCode = Self.defaultAppendNoteKeyCode
        appendNoteModifiers = Self.defaultAppendNoteModifiers
        modeToggleKeyCode = Self.defaultModeToggleKeyCode
        modeToggleModifiers = Self.defaultModeToggleModifiers
    }

    private func normalizePanelShortcutCollisionsIfNeeded() {
        guard sendNoteKeyCode == appendNoteKeyCode,
              sendNoteModifiers == appendNoteModifiers else {
            return
        }

        appendNoteKeyCode = Self.defaultAppendNoteKeyCode
        appendNoteModifiers = Self.defaultAppendNoteModifiers
    }

    private static var defaultSectionTitles: [String] {
        NoteSection.defaultTitles
    }

    private static func normalizedSectionTitles(_ titles: [String]) -> [String] {
        let trimmed = Array(titles.prefix(NoteSection.maximumCount))
        let sourceTitles = trimmed.isEmpty ? defaultSectionTitles : trimmed

        return sourceTitles.enumerated().map { index, candidate in
            normalizedSectionTitle(candidate, fallback: NoteSection.defaultTitle(for: index))
        }
    }

    private static func migratedStoredSectionTitles(_ titles: [String], storedVersion: Int) -> [String] {
        let orderMigrated = migrateLegacySectionTitleOrder(titles, storedVersion: storedVersion)
        let titleMigrated = migrateLegacyProjectTitle(orderMigrated, storedVersion: storedVersion)
        return normalizedSectionTitles(titleMigrated)
    }

    private static func migrateLegacySectionTitleOrder(_ titles: [String], storedVersion: Int) -> [String] {
        guard storedVersion < currentSectionTitleOrderVersion else { return titles }
        guard titles.count >= 5 else { return titles }

        var migrated = titles
        migrated.swapAt(3, 4)
        return migrated
    }

    private static func migrateLegacyProjectTitle(_ titles: [String], storedVersion: Int) -> [String] {
        guard storedVersion < currentSectionTitleOrderVersion else { return titles }

        let projectIndex = NoteSection.project.index
        guard titles.indices.contains(projectIndex) else { return titles }

        let title = titles[projectIndex]

        let compacted = title
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: " ", with: "")
            .uppercased()

        guard compacted == "TODO" else { return titles }

        var migrated = titles
        migrated[projectIndex] = NoteSection.defaultTitle(for: projectIndex)
        return migrated
    }

    private static func normalizedSectionTitle(_ rawTitle: String, fallback: String) -> String {
        let singleLine = rawTitle
            .replacingOccurrences(of: "\r", with: " ")
            .replacingOccurrences(of: "\n", with: " ")
        let withoutHeadingMarks = singleLine.replacingOccurrences(
            of: #"^#+\s*"#,
            with: "",
            options: .regularExpression
        )
        let trimmed = withoutHeadingMarks.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? fallback : trimmed
    }

    private static func defaultLegacyDefaults(for defaults: UserDefaults) -> UserDefaults? {
        defaults === UserDefaults.standard ? UserDefaults(suiteName: LegacySettingKeys.bundleIdentifier) : nil
    }

    private static func migrateLegacyDefaultsIfNeeded(
        into defaults: UserDefaults,
        legacyDefaults: UserDefaults?
    ) {
        let legacyPairs = [
            (SettingKeys.vaultPath, LegacySettingKeys.vaultPath),
            (SettingKeys.dailyFolderName, LegacySettingKeys.dailyFolderName),
            (SettingKeys.dailyFileDateFormat, LegacySettingKeys.dailyFileDateFormat),
            (SettingKeys.noteWriteMode, LegacySettingKeys.noteWriteMode),
            (SettingKeys.inboxFolderName, LegacySettingKeys.inboxFolderName),
            (SettingKeys.hotKeyCode, LegacySettingKeys.hotKeyCode),
            (SettingKeys.hotKeyModifiers, LegacySettingKeys.hotKeyModifiers),
            (SettingKeys.sendNoteKeyCode, LegacySettingKeys.sendNoteKeyCode),
            (SettingKeys.sendNoteModifiers, LegacySettingKeys.sendNoteModifiers),
            (SettingKeys.appendNoteKeyCode, LegacySettingKeys.appendNoteKeyCode),
            (SettingKeys.appendNoteModifiers, LegacySettingKeys.appendNoteModifiers),
            (SettingKeys.launchAtLogin, LegacySettingKeys.launchAtLogin),
            (SettingKeys.panelOriginX, LegacySettingKeys.panelOriginX),
            (SettingKeys.panelOriginY, LegacySettingKeys.panelOriginY),
            (SettingKeys.panelWidth, LegacySettingKeys.panelWidth),
            (SettingKeys.panelHeight, LegacySettingKeys.panelHeight),
            (SettingKeys.sectionTitles, LegacySettingKeys.sectionTitles),
            (SettingKeys.sectionTitlesOrderVersion, LegacySettingKeys.sectionTitlesOrderVersion),
            (SettingKeys.markdownEntrySeparatorStyle, LegacySettingKeys.markdownEntrySeparatorStyle)
        ]

        for (newKey, oldKey) in legacyPairs {
            migrateLegacyValueIfNeeded(into: defaults, newKey: newKey, oldKey: oldKey, legacyDefaults: legacyDefaults)
        }

        if defaults.object(forKey: SettingKeys.appThemePreset) == nil {
            if let legacyThemeRawValue = legacyString(
                forKey: LegacySettingKeys.appThemePreset,
                in: defaults,
                legacyDefaults: legacyDefaults
            ), let resolvedThemePreset = AppThemePreset.resolved(fromStoredRawValue: legacyThemeRawValue) {
                defaults.set(resolvedThemePreset.rawValue, forKey: SettingKeys.appThemePreset)
            } else if let migratedThemePreset = AppThemePreset.migrated(
                fromLegacyCaptureAppearance: legacyString(
                    forKey: LegacySettingKeys.captureAppearance,
                    in: defaults,
                    legacyDefaults: legacyDefaults
                )
            ) {
                defaults.set(migratedThemePreset.rawValue, forKey: SettingKeys.appThemePreset)
            }
        }

        if defaults.object(forKey: SettingKeys.dailyEntryThemePreset) == nil {
            if let legacyThemeRawValue = legacyString(
                forKey: LegacySettingKeys.dailyEntryThemePreset,
                in: defaults,
                legacyDefaults: legacyDefaults
            ), DailyEntryThemePreset(rawValue: legacyThemeRawValue) != nil {
                defaults.set(legacyThemeRawValue, forKey: SettingKeys.dailyEntryThemePreset)
            } else {
                let legacyContainer = LegacyDailyEntryContainerStyle(
                    rawValue: legacyString(
                        forKey: LegacySettingKeys.dailyEntryContainerStyle,
                        in: defaults,
                        legacyDefaults: legacyDefaults
                    ) ?? ""
                ) ?? .codeBlock
                let legacyCardTheme = LegacyDailyCardTheme(
                    rawValue: legacyString(
                        forKey: LegacySettingKeys.dailyCardTheme,
                        in: defaults,
                        legacyDefaults: legacyDefaults
                    ) ?? ""
                ) ?? .anthropic
                let migratedPreset = DailyEntryThemePreset.migrated(
                    from: legacyContainer,
                    legacyCardTheme: legacyCardTheme
                )
                defaults.set(migratedPreset.rawValue, forKey: SettingKeys.dailyEntryThemePreset)
            }
        }
    }

    private static func migrateLegacyValueIfNeeded(
        into defaults: UserDefaults,
        newKey: String,
        oldKey: String,
        legacyDefaults: UserDefaults?
    ) {
        guard defaults.object(forKey: newKey) == nil else { return }
        guard let legacyValue = legacyObject(forKey: oldKey, in: defaults, legacyDefaults: legacyDefaults) else { return }
        defaults.set(legacyValue, forKey: newKey)
    }

    private static func legacyObject(
        forKey key: String,
        in defaults: UserDefaults,
        legacyDefaults: UserDefaults?
    ) -> Any? {
        defaults.object(forKey: key) ?? legacyDefaults?.object(forKey: key)
    }

    private static func legacyString(
        forKey key: String,
        in defaults: UserDefaults,
        legacyDefaults: UserDefaults?
    ) -> String? {
        legacyObject(forKey: key, in: defaults, legacyDefaults: legacyDefaults) as? String
    }

    private func updateLaunchAtLogin() {
        guard #available(macOS 13.0, *) else { return }

        do {
            if launchAtLogin {
                if SMAppService.mainApp.status != .enabled {
                    try SMAppService.mainApp.register()
                }
            } else {
                if SMAppService.mainApp.status == .enabled {
                    try SMAppService.mainApp.unregister()
                }
            }
        } catch {
            NSLog("Launch at login update failed: \(error.localizedDescription)")
        }
    }
}

extension AppSettings: DailyNoteSettingsProviding {}

# Thread 模式实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 Trace 应用增加第三种写入模式——Thread 模式，支持用户预配置主题线程，输入内容直接追加到对应线程文件。

**Architecture:** 扩展现有 `NoteWriteMode` enum 为三态（dimension/file/thread），新增 `ThreadConfig` 模型和 `ThreadWriter` 写入逻辑，UI 层复用现有 Section 网格样式展示 Thread 选择器。

**Tech Stack:** SwiftUI, AppKit, Combine, UserDefaults

---

## Task 1: 创建 ThreadConfig 数据模型

**Files:**
- Create: `Sources/Trace/Models/ThreadConfig.swift`

**Step 1: 创建 ThreadConfig 模型**

```swift
import Foundation

struct ThreadConfig: Identifiable, Codable, Equatable {
    let id: UUID
    var name: String
    var targetFile: String
    var icon: String?
    var order: Int
    
    init(
        id: UUID = UUID(),
        name: String,
        targetFile: String,
        icon: String? = nil,
        order: Int = 0
    ) {
        self.id = id
        self.name = name
        self.targetFile = targetFile
        self.icon = icon
        self.order = order
    }
    
    static func == (lhs: ThreadConfig, rhs: ThreadConfig) -> Bool {
        lhs.id == rhs.id
    }
    
    static let `default` = ThreadConfig(
        name: "想法",
        targetFile: "Threads/想法.md",
        icon: "lightbulb"
    )
}
```

**Step 2: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add Sources/Trace/Models/ThreadConfig.swift
git commit -m "feat: add ThreadConfig data model"
```

---

## Task 2: 扩展 NoteWriteMode 为三态

**Files:**
- Modify: `Sources/Trace/Services/AppSettings.swift:89-147`

**Step 1: 修改 NoteWriteMode enum**

将现有 `NoteWriteMode` 从二态扩展为三态：

```swift
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
```

**Step 2: 删除旧的 toggled 属性**

删除旧代码：
```swift
// 删除这段代码
var toggled: NoteWriteMode {
    switch self {
    case .dimension:
        return .file
    case .file:
        return .dimension
    }
}
```

**Step 3: 更新 SettingKeys 添加 Thread 相关键**

在 `SettingKeys` 中添加：
```swift
static let threadConfigs = "trace.threadConfigs"
static let lastUsedThreadId = "trace.lastUsedThreadId"
static let threadVaultPath = "trace.threadVaultPath"
```

**Step 4: 更新 LegacySettingKeys**

添加 legacy 键（用于未来迁移）：
```swift
static let threadConfigs = "flashnote.threadConfigs"
```

**Step 5: 编译验证**

Run: `swift build`
Expected: 编译成功，可能需要修复引用 `toggled` 的地方

**Step 6: Commit**

```bash
git add Sources/Trace/Services/AppSettings.swift
git commit -m "feat: extend NoteWriteMode to three states"
```

---

## Task 3: 在 AppSettings 中添加 Thread 配置管理

**Files:**
- Modify: `Sources/Trace/Services/AppSettings.swift`

**Step 1: 添加 Published 属性**

在 `@Published var markdownEntrySeparatorStyle` 后添加：

```swift
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
```

**Step 2: 在 init 中初始化 Thread 属性**

在 `markdownEntrySeparatorStyle` 初始化后添加：

```swift
// Thread configs
if let data = defaults.data(forKey: SettingKeys.threadConfigs),
   let configs = try? JSONDecoder().decode([ThreadConfig].self, from: data) {
    threadConfigs = configs
} else {
    threadConfigs = Self.defaultThreadConfigs
}
threadVaultPath = defaults.string(forKey: SettingKeys.threadVaultPath) ?? ""
```

**Step 3: 添加默认 Thread 配置**

添加默认配置：

```swift
private static var defaultThreadConfigs: [ThreadConfig] {
    [
        ThreadConfig(name: "想法", targetFile: "Threads/想法.md", icon: "lightbulb", order: 0),
        ThreadConfig(name: "读书笔记", targetFile: "Threads/读书笔记.md", icon: "book", order: 1),
        ThreadConfig(name: "产品设计", targetFile: "Threads/产品设计.md", icon: "pencil", order: 2),
        ThreadConfig(name: "技术研究", targetFile: "Threads/技术研究.md", icon: "cpu", order: 3)
    ]
}
```

**Step 4: 添加 Thread 相关方法**

添加 Thread 管理方法：

```swift
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
    let config = ThreadConfig(
        name: name,
        targetFile: targetFile,
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
```

**Step 5: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 6: Commit**

```bash
git add Sources/Trace/Services/AppSettings.swift
git commit -m "feat: add thread configuration management to AppSettings"
```

---

## Task 4: 实现 ThreadWriter 写入逻辑

**Files:**
- Create: `Sources/Trace/Services/ThreadWriter.swift`

**Step 1: 创建 ThreadWriter 类**

```swift
import Foundation

protocol ThreadSettingsProviding {
    var threadVaultPath: String { get }
    var dailyEntryThemePreset: DailyEntryThemePreset { get }
}

enum ThreadWriterError: LocalizedError, Equatable {
    case invalidVaultPath
    case invalidTargetFilePath

    var errorDescription: String? {
        switch self {
        case .invalidVaultPath: return L10n.vaultNotConfigured
        case .invalidTargetFilePath: return L10n.invalidTargetFolder
        }
    }
}

final class ThreadWriter {
    private let settings: ThreadSettingsProviding
    private let fileManager: FileManager

    init(settings: ThreadSettingsProviding, fileManager: FileManager = .default) {
        self.settings = settings
        self.fileManager = fileManager
    }

    func save(
        text: String,
        to thread: ThreadConfig,
        mode: DailyNoteSaveMode = .createNewEntry,
        now: Date = Date()
    ) throws {
        let trimmedText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedText.isEmpty else { return }

        let fileURL = try threadFileURL(for: thread)
        try ensureDirectoryExists(at: fileURL.deletingLastPathComponent())

        let content = try loadOrCreateContent(for: fileURL)
        let updated: String

        switch mode {
        case .createNewEntry:
            updated = append(entryForText(trimmedText, at: now), to: content, for: thread)
        case .appendToLatestEntry:
            if let appended = tryAppendToLatestEntry(trimmedText, at: now, into: content) {
                updated = appended
            } else {
                updated = append(entryForText(trimmedText, at: now), to: content, for: thread)
            }
        }

        try updated.write(to: fileURL, atomically: true, encoding: .utf8)
    }

    func threadFileURL(for thread: ThreadConfig) throws -> URL {
        let vaultURL = try vaultURL()
        let normalizedPath = thread.targetFile
            .replacingOccurrences(of: "\\", with: "/")
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        
        guard !normalizedPath.isEmpty else {
            throw ThreadWriterError.invalidTargetFilePath
        }
        
        // Security: prevent path traversal
        let components = normalizedPath.split(separator: "/").map(String.init)
        for component in components {
            if component == "." || component == ".." {
                throw ThreadWriterError.invalidTargetFilePath
            }
        }
        
        // Ensure .md extension
        let fileName = normalizedPath.hasSuffix(".md") ? normalizedPath : "\(normalizedPath).md"
        
        return vaultURL.appendingPathComponent(fileName, isDirectory: false)
    }

    private func vaultURL() throws -> URL {
        let trimmedVaultPath = settings.threadVaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedVaultPath.isEmpty else {
            throw ThreadWriterError.invalidVaultPath
        }
        return URL(fileURLWithPath: trimmedVaultPath, isDirectory: true)
    }

    private func ensureDirectoryExists(at directoryURL: URL) throws {
        try fileManager.createDirectory(at: directoryURL, withIntermediateDirectories: true)
    }

    private func loadOrCreateContent(for fileURL: URL) throws -> String {
        if fileManager.fileExists(atPath: fileURL.path) {
            return try String(contentsOf: fileURL, encoding: .utf8)
        }
        return ""
    }

    private func append(_ entry: String, to content: String, for thread: ThreadConfig) -> String {
        var mutableContent = content
        
        // If file is empty or doesn't have a header, add one
        if mutableContent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return "# \(thread.name)\n\n\(entry)"
        }
        
        // Ensure there's a blank line before new entry
        if !mutableContent.hasSuffix("\n") {
            mutableContent.append("\n")
        }
        if !mutableContent.hasSuffix("\n\n") {
            mutableContent.append("\n")
        }
        
        mutableContent.append(entry)
        return mutableContent
    }

    private func entryForText(_ text: String, at date: Date) -> String {
        switch settings.dailyEntryThemePreset {
        case .codeBlockClassic:
            return """
            ## \(timestamp(for: date))
            
            ```
            \(text)
            ```
            
            ---
            
            """
        case .plainTextTimestamp:
            return """
            ## \(timestamp(for: date))
            
            \(text)
            
            ---
            
            """
        case .markdownQuote:
            let quotedText = text
                .components(separatedBy: .newlines)
                .map { line in
                    line.isEmpty ? ">" : "> \(line)"
                }
                .joined(separator: "\n")
            return """
            ## \(timestamp(for: date))
            
            \(quotedText)
            
            ---
            
            """
        }
    }

    private func tryAppendToLatestEntry(_ text: String, at date: Date, into content: String) -> String? {
        // Find the last entry and append to it
        // Look for the last "---" separator and append before it
        guard let lastSeparatorRange = content.range(of: "\n---\n", options: .backwards) else {
            return nil
        }
        
        let insertIndex = lastSeparatorRange.lowerBound
        
        let appendedContent: String
        switch settings.dailyEntryThemePreset {
        case .codeBlockClassic:
            appendedContent = "\n\n## \(timestamp(for: date))\n\n```\n\(text)\n```"
        case .plainTextTimestamp:
            appendedContent = "\n\n## \(timestamp(for: date))\n\n\(text)"
        case .markdownQuote:
            let quotedText = text
                .components(separatedBy: .newlines)
                .map { line in
                    line.isEmpty ? ">" : "> \(line)"
                }
                .joined(separator: "\n")
            appendedContent = "\n\n## \(timestamp(for: date))\n\n\(quotedText)"
        }
        
        var mutableContent = content
        mutableContent.insert(contentsOf: appendedContent, at: insertIndex)
        return mutableContent
    }

    private func timestamp(for date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd HH:mm"
        return formatter.string(from: date)
    }
}

// MARK: - AppSettings Extension

extension AppSettings: ThreadSettingsProviding {}
```

**Step 2: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add Sources/Trace/Services/ThreadWriter.swift
git commit -m "feat: add ThreadWriter for thread mode persistence"
```

---

## Task 5: 更新 DailyNoteWriter 支持三态模式

**Files:**
- Modify: `Sources/Trace/Services/DailyNoteWriter.swift`

**Step 1: 修改 save 方法签名**

修改 `save` 方法以支持 Thread 模式：

```swift
func save(
    text: String,
    to section: NoteSection,
    mode: DailyNoteSaveMode = .createNewEntry,
    documentTitle: String? = nil,
    fileTargetFolder: String? = nil,
    thread: ThreadConfig? = nil,
    now: Date = Date()
) throws {
    let trimmedText = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmedText.isEmpty else { return }

    switch settings.noteWriteMode {
    case .dimension:
        try saveToDailyNote(trimmedText, to: section, mode: mode, now: now)
    case .file:
        try saveToInboxFile(
            trimmedText,
            title: documentTitle,
            targetFolder: fileTargetFolder,
            now: now
        )
    case .thread:
        guard let thread = thread else {
            throw DailyNoteWriterError.invalidTargetFolderPath
        }
        let threadWriter = ThreadWriter(settings: settings)
        try threadWriter.save(text: trimmedText, to: thread, mode: mode, now: now)
    }
}
```

**Step 2: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add Sources/Trace/Services/DailyNoteWriter.swift
git commit -m "feat: update DailyNoteWriter to support thread mode"
```

---

## Task 6: 更新 CaptureViewModel 支持 Thread

**Files:**
- Modify: `Sources/Trace/UI/Capture/CaptureViewModel.swift`

**Step 1: 添加 Thread 相关属性**

```swift
import Foundation

final class CaptureViewModel: ObservableObject {
    @Published var text: String = ""
    @Published var selectedSection: NoteSection = .note
    @Published var selectedThread: ThreadConfig? = nil
    @Published var fileTitle: String = ""
    @Published var pinned: Bool = false
    @Published var toastMessage: String?

    func resetInput() {
        text = ""
        fileTitle = ""
    }

    func showToast(_ message: String, duration: TimeInterval = 1.5) {
        toastMessage = message
        DispatchQueue.main.asyncAfter(deadline: .now() + duration) { [weak self] in
            if self?.toastMessage == message {
                self?.toastMessage = nil
            }
        }
    }
}
```

**Step 2: Commit**

```bash
git add Sources/Trace/UI/Capture/CaptureViewModel.swift
git commit -m "feat: add thread selection to CaptureViewModel"
```

---

## Task 7: 更新 CaptureView 添加 Thread 选择器 UI

**Files:**
- Modify: `Sources/Trace/UI/Capture/CaptureView.swift`

**Step 1: 添加 Thread 相关属性**

在 `sectionGridSpacing` 后添加：

```swift
@State private var threadGridWidth: CGFloat = 0
```

**Step 2: 添加 threadFooter**

在 `documentFooter` 后添加：

```swift
private var threadFooter: some View {
    VStack(spacing: 0) {
        Divider().overlay(theme.border)
        
        threadButtons
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
    }
    .background(theme.chromeBackground)
    .onPreferenceChange(ThreadGridWidthPreferenceKey.self) { width in
        threadGridWidth = width
    }
}

private var threadButtons: some View {
    LazyVGrid(columns: threadGridColumns, spacing: sectionGridSpacing) {
        ForEach(settings.threadConfigs.sorted(by: { $0.order < $1.order })) { thread in
            Button {
                viewModel.selectedThread = thread
            } label: {
                HStack(spacing: 4) {
                    if let icon = thread.icon {
                        Image(systemName: icon)
                            .font(.system(size: 11))
                    }
                    Text(thread.name)
                        .lineLimit(1)
                }
                .font(.system(size: 12, weight: viewModel.selectedThread?.id == thread.id ? .semibold : .medium))
                .foregroundStyle(viewModel.selectedThread?.id == thread.id ? theme.selectedText : theme.textSecondary)
                .padding(.horizontal, 8)
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity)
                .frame(minHeight: 34)
                .background(viewModel.selectedThread?.id == thread.id ? theme.accentStrong : theme.surface.opacity(0.6))
                .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
            }
            .buttonStyle(.plain)
        }
    }
    .background(
        GeometryReader { proxy in
            Color.clear.preference(key: ThreadGridWidthPreferenceKey.self, value: proxy.size.width)
        }
    )
}

private var threadGridColumns: [GridItem] {
    let columnCount = threadColumnCount(for: threadGridWidth, itemCount: settings.threadConfigs.count)
    return Array(
        repeating: GridItem(.flexible(), spacing: sectionGridSpacing, alignment: .top),
        count: columnCount
    )
}

private func threadColumnCount(for width: CGFloat, itemCount: Int) -> Int {
    guard itemCount > 0 else { return 1 }
    
    let minimumColumnsForThreeRows = Int(ceil(Double(itemCount) / Double(maximumSectionRows)))
    guard width > 0 else { return min(itemCount, max(1, minimumColumnsForThreeRows)) }
    
    let widthBasedColumns = max(
        1,
        Int((width + sectionGridSpacing) / (minimumSectionButtonWidth + sectionGridSpacing))
    )
    return min(itemCount, max(minimumColumnsForThreeRows, widthBasedColumns))
}
```

**Step 3: 添加 PreferenceKey**

在文件顶部 `SectionGridWidthPreferenceKey` 后添加：

```swift
private struct ThreadGridWidthPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}
```

**Step 4: 更新 body 中的 switch 语句**

```swift
var body: some View {
    VStack(spacing: 0) {
        header
        Divider().overlay(theme.border)
        editor
        switch settings.noteWriteMode {
        case .dimension:
            modeFooter
        case .file:
            documentFooter
        case .thread:
            threadFooter
        }
    }
    ...
}
```

**Step 5: 更新 editorPlaceholder**

```swift
private var editorPlaceholder: String {
    switch settings.noteWriteMode {
    case .dimension: return L10n.notePlaceholder
    case .file: return L10n.documentPlaceholder
    case .thread: return L10n.threadPlaceholder
    }
}
```

**Step 6: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 7: Commit**

```bash
git add Sources/Trace/UI/Capture/CaptureView.swift
git commit -m "feat: add thread selector UI to CaptureView"
```

---

## Task 8: 更新 CapturePanelController 支持 Thread 模式

**Files:**
- Modify: `Sources/Trace/UI/Capture/CapturePanelController.swift`

**Step 1: 注入 ThreadWriter**

在 `clipboardImageWriter` 后添加：

```swift
private lazy var threadWriter = ThreadWriter(settings: settings)
```

**Step 2: 更新 show 方法初始化 Thread**

在 `show` 方法中：

```swift
viewModel.selectedSection = settings.defaultSection
viewModel.selectedThread = settings.defaultThread  // 添加这行
```

**Step 3: 更新 mode toggle 快捷键处理**

修改 toggle 逻辑：

```swift
if matchesShortcut(
    event,
    keyCode: settings.modeToggleKeyCode,
    modifiers: settings.modeToggleModifiers
) {
    settings.noteWriteMode = settings.noteWriteMode.next()  // 改为 next()
    NotificationCenter.default.post(name: .traceFocusInput, object: nil)
    return nil
}
```

**Step 4: 更新 saveCurrentNote 方法**

```swift
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
```

**Step 5: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 6: Commit**

```bash
git add Sources/Trace/UI/Capture/CapturePanelController.swift
git commit -m "feat: update CapturePanelController for thread mode"
```

---

## Task 9: 更新 SettingsView 添加 Thread 管理界面

**Files:**
- Modify: `Sources/Trace/UI/Settings/SettingsView.swift`

**Step 1: 在 Storage Section 添加 Thread 模式配置**

在 `if settings.noteWriteMode == .file { ... }` 后添加：

```swift
if settings.noteWriteMode == .thread {
    SettingRow(label: L10n.vault, hint: L10n.vaultHintThread, palette: palette) {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                TextField("/Users/you/Vault", text: $settings.threadVaultPath)
                    .textFieldStyle(.plain)
                    .settingsFieldChrome(palette)

                Button(L10n.browse) {
                    chooseFolderPath(binding: $settings.threadVaultPath)
                }
                .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))
            }

            if let issue = settings.threadVaultPathValidationIssue {
                Text(issue.message)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(palette.warningText)
            }
        }
    }
}
```

**Step 2: 添加 Thread 管理 Section**

在 Modules Section 后添加 Thread 管理 Section：

```swift
// Thread Management
SectionCard(title: L10n.threadManagement, palette: palette) {
    VStack(alignment: .leading, spacing: 8) {
        ForEach(settings.threadConfigs.sorted(by: { $0.order < $1.order })) { thread in
            ThreadConfigRow(
                thread: thread,
                settings: settings,
                palette: palette
            )
        }

        HStack {
            Button {
                settings.addThread(
                    name: L10n.newThreadDefaultName,
                    targetFile: "Threads/\(L10n.newThreadDefaultName).md"
                )
            } label: {
                Label(L10n.addThread, systemImage: "plus")
            }
            .buttonStyle(SettingsSecondaryButtonStyle(palette: palette))
            .disabled(!settings.canAddThread)

            Spacer()

            Button(L10n.save) {
                NSApp.keyWindow?.makeFirstResponder(nil)
            }
            .buttonStyle(SettingsPrimaryButtonStyle(palette: palette))
        }
    }
}
```

**Step 3: 在文件末尾添加 ThreadConfigRow 组件**

```swift
// MARK: - Thread Config Row

private struct ThreadConfigRow: View {
    let thread: ThreadConfig
    @ObservedObject var settings: AppSettings
    let palette: SettingsPalette

    @State private var nameDraft: String = ""
    @State private var fileDraft: String = ""
    @FocusState private var isNameFocused: Bool
    @FocusState private var isFileFocused: Bool

    var body: some View {
        HStack(spacing: 8) {
            TextField(L10n.threadName, text: $nameDraft)
                .textFieldStyle(.plain)
                .settingsFieldChrome(palette)
                .focused($isNameFocused)
                .onSubmit { commitName() }
                .onChange(of: isNameFocused) { focused in
                    if !focused { commitName() }
                }
                .frame(width: 120)

            TextField(L10n.threadTargetFile, text: $fileDraft)
                .textFieldStyle(.plain)
                .settingsFieldChrome(palette)
                .focused($isFileFocused)
                .onSubmit { commitFile() }
                .onChange(of: isFileFocused) { focused in
                    if !focused { commitFile() }
                }

            Button {
                settings.removeThread(thread)
            } label: {
                Image(systemName: "minus.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(settings.canRemoveThread ? palette.warningText : palette.mutedText)
            }
            .buttonStyle(.plain)
            .help(L10n.deleteThread)
            .disabled(!settings.canRemoveThread)
        }
        .onAppear {
            nameDraft = thread.name
            fileDraft = thread.targetFile
        }
        .onChange(of: settings.threadConfigs) { _ in
            if !isNameFocused && !isFileFocused {
                nameDraft = thread.name
                fileDraft = thread.targetFile
            }
        }
    }

    private func commitName() {
        let trimmed = nameDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            nameDraft = thread.name
        } else {
            var updated = thread
            updated.name = trimmed
            settings.updateThread(updated)
        }
    }

    private func commitFile() {
        let trimmed = fileDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            fileDraft = thread.targetFile
        } else {
            var updated = thread
            updated.targetFile = trimmed
            settings.updateThread(updated)
        }
    }
}
```

**Step 4: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 5: Commit**

```bash
git add Sources/Trace/UI/Settings/SettingsView.swift
git commit -m "feat: add thread management UI in settings"
```

---

## Task 10: 更新本地化字符串 (L10n)

**Files:**
- Modify: `Sources/Trace/Utils/L10n.swift`

**Step 1: 添加 Thread 相关本地化字符串**

添加以下属性（根据现有语言支持中文、英文、日文）：

```swift
// MARK: - Thread Mode

static var writeModeThreadTitle: String {
    switch current {
    case .zh: return "线程模式"
    case .en: return "Thread Mode"
    case .ja: return "スレッドモード"
    }
}

static var writeModeThreadCompact: String {
    switch current {
    case .zh: return "线程"
    case .en: return "Thread"
    case .ja: return "スレッド"
    }
}

static var writeModeThreadDestination: String {
    switch current {
    case .zh: return "线程文件"
    case .en: return "Thread file"
    case .ja: return "スレッドファイル"
    }
}

static var writeModeThreadSummary: String {
    switch current {
    case .zh: return "追加到主题线程"
    case .en: return "Append to topic thread"
    case .ja: return "トピックスレッドに追加"
    }
}

static var writeModeThreadTarget: String {
    switch current {
    case .zh: return "线程文件"
    case .en: return "Thread file"
    case .ja: return "スレッドファイル"
    }
}

static var threadPlaceholder: String {
    switch current {
    case .zh: return "输入想法，追加到选中线程..."
    case .en: return "Type your thought to append to thread..."
    case .ja: return "スレッドに追加するアイデアを入力..."
    }
}

static var noThreadSelected: String {
    switch current {
    case .zh: return "请选择一个线程"
    case .en: return "Please select a thread"
    case .ja: return "スレッドを選択してください"
    }
}

static var vaultHintThread: String {
    switch current {
    case .zh: return "线程文件将保存在此目录"
    case .en: return "Thread files will be saved here"
    case .ja: return "スレッドファイルはここに保存されます"
    }
}

static var threadManagement: String {
    switch current {
    case .zh: return "线程管理"
    case .en: return "Thread Management"
    case .ja: return "スレッド管理"
    }
}

static var newThreadDefaultName: String {
    switch current {
    case .zh: return "新线程"
    case .en: return "New Thread"
    case .ja: return "新しいスレッド"
    }
}

static var addThread: String {
    switch current {
    case .zh: return "添加线程"
    case .en: return "Add Thread"
    case .ja: return "スレッドを追加"
    }
}

static var deleteThread: String {
    switch current {
    case .zh: return "删除线程"
    case .en: return "Delete Thread"
    case .ja: return "スレッドを削除"
    }
}

static var threadName: String {
    switch current {
    case .zh: return "名称"
    case .en: return "Name"
    case .ja: return "名前"
    }
}

static var threadTargetFile: String {
    switch current {
    case .zh: return "目标文件路径"
    case .en: return "Target file path"
    case .ja: return "対象ファイルパス"
    }
}

static var writeModeDailyCompact: String {
    switch current {
    case .zh: return "Daily"
    case .en: return "Daily"
    case .ja: return "Daily"
    }
}

static var writeModeDocumentCompact: String {
    switch current {
    case .zh: return "文档"
    case .en: return "Doc"
    case .ja: return "ドキュメント"
    }
}
```

**Step 2: 编译验证**

Run: `swift build`
Expected: 编译成功

**Step 3: Commit**

```bash
git add Sources/Trace/Utils/L10n.swift
git commit -m "feat: add thread mode localization strings"
```

---

## Task 11: 更新 AppDelegate 初始化

**Files:**
- Modify: `Sources/Trace/App/AppDelegate.swift`

**Step 1: 确认 ThreadWriter 注入**

检查 `AppDelegate` 中的 `CapturePanelController` 初始化：

```swift
private lazy var capturePanelController = CapturePanelController(
    settings: AppSettings.shared,
    writer: DailyNoteWriter(settings: AppSettings.shared)
)
```

**Step 2: 确保 ThreadWriter 懒加载正确**

在 `CapturePanelController` 中我们已经添加了懒加载的 `threadWriter`，不需要在 AppDelegate 中修改。

**Step 3: Commit（如有修改）**

```bash
git add Sources/Trace/App/AppDelegate.swift  # 如果有修改
git commit -m "chore: update AppDelegate for thread support" 2>/dev/null || echo "No changes to commit"
```

---

## Task 12: 运行测试

**Files:**
- Run: `swift test`

**Step 1: 运行测试**

Run: `swift test`
Expected: 所有测试通过

**Step 2: 编译验证**

Run: `swift build`
Expected: 编译成功，无警告

**Step 3: Commit**

```bash
git commit -m "test: verify thread mode implementation" --allow-empty
```

---

## Task 13: 功能验证清单

手动验证以下功能：

- [ ] 快捷键 `Shift+Tab` 可以循环切换三种模式
- [ ] Thread 模式显示线程选择网格
- [ ] 选择线程后输入内容可以保存到对应文件
- [ ] 设置面板可以添加/删除/修改线程
- [ ] 线程文件路径支持子目录（如 `Threads/工作/项目.md`）
- [ ] 文件首次创建时自动添加 `# 线程名` 标题
- [ ] 后续输入追加为 `## 时间戳` 条目
- [ ] 三种模式的本地化字符串正确显示
- [ ] 上次使用的线程会被记住
- [ ] Daily 模式和文档模式功能不受影响

---

## 实现完成总结

**新增文件：**
- `Sources/Trace/Models/ThreadConfig.swift`
- `Sources/Trace/Services/ThreadWriter.swift`

**修改文件：**
- `Sources/Trace/Services/AppSettings.swift`
- `Sources/Trace/Services/DailyNoteWriter.swift`
- `Sources/Trace/UI/Capture/CaptureViewModel.swift`
- `Sources/Trace/UI/Capture/CaptureView.swift`
- `Sources/Trace/UI/Capture/CapturePanelController.swift`
- `Sources/Trace/UI/Settings/SettingsView.swift`
- `Sources/Trace/Utils/L10n.swift`

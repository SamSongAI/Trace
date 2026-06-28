import Foundation

protocol DailyNoteSettingsProviding: ThreadSettingsProviding {
    var vaultPath: String { get }
    var dailyFolderName: String { get }
    var dailyFileDateFormat: String { get }
    var noteWriteMode: NoteWriteMode { get }
    var markdownEntrySeparatorStyle: MarkdownEntrySeparatorStyle { get }
    func title(for section: NoteSection) -> String
    func header(for section: NoteSection) -> String
}

enum DailyNoteSaveMode {
    case createNewEntry
    case appendToLatestEntry
}

enum DailyNoteWriterError: LocalizedError, Equatable {
    case invalidVaultPath
    case invalidTargetFolderPath

    var errorDescription: String? {
        switch self {
        case .invalidVaultPath: return L10n.vaultNotConfigured
        case .invalidTargetFolderPath: return L10n.invalidTargetFolder
        }
    }
}

final class DailyNoteWriter {
    private let settings: DailyNoteSettingsProviding
    private let fileManager: FileManager

    init(settings: DailyNoteSettingsProviding, fileManager: FileManager = .default) {
        self.settings = settings
        self.fileManager = fileManager
    }

    func save(
        text: String,
        to section: NoteSection,
        mode: DailyNoteSaveMode = .createNewEntry,
        thread: ThreadConfig? = nil,
        now: Date = Date()
    ) throws {
        let trimmedText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedText.isEmpty else { return }

        switch settings.noteWriteMode {
        case .dimension:
            try saveToDailyNote(trimmedText, to: section, mode: mode, now: now)
        case .thread:
            guard let thread = thread else {
                throw DailyNoteWriterError.invalidTargetFolderPath
            }
            let threadWriter = ThreadWriter(settings: settings)
            try threadWriter.save(text: trimmedText, to: thread, mode: mode, now: now)
        case .agent:
            break
        }
    }

    private func saveToDailyNote(
        _ text: String,
        to section: NoteSection,
        mode: DailyNoteSaveMode,
        now: Date
    ) throws {
        let dailyNoteURL = try dailyNoteFileURL(for: now)
        try ensureDailyDirectoryExists(at: dailyNoteURL.deletingLastPathComponent())

        let content = try loadOrCreateContent(for: dailyNoteURL)
        let updated: String

        switch mode {
        case .createNewEntry:
            updated = insert(entryForText(text, at: now), into: content, under: section)
        case .appendToLatestEntry:
            if let appended = appendLatestEntry(text, at: now, into: content, under: section) {
                updated = appended
            } else {
                updated = insert(entryForText(text, at: now), into: content, under: section)
            }
        }

        try updated.write(to: dailyNoteURL, atomically: true, encoding: .utf8)
    }

    func dailyNoteFileURL(for date: Date = Date()) throws -> URL {
        let vaultURL = try vaultURL()
        let dailyFolderName = normalizedFolderName(settings.dailyFolderName, fallback: "Daily")
        let dailyDirectoryURL = vaultURL.appendingPathComponent(dailyFolderName, isDirectory: true)
        let fileName = formattedFileName(for: date)

        return dailyDirectoryURL.appendingPathComponent(fileName, isDirectory: false)
    }

    private func vaultURL() throws -> URL {
        let trimmedVaultPath = settings.vaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedVaultPath.isEmpty else {
            throw DailyNoteWriterError.invalidVaultPath
        }
        return URL(fileURLWithPath: trimmedVaultPath, isDirectory: true)
    }

    private func normalizedFolderName(_ folderName: String, fallback: String) -> String {
        let trimmed = folderName.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? fallback : trimmed
    }

    private func ensureDailyDirectoryExists(at directoryURL: URL) throws {
        try fileManager.createDirectory(at: directoryURL, withIntermediateDirectories: true)
    }

    private func loadOrCreateContent(for fileURL: URL) throws -> String {
        if fileManager.fileExists(atPath: fileURL.path) {
            return try String(contentsOf: fileURL, encoding: .utf8)
        }

        try ensureCSSExists(at: fileURL)
        return htmlTemplate(for: fileURL)
    }

    private func ensureCSSExists(at dailyFileURL: URL) throws {
        let vaultURL = dailyFileURL.deletingLastPathComponent().deletingLastPathComponent()
        let traceDir = vaultURL.appendingPathComponent(".trace", isDirectory: true)
        let cssURL = traceDir.appendingPathComponent("style.css", isDirectory: false)

        guard !fileManager.fileExists(atPath: cssURL.path) else { return }

        try fileManager.createDirectory(at: traceDir, withIntermediateDirectories: true)
        let css = """
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
            background: #fafafa;
            color: #333;
            line-height: 1.6;
            padding: 24px;
            max-width: 720px;
            margin: 0 auto;
        }
        h1 {
            font-size: 22px;
            font-weight: 700;
            margin-bottom: 20px;
            color: #1a1a1a;
        }
        section { margin-bottom: 28px; }
        h2 {
            font-size: 15px;
            font-weight: 600;
            color: #666;
            margin-bottom: 12px;
            padding-bottom: 6px;
            border-bottom: 1px solid #eee;
        }
        .card {
            background: #fff;
            border-radius: 10px;
            padding: 14px 16px;
            margin-bottom: 10px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.06);
        }
        .card p {
            font-size: 14px;
            white-space: pre-wrap;
            word-break: break-word;
        }
        .card time {
            display: block;
            font-size: 11px;
            color: #999;
            margin-top: 8px;
        }
        .card .separator {
            border: none;
            border-top: 1px dashed #e0e0e0;
            margin: 10px 0;
        }
        code {
            font-family: "SF Mono", Menlo, monospace;
            font-size: 12px;
            background: #f0f0f0;
            padding: 2px 5px;
            border-radius: 4px;
        }
        pre {
            background: #f5f5f5;
            padding: 12px;
            border-radius: 8px;
            overflow-x: auto;
            margin: 8px 0;
        }
        pre code { background: none; padding: 0; }
        img { max-width: 100%; border-radius: 8px; margin: 8px 0; }
        a { color: #4A90D9; text-decoration: none; }
        a:hover { text-decoration: underline; }
        """
        try css.write(to: cssURL, atomically: true, encoding: .utf8)
    }

    private func htmlTemplate(for fileURL: URL) -> String {
        let fileName = fileURL.deletingPathExtension().lastPathComponent
        return """
        <!DOCTYPE html>
        <html lang="zh">
        <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link rel="stylesheet" href="../.trace/style.css">
        <title>\(fileName)</title>
        </head>
        <body>
        <h1>\(fileName)</h1>
        </body>
        </html>
        """
    }

    private func insert(_ entry: String, into content: String, under section: NoteSection) -> String {
        let sectionTitle = settings.title(for: section)
        let sectionTag = "<section data-section=\"\(sectionTitle)\">"
        let h2Tag = "<h2>\(sectionTitle)</h2>"

        // Section already exists — insert card before </section>
        if let sectionRange = content.range(of: sectionTag) {
            let afterSection = sectionRange.upperBound
            // Find </section> for this section
            if let closeRange = content[afterSection...].range(of: "</section>") {
                var mutable = content
                mutable.insert(contentsOf: "\n\(entry)", at: closeRange.lowerBound)
                return mutable
            }
        }

        // Section doesn't exist — create it before </body>
        let sectionHtml = "\n\(sectionTag)\n\(h2Tag)\n\(entry)\n</section>"
        if let bodyCloseRange = content.range(of: "</body>") {
            var mutable = content
            mutable.insert(contentsOf: sectionHtml, at: bodyCloseRange.lowerBound)
            return mutable
        }

        // Fallback: append
        return content + sectionHtml
    }

    private func entryForText(_ text: String, at date: Date) -> String {
        let escaped = htmlEscape(text)
        let time = timestamp(for: date)
        return """
        <div class="card">
        <p>\(escaped)</p>
        <time>\(time)</time>
        </div>
        """
    }

    private func appendLatestEntry(_ text: String, at date: Date, into content: String, under section: NoteSection) -> String? {
        let sectionTitle = settings.title(for: section)
        let sectionTag = "<section data-section=\"\(sectionTitle)\">"

        guard let sectionRange = content.range(of: sectionTag) else { return nil }
        let afterSection = sectionRange.upperBound

        // Find the last </time></div> in this section, insert before </div>
        guard let closeSectionRange = content[afterSection...].range(of: "</section>") else { return nil }
        let sectionBody = content[afterSection..<closeSectionRange.lowerBound]

        guard let lastTimeRange = sectionBody.range(of: "</time>", options: .backwards) else { return nil }
        // Find the </div> after this </time>
        let afterTime = lastTimeRange.upperBound
        guard let divCloseRange = content[afterTime...].range(of: "</div>") else { return nil }

        let escaped = htmlEscape(text)
        let time = timestamp(for: date)
        let chunk = """
        \n<hr class="separator">\n<p>\(escaped)</p>\n<time>\(time)</time>
        """

        var mutable = content
        mutable.insert(contentsOf: chunk, at: divCloseRange.lowerBound)
        return mutable
    }

    private func htmlEscape(_ text: String) -> String {
        text
            .replacingOccurrences(of: "&", with: "&amp;")
            .replacingOccurrences(of: "<", with: "&lt;")
            .replacingOccurrences(of: ">", with: "&gt;")
            .replacingOccurrences(of: "\"", with: "&quot;")
    }

    private func timestamp(for date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd HH:mm"
        return formatter.string(from: date)
    }

    private func formattedFileName(for date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "zh_CN")
        formatter.dateFormat = settings.dailyFileDateFormat
        return formatter.string(from: date) + ".html"
    }

    private func fileNameTimestampWithoutMilliseconds(for date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd-HHmmss"
        return formatter.string(from: date)
    }
}

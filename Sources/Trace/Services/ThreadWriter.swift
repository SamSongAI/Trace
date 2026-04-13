import Foundation

protocol ThreadSettingsProviding {
    var vaultPath: String { get }
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
    private let coordinator = NSFileCoordinator(filePresenter: nil)

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

        // Use file coordinator for thread-safe access
        var nsError: NSError?
        var writeError: Error?
        coordinator.coordinate(writingItemAt: fileURL, options: .forMerging, error: &nsError) { url in
            do {
                let content = try loadOrCreateContent(for: url)
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

                try updated.write(to: url, atomically: true, encoding: .utf8)
            } catch {
                writeError = error
            }
        }

        if let error = writeError ?? nsError {
            throw error
        }
    }

    func threadFileURL(for thread: ThreadConfig) throws -> URL {
        let vaultURL = try vaultURL()
        let normalizedPath = thread.targetFile
            .replacingOccurrences(of: "\\", with: "/")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        guard !normalizedPath.isEmpty else {
            throw ThreadWriterError.invalidTargetFilePath
        }

        // Handle absolute path (outside vault)
        if normalizedPath.hasPrefix("/") {
            // Security: resolve symlinks and check for path traversal
            let fileURL = URL(fileURLWithPath: normalizedPath, isDirectory: false)
            let resolvedURL = fileURL.resolvingSymlinksInPath()
            let resolvedPath = resolvedURL.path

            // Security: verify no ".." components in the original path
            let components = normalizedPath.split(separator: "/").map(String.init)
            for component in components {
                if component == "." || component == ".." || component.isEmpty {
                    throw ThreadWriterError.invalidTargetFilePath
                }
            }

            // Ensure .md extension
            let finalPath = resolvedPath.hasSuffix(".md") ? resolvedPath : "\(resolvedPath).md"
            return URL(fileURLWithPath: finalPath, isDirectory: false)
        }

        // Handle relative path (within vault)
        let fileURL = vaultURL.appendingPathComponent(normalizedPath, isDirectory: false)
        let resolvedURL = fileURL.resolvingSymlinksInPath()
        let resolvedVaultURL = vaultURL.resolvingSymlinksInPath()

        // Ensure resolved path is within vault
        let resolvedPath = resolvedURL.path
        let vaultPath = resolvedVaultURL.path

        guard resolvedPath.hasPrefix(vaultPath + "/") || resolvedPath == vaultPath else {
            throw ThreadWriterError.invalidTargetFilePath
        }

        // Security: verify no ".." components after resolution
        let relativePath = String(resolvedPath.dropFirst(vaultPath.count))
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let components = relativePath.split(separator: "/").map(String.init)
        for component in components {
            if component == "." || component == ".." || component.isEmpty {
                throw ThreadWriterError.invalidTargetFilePath
            }
        }

        // Ensure .md extension
        let finalPath = resolvedPath.hasSuffix(".md") ? resolvedPath : "\(resolvedPath).md"
        return URL(fileURLWithPath: finalPath, isDirectory: false)
    }

    private func vaultURL() throws -> URL {
        let trimmedVaultPath = settings.vaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
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

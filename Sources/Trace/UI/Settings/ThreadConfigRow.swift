import SwiftUI

// MARK: - Thread Config Row

struct ThreadConfigRow: View {
    let thread: ThreadConfig
    @ObservedObject var settings: AppSettings
    let palette: TraceTheme.SettingsPalette

    @State private var nameDraft: String = ""
    @State private var folderDraft: String = ""
    @State private var filenameDraft: String = ""

    @FocusState private var isNameFocused: Bool
    @FocusState private var isFolderFocused: Bool
    @FocusState private var isFilenameFocused: Bool

    var body: some View {
        HStack(spacing: 8) {
            // 1. Module name (thread name)
            TextField(L10n.threadName, text: $nameDraft)
                .textFieldStyle(.plain)
                .font(.system(size: 13, weight: .medium))
                .foregroundColor(palette.fieldText)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(palette.fieldBackground)
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(palette.fieldBorder, lineWidth: 1)
                )
                .focused($isNameFocused)
                .onSubmit { commitName() }
                .onChange(of: isNameFocused) { focused in
                    if !focused { commitName() }
                }
                .frame(width: 100)

            // 2. Folder path
            HStack(spacing: 6) {
                TextField(L10n.folderPath, text: $folderDraft)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundColor(palette.fieldText)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .focused($isFolderFocused)
                    .onSubmit { commitFolder() }
                    .onChange(of: isFolderFocused) { focused in
                        if !focused { commitFolder() }
                    }

                Divider()
                    .frame(height: 16)
                    .overlay(palette.fieldBorder)

                Button(L10n.chooseFolder) {
                    chooseFolder()
                }
                .font(.system(size: 11, weight: .medium))
                .buttonStyle(.plain)
                .foregroundColor(palette.accent)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(palette.fieldBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .stroke(palette.fieldBorder, lineWidth: 1)
            )
            .frame(minWidth: 380, maxWidth: .infinity)

            // 3. Filename
            TextField(L10n.fileName, text: $filenameDraft)
                .textFieldStyle(.plain)
                .font(.system(size: 12, weight: .medium))
                .foregroundColor(palette.fieldText)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(palette.fieldBackground)
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(palette.fieldBorder, lineWidth: 1)
                )
                .focused($isFilenameFocused)
                .onSubmit { commitFilename() }
                .onChange(of: isFilenameFocused) { focused in
                    if !focused { commitFilename() }
                }
                .frame(width: 120)

            // Delete button
            Button {
                settings.removeThread(thread)
            } label: {
                Image(systemName: "minus.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundColor(settings.canRemoveThread ? palette.warningText : palette.mutedText)
            }
            .buttonStyle(.plain)
            .help(L10n.deleteThread)
            .disabled(!settings.canRemoveThread)
        }
        .onAppear {
            loadFromThread()
        }
        .onChange(of: settings.threadConfigs) { newConfigs in
            guard let updatedThread = newConfigs.first(where: { $0.id == thread.id }) else { return }
            if !isNameFocused && !isFolderFocused && !isFilenameFocused {
                loadFrom(updatedThread)
            }
        }
    }

    private func loadFromThread() {
        nameDraft = thread.name
        let (folder, filename) = parseTargetFile(thread.targetFile)
        folderDraft = folder
        filenameDraft = filename
    }

    private func loadFrom(_ config: ThreadConfig) {
        nameDraft = config.name
        let (folder, filename) = parseTargetFile(config.targetFile)
        folderDraft = folder
        filenameDraft = filename
    }

    /// Parse targetFile into (folder, filename)
    /// e.g., "Projects/OpenClaw/notes.md" -> ("Projects/OpenClaw", "notes.md")
    /// e.g., "notes.md" -> ("", "notes.md")
    /// e.g., "/Users/.../Projects/OpenClaw/notes.md" -> ("/Users/.../Projects/OpenClaw", "notes.md")
    private func parseTargetFile(_ targetFile: String) -> (folder: String, filename: String) {
        let normalized = targetFile.replacingOccurrences(of: "\\", with: "/")

        // Handle absolute path
        if normalized.hasPrefix("/") {
            guard let lastSlash = normalized.lastIndex(of: "/") else {
                return ("", normalized)
            }
            let folder = String(normalized[..<lastSlash])
            let filename = String(normalized[normalized.index(after: lastSlash)...])
            return (folder, filename)
        }

        // Handle relative path
        guard let lastSlash = normalized.lastIndex(of: "/") else {
            return ("", normalized)
        }
        let folder = String(normalized[..<lastSlash])
        let filename = String(normalized[normalized.index(after: lastSlash)...])
        return (folder, filename)
    }

    /// Build targetFile from folder and filename
    private func buildTargetFile(folder: String, filename: String) -> String {
        let trimmedFolder = folder.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedFilename = filename.trimmingCharacters(in: .whitespacesAndNewlines)

        if trimmedFolder.isEmpty {
            return trimmedFilename
        }
        // Handle absolute path
        if trimmedFolder.hasPrefix("/") {
            return "\(trimmedFolder)/\(trimmedFilename)"
        }
        return "\(trimmedFolder)/\(trimmedFilename)"
    }

    private func commitName() {
        let trimmed = nameDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            nameDraft = thread.name
        } else if trimmed != thread.name {
            var updated = thread
            updated.name = trimmed
            settings.updateThread(updated)
        }
    }

    private func commitFolder() {
        let newTargetFile = buildTargetFile(folder: folderDraft, filename: filenameDraft)
        if newTargetFile != thread.targetFile {
            var updated = thread
            updated.targetFile = newTargetFile
            settings.updateThread(updated)
        }
    }

    private func commitFilename() {
        let trimmed = filenameDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        let finalFilename = trimmed.isEmpty ? "Threads.md" : trimmed
        if trimmed.isEmpty {
            filenameDraft = finalFilename
        }
        let newTargetFile = buildTargetFile(folder: folderDraft, filename: finalFilename)
        if newTargetFile != thread.targetFile {
            var updated = thread
            updated.targetFile = newTargetFile
            settings.updateThread(updated)
        }
    }

    private func chooseFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = L10n.chooseFolder

        // Default to current folder if exists
        if !folderDraft.isEmpty {
            let folderPath: String
            if folderDraft.hasPrefix("/") {
                folderPath = folderDraft
            } else if !settings.vaultPath.isEmpty {
                folderPath = "\(settings.vaultPath)/\(folderDraft)"
            } else {
                folderPath = folderDraft
            }
            panel.directoryURL = URL(fileURLWithPath: folderPath)
        } else {
            let desktopPath = NSSearchPathForDirectoriesInDomains(.desktopDirectory, .userDomainMask, true).first
            if let desktopPath = desktopPath {
                panel.directoryURL = URL(fileURLWithPath: desktopPath)
            }
        }

        guard panel.runModal() == .OK, let url = panel.url else { return }

        let vaultPath = settings.vaultPath
        let vaultURL = URL(fileURLWithPath: vaultPath)

        // Determine if within vault (use "vault/" prefix check to avoid matching "/vault-backup")
        let vaultPathWithSlash = vaultURL.path.hasSuffix("/") ? vaultURL.path : vaultURL.path + "/"
        if !vaultPath.isEmpty && (url.path == vaultURL.path || url.path.hasPrefix(vaultPathWithSlash)) {
            // Within vault - use relative path
            let relativeFolder: String
            if url.path == vaultURL.path || url.path == vaultURL.path + "/" {
                relativeFolder = ""
            } else {
                var relative = url.path
                if relative.hasPrefix(vaultURL.path + "/") {
                    relative = String(relative.dropFirst(vaultURL.path.count + 1))
                } else if relative.hasPrefix(vaultURL.path) {
                    relative = String(relative.dropFirst(vaultURL.path.count))
                }
                relativeFolder = relative.hasPrefix("/") ? String(relative.dropFirst()) : relative
            }
            folderDraft = relativeFolder
        } else {
            // Outside vault - use absolute path
            folderDraft = url.path
        }

        // Auto-set filename if empty
        if filenameDraft.isEmpty {
            filenameDraft = "Threads.md"
        }

        commitFolder()
    }
}

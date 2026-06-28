import Foundation

private func resolveVaultPath(_ path: String, vaultPath: String) -> String {
    if path.hasPrefix("/") {
        let standardized = (path as NSString).standardizingPath
        let vaultStandardized = (vaultPath as NSString).standardizingPath
        if standardized.hasPrefix(vaultStandardized) {
            return standardized
        }
        return (vaultPath as NSString).appendingPathComponent(path)
    }
    return (vaultPath as NSString).appendingPathComponent(path)
}

struct ReadFileTool: AgentTool {
    let name = "read_file"
    let description = "Read the contents of a file at the given absolute or vault-relative path."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "path": [
                    "type": "string",
                    "description": "Absolute path or path relative to the vault root."
                ]
            ],
            "required": ["path"]
        ]
    }

    private let vaultPath: String

    init(vaultPath: String) {
        self.vaultPath = vaultPath
    }

    private func resolvePath(_ path: String) -> String {
        resolveVaultPath(path, vaultPath: vaultPath)
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let path = arguments["path"] as? String else {
            return "Error: Missing 'path' parameter."
        }

        let resolvedPath = resolvePath(path)
        guard FileManager.default.fileExists(atPath: resolvedPath) else {
            return "Error: File not found at \(resolvedPath)"
        }

        let content = try String(contentsOfFile: resolvedPath, encoding: .utf8)
        if content.count > 10000 {
            return String(content.prefix(10000)) + "\n\n... (truncated, file is \(content.count) chars)"
        }
        return content
    }
}

struct WriteFileTool: AgentTool {
    let name = "write_file"
    let description = "Write content to a file at the given path. Creates parent directories if needed."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "path": [
                    "type": "string",
                    "description": "Absolute path or path relative to the vault root."
                ],
                "content": [
                    "type": "string",
                    "description": "The content to write to the file."
                ]
            ],
            "required": ["path", "content"]
        ]
    }

    private let vaultPath: String

    init(vaultPath: String) {
        self.vaultPath = vaultPath
    }

    private func resolvePath(_ path: String) -> String {
        resolveVaultPath(path, vaultPath: vaultPath)
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let path = arguments["path"] as? String,
              let content = arguments["content"] as? String else {
            return "Error: Missing 'path' or 'content' parameter."
        }

        let resolvedPath = resolvePath(path)
        let dir = (resolvedPath as NSString).deletingLastPathComponent
        try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)

        try content.write(toFile: resolvedPath, atomically: true, encoding: .utf8)
        return "Successfully wrote \(content.count) chars to \(path)"
    }
}

struct ListDirectoryTool: AgentTool {
    let name = "list_directory"
    let description = "List the contents of a directory at the given path."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "path": [
                    "type": "string",
                    "description": "Absolute path or path relative to the vault root. Defaults to vault root."
                ]
            ],
            "required": []
        ]
    }

    private let vaultPath: String

    init(vaultPath: String) {
        self.vaultPath = vaultPath
    }

    func execute(arguments: [String: Any]) throws -> String {
        let path = arguments["path"] as? String ?? ""
        let resolvedPath = path.isEmpty ? vaultPath : resolveVaultPath(path, vaultPath: vaultPath)

        guard FileManager.default.fileExists(atPath: resolvedPath) else {
            return "Error: Directory not found at \(resolvedPath)"
        }

        let contents = try FileManager.default.contentsOfDirectory(atPath: resolvedPath)
        if contents.isEmpty {
            return "Directory is empty."
        }

        return contents.sorted().joined(separator: "\n")
    }
}

struct GetConfigTool: AgentTool {
    let name = "get_config"
    let description = "Get the current Trace configuration including vault path, daily folder, sections, and threads."
    var parameterSchema: [String: Any] {
        ["type": "object", "properties": [:]]
    }

    private let settings: AppSettings

    init(settings: AppSettings) {
        self.settings = settings
    }

    func execute(arguments: [String: Any]) throws -> String {
        let sections = settings.sections.map { "- \($0.title) (index: \($0.index))" }.joined(separator: "\n")
        let threads = settings.threadConfigs.map { "- \($0.name) → \($0.targetFile)" }.joined(separator: "\n")

        return """
        Configuration:
        - Vault path: \(settings.vaultPath)
        - Daily folder: \(settings.dailyFolderName)
        - Image assets folder: \(settings.imageAssetsFolderName)
        - Write mode: \(settings.noteWriteMode.rawValue)
        - File date format: \(settings.dailyFileDateFormat)

        Sections:
        \(sections.isEmpty ? "(none)" : sections)

        Threads:
        \(threads.isEmpty ? "(none)" : threads)
        """
    }
}

struct UpdateConfigTool: AgentTool {
    let name = "update_config"
    let description = "Update Trace configuration. All fields are optional — only provided fields will be updated."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "vaultPath": ["type": "string", "description": "New vault path."],
                "dailyFolderName": ["type": "string", "description": "New daily folder name."],
                "imageAssetsFolderName": ["type": "string", "description": "New image assets folder name."],
                "noteWriteMode": ["type": "string", "description": "Write mode: 'dimension' or 'thread'.", "enum": ["dimension", "thread"]]
            ],
            "required": []
        ]
    }

    private let settings: AppSettings

    init(settings: AppSettings) {
        self.settings = settings
    }

    func execute(arguments: [String: Any]) throws -> String {
        var updated: [String] = []

        if let vaultPath = arguments["vaultPath"] as? String, !vaultPath.isEmpty {
            settings.vaultPath = vaultPath
            updated.append("vaultPath")
        }
        if let dailyFolder = arguments["dailyFolderName"] as? String, !dailyFolder.isEmpty {
            settings.dailyFolderName = dailyFolder
            updated.append("dailyFolderName")
        }
        if let assetsFolder = arguments["imageAssetsFolderName"] as? String, !assetsFolder.isEmpty {
            settings.imageAssetsFolderName = assetsFolder
            updated.append("imageAssetsFolderName")
        }
        if let mode = arguments["noteWriteMode"] as? String,
           let writeMode = NoteWriteMode(rawValue: mode) {
            settings.noteWriteMode = writeMode
            updated.append("noteWriteMode")
        }

        return updated.isEmpty ? "No changes made." : "Updated: \(updated.joined(separator: ", "))"
    }
}

struct CreateDirectoryTool: AgentTool {
    let name = "create_directory"
    let description = "Create a directory at the given path, including parent directories."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "path": [
                    "type": "string",
                    "description": "Absolute path or path relative to the vault root."
                ]
            ],
            "required": ["path"]
        ]
    }

    private let vaultPath: String

    init(vaultPath: String) {
        self.vaultPath = vaultPath
    }

    private func resolvePath(_ path: String) -> String {
        resolveVaultPath(path, vaultPath: vaultPath)
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let path = arguments["path"] as? String else {
            return "Error: Missing 'path' parameter."
        }

        let resolvedPath = resolvePath(path)
        try FileManager.default.createDirectory(atPath: resolvedPath, withIntermediateDirectories: true)
        return "Created directory: \(path)"
    }
}

// MARK: - Memory Tools

struct UpdateIdentityTool: AgentTool {
    let name = "update_identity"
    let description = "Update the user identity profile. This persists across sessions and makes you remember the user. Provide the full updated content."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "content": [
                    "type": "string",
                    "description": "Full identity profile content in Markdown. Include: user's name/role, interests, work domain, preferences, recurring topics, communication style."
                ]
            ],
            "required": ["content"]
        ]
    }

    private let memory: AgentMemory

    init(memory: AgentMemory) {
        self.memory = memory
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let content = arguments["content"] as? String else {
            return "Error: Missing 'content' parameter."
        }
        try memory.updateIdentity(content)
        return "Identity profile updated successfully."
    }
}

struct SaveSessionSummaryTool: AgentTool {
    let name = "save_session_summary"
    let description = "Save a brief summary of the current conversation session. This is called at the end of each session for cross-session memory."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "summary": [
                    "type": "string",
                    "description": "Brief summary of what was discussed, key decisions, and any action items."
                ]
            ],
            "required": ["summary"]
        ]
    }

    private let memory: AgentMemory

    init(memory: AgentMemory) {
        self.memory = memory
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let summary = arguments["summary"] as? String else {
            return "Error: Missing 'summary' parameter."
        }
        try memory.saveSessionSummary(date: Date(), summary: summary)
        return "Session summary saved."
    }
}

struct SaveInsightTool: AgentTool {
    let name = "save_insight"
    let description = "Save an insight extracted from the user's captures or conversation. Insights are stored separately for future reference."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "title": [
                    "type": "string",
                    "description": "Short title for the insight (e.g. 'Product pricing patterns')"
                ],
                "content": [
                    "type": "string",
                    "description": "The insight content in Markdown."
                ]
            ],
            "required": ["title", "content"]
        ]
    }

    private let memory: AgentMemory

    init(memory: AgentMemory) {
        self.memory = memory
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let title = arguments["title"] as? String,
              let content = arguments["content"] as? String else {
            return "Error: Missing 'title' or 'content' parameter."
        }
        try memory.saveInsight(title: title, content: content)
        return "Insight saved: \(title)"
    }
}

struct UpdateTodosTool: AgentTool {
    let name = "update_todos"
    let description = "Update the current task list / plan. Use this to track multi-step work and show progress to the user."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "content": [
                    "type": "string",
                    "description": "Markdown todo list. Use '- [ ]' for pending, '- [x]' for completed."
                ]
            ],
            "required": ["content"]
        ]
    }

    private let memory: AgentMemory

    init(memory: AgentMemory) {
        self.memory = memory
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let content = arguments["content"] as? String else {
            return "Error: Missing 'content' parameter."
        }
        try memory.updateTodos(content)
        return "Todos updated."
    }
}

// MARK: - Search Tool

struct SearchVaultTool: AgentTool {
    let name = "search_vault"
    let description = "Search for a keyword across all Markdown files in the vault. Returns matching lines with file paths."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "query": [
                    "type": "string",
                    "description": "Search keyword or phrase."
                ]
            ],
            "required": ["query"]
        ]
    }

    private let vaultPath: String

    init(vaultPath: String) {
        self.vaultPath = vaultPath
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let query = arguments["query"] as? String, !query.isEmpty else {
            return "Error: Missing or empty 'query' parameter."
        }

        let lowerQuery = query.lowercased()
        var results: [String] = []
        let maxResults = 20

        searchDirectory(at: vaultPath, query: lowerQuery, results: &results, max: maxResults)

        if results.isEmpty {
            return "No matches found for '\(query)'."
        }
        return "Found \(results.count) match(es):\n" + results.joined(separator: "\n")
    }

    private func searchDirectory(at path: String, query: String, results: inout [String], max: Int) {
        guard results.count < max else { return }
        guard let contents = try? FileManager.default.contentsOfDirectory(atPath: path) else { return }

        for item in contents.sorted() {
            guard results.count < max else { return }
            if item.hasPrefix(".") { continue }

            let itemPath = "\(path)/\(item)"
            var isDir: ObjCBool = false
            FileManager.default.fileExists(atPath: itemPath, isDirectory: &isDir)

            if isDir.boolValue {
                searchDirectory(at: itemPath, query: query, results: &results, max: max)
            } else if item.hasSuffix(".md") {
                if let content = try? String(contentsOfFile: itemPath, encoding: .utf8) {
                    let lines = content.components(separatedBy: "\n")
                    for (lineNum, line) in lines.enumerated() {
                        if line.lowercased().contains(query) {
                            let relativePath = itemPath.replacingOccurrences(of: vaultPath + "/", with: "")
                            results.append("\(relativePath):\(lineNum + 1): \(line.trimmingCharacters(in: .whitespaces))")
                            if results.count >= max { return }
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Route Capture Tool

struct RouteCaptureTool: AgentTool {
    let name = "route_capture"
    let description = "Determine where to save a user's capture. Returns the recommended file path within the vault."
    var parameterSchema: [String: Any] {
        [
            "type": "object",
            "properties": [
                "content": [
                    "type": "string",
                    "description": "The user's input content to route."
                ]
            ],
            "required": ["content"]
        ]
    }

    private let settings: AppSettings

    init(settings: AppSettings) {
        self.settings = settings
    }

    func execute(arguments: [String: Any]) throws -> String {
        guard let content = arguments["content"] as? String else {
            return "Error: Missing 'content' parameter."
        }

        switch settings.noteWriteMode {
        case .dimension:
            let sections = settings.sections
            if sections.isEmpty {
                return "No sections configured. Using daily note."
            }
            let sectionList = sections.enumerated().map { "\($0.offset): \($0.element.title)" }.joined(separator: ", ")
            return """
            Current sections: \(sectionList)
            Write mode: dimension (daily notes with sections)
            The capture will be saved to: \(settings.dailyFolderName)/{today's date}.md
            Analyze the content and suggest which section it belongs to.
            """
        case .thread:
            let threads = settings.threadConfigs
            if threads.isEmpty {
                return "No threads configured."
            }
            let threadList = threads.map { "- \($0.name) → \($0.targetFile)" }.joined(separator: "\n")
            return """
            Available threads:
            \(threadList)
            Analyze the content and suggest which thread it belongs to.
            """
        }
    }
}

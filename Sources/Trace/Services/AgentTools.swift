import Foundation

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

    func execute(arguments: [String: Any]) throws -> String {
        guard let path = arguments["path"] as? String else {
            return "Error: Missing 'path' parameter."
        }

        let resolvedPath: String
        if path.hasPrefix("/") {
            resolvedPath = path
        } else {
            resolvedPath = (vaultPath as NSString).appendingPathComponent(path)
        }

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

    func execute(arguments: [String: Any]) throws -> String {
        guard let path = arguments["path"] as? String,
              let content = arguments["content"] as? String else {
            return "Error: Missing 'path' or 'content' parameter."
        }

        let resolvedPath: String
        if path.hasPrefix("/") {
            resolvedPath = path
        } else {
            resolvedPath = (vaultPath as NSString).appendingPathComponent(path)
        }

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
        let resolvedPath: String
        if path.isEmpty || !path.hasPrefix("/") {
            resolvedPath = (vaultPath as NSString).appendingPathComponent(path)
        } else {
            resolvedPath = path
        }

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

    func execute(arguments: [String: Any]) throws -> String {
        guard let path = arguments["path"] as? String else {
            return "Error: Missing 'path' parameter."
        }

        let resolvedPath: String
        if path.hasPrefix("/") {
            resolvedPath = path
        } else {
            resolvedPath = (vaultPath as NSString).appendingPathComponent(path)
        }

        try FileManager.default.createDirectory(atPath: resolvedPath, withIntermediateDirectories: true)
        return "Created directory: \(path)"
    }
}

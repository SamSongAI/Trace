import Foundation

final class AgentMemory {
    private let vaultPath: String
    private let fileManager: FileManager

    private var agentDir: String { "\(vaultPath)/agent" }
    private var sessionsDir: String { "\(agentDir)/sessions" }
    private var insightsDir: String { "\(agentDir)/insights" }
    private var identityPath: String { "\(agentDir)/identity.md" }
    private var todosPath: String { "\(agentDir)/todos.md" }

    private let maxSessionSummaries = 5
    private let maxVaultStructureDepth = 2

    init(vaultPath: String, fileManager: FileManager = .default) {
        self.vaultPath = vaultPath
        self.fileManager = fileManager
    }

    // MARK: - Setup

    func ensureDirectories() {
        for dir in [agentDir, sessionsDir, insightsDir] {
            if !fileManager.fileExists(atPath: dir) {
                try? fileManager.createDirectory(atPath: dir, withIntermediateDirectories: true)
            }
        }
    }

    // MARK: - Context Injection

    func buildContextBlock() -> String {
        ensureDirectories()
        var parts: [String] = []

        if let identity = readIdentity() {
            parts.append("=== USER IDENTITY ===\n\(identity)")
        }

        if let summaries = readRecentSessionSummaries() {
            parts.append("=== RECENT SESSIONS ===\n\(summaries)")
        }

        if let structure = readVaultStructure() {
            parts.append("=== VAULT STRUCTURE ===\n\(structure)")
        }

        if let todos = readTodos() {
            parts.append("=== CURRENT TODOS ===\n\(todos)")
        }

        return parts.isEmpty ? "" : "\n\n--- AGENT MEMORY ---\n" + parts.joined(separator: "\n\n") + "\n--- END MEMORY ---"
    }

    // MARK: - Identity

    func readIdentity() -> String? {
        guard fileManager.fileExists(atPath: identityPath) else { return nil }
        return try? String(contentsOfFile: identityPath, encoding: .utf8)
    }

    func updateIdentity(_ content: String) throws {
        ensureDirectories()
        try content.write(toFile: identityPath, atomically: true, encoding: .utf8)
    }

    // MARK: - Session Summaries

    func readRecentSessionSummaries() -> String? {
        guard let files = try? fileManager.contentsOfDirectory(atPath: sessionsDir) else { return nil }
        let sorted = files.filter { $0.hasSuffix(".md") }.sorted().suffix(maxSessionSummaries)
        guard !sorted.isEmpty else { return nil }

        var summaries: [String] = []
        for file in sorted {
            let path = "\(sessionsDir)/\(file)"
            if let content = try? String(contentsOfFile: path, encoding: .utf8) {
                let title = file.replacingOccurrences(of: ".md", with: "")
                summaries.append("[\(title)]\n\(content)")
            }
        }
        return summaries.isEmpty ? nil : summaries.joined(separator: "\n\n")
    }

    func saveSessionSummary(date: Date, summary: String) throws {
        ensureDirectories()
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        let filename = "\(formatter.string(from: date)).md"
        let path = "\(sessionsDir)/\(filename)"
        try summary.write(toFile: path, atomically: true, encoding: .utf8)
    }

    // MARK: - Insights

    func saveInsight(title: String, content: String) throws {
        ensureDirectories()
        let safeTitle = title.replacingOccurrences(of: "/", with: "-")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !safeTitle.isEmpty else { return }
        let path = "\(insightsDir)/\(safeTitle).md"
        try content.write(toFile: path, atomically: true, encoding: .utf8)
    }

    // MARK: - Todos (Planning)

    func readTodos() -> String? {
        guard fileManager.fileExists(atPath: todosPath) else { return nil }
        return try? String(contentsOfFile: todosPath, encoding: .utf8)
    }

    func updateTodos(_ content: String) throws {
        ensureDirectories()
        try content.write(toFile: todosPath, atomically: true, encoding: .utf8)
    }

    // MARK: - Vault Structure

    func readVaultStructure() -> String? {
        guard !vaultPath.isEmpty, fileManager.fileExists(atPath: vaultPath) else { return nil }

        var lines: [String] = []
        collectDirectoryStructure(at: vaultPath, depth: 0, maxDepth: maxVaultStructureDepth, into: &lines)
        return lines.isEmpty ? nil : lines.joined(separator: "\n")
    }

    private func collectDirectoryStructure(at path: String, depth: Int, maxDepth: Int, into lines: inout [String]) {
        guard depth <= maxDepth else { return }
        let indent = String(repeating: "  ", count: depth)
        let name = (path as NSString).lastPathComponent

        guard let contents = try? fileManager.contentsOfDirectory(atPath: path) else { return }

        let visible = contents.filter { !$0.hasPrefix(".") }.sorted()
        for item in visible {
            let itemPath = "\(path)/\(item)"
            var isDir: ObjCBool = false
            fileManager.fileExists(atPath: itemPath, isDirectory: &isDir)

            if isDir.boolValue {
                lines.append("\(indent)[DIR] \(item)/")
                collectDirectoryStructure(at: itemPath, depth: depth + 1, maxDepth: maxDepth, into: &lines)
            } else {
                lines.append("\(indent)\(item)")
            }
        }
    }

    // MARK: - Session End

    func sessionEndPrompt() -> String {
        return """
        Wrap up this session. Do ALL of the following in a SINGLE response, then stop:
        1. If you learned new things about the user, call update_identity once.
        2. Call save_session_summary once with a brief summary.
        3. Do NOT call any other tools. Do NOT repeat tool calls. Output a brief farewell message and stop.
        """
    }
}

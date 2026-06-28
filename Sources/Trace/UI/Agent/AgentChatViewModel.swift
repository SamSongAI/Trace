import Foundation
import SwiftUI

struct AgentEvent: Identifiable {
    let id = UUID()
    let kind: Kind
    let toolName: String?
    let toolArgs: String?
    let toolResult: String?
    let iteration: Int?
    
    enum Kind {
        case iteration
        case toolStart
        case toolResult
    }
}

struct AgentSession: Identifiable, Codable {
    let id: UUID
    var title: String
    var messages: [AgentMessage]
    var createdAt: Date
    var updatedAt: Date
    
    init(id: UUID = UUID(), title: String = "New Chat", messages: [AgentMessage] = [], createdAt: Date = Date(), updatedAt: Date = Date()) {
        self.id = id
        self.title = title
        self.messages = messages
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
    
    var preview: String {
        let lastUser = messages.last(where: { $0.role == "user" })
        if let content = lastUser?.content {
            return String(content.prefix(40))
        }
        return title
    }
}

@MainActor
final class AgentChatViewModel: ObservableObject {
    @Published var messages: [AgentMessage] = []
    @Published var inputText: String = ""
    @Published var isLoading: Bool = false
    @Published var errorMessage: String?
    @Published var streamingText: String = ""
    @Published var events: [AgentEvent] = []
    @Published var isSavingMemory: Bool = false
    @Published var sessions: [AgentSession] = []
    @Published var currentSessionId: UUID?

    private let agent: AgentCore
    private let memory: AgentMemory
    private var tokenBuffer = ""
    private var flushTimer: Timer?
    private let sessionsURL: URL?
    private var sendTask: Task<Void, Never>?

    init(settings: AppSettings) {
        self.memory = AgentMemory(vaultPath: settings.vaultPath)
        self.agent = AgentCore(settings: settings)

        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        self.sessionsURL = appSupport?.appendingPathComponent("Trace/agent-sessions.json")

        // File system tools
        agent.registerTool(ReadFileTool(vaultPath: settings.vaultPath))
        agent.registerTool(WriteFileTool(vaultPath: settings.vaultPath))
        agent.registerTool(ListDirectoryTool(vaultPath: settings.vaultPath))
        agent.registerTool(CreateDirectoryTool(vaultPath: settings.vaultPath))

        // Config tools
        agent.registerTool(GetConfigTool(settings: settings))
        agent.registerTool(UpdateConfigTool(settings: settings))

        // Memory tools
        agent.registerTool(UpdateIdentityTool(memory: memory))
        agent.registerTool(SaveSessionSummaryTool(memory: memory))
        agent.registerTool(SaveInsightTool(memory: memory))
        agent.registerTool(UpdateTodosTool(memory: memory))

        // Search & routing
        agent.registerTool(SearchVaultTool(vaultPath: settings.vaultPath))
        agent.registerTool(RouteCaptureTool(settings: settings))

        loadSessions()
        if sessions.isEmpty {
            newSession()
        } else {
            currentSessionId = sessions.first?.id
            messages = sessions.first?.messages ?? []
        }
    }

    var isConfigured: Bool {
        agent.isConfigured
    }

    func send() async {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, !isLoading else { return }

        inputText = ""
        errorMessage = nil
        streamingText = ""
        events = []
        tokenBuffer = ""

        let userMessage = AgentMessage(role: "user", content: text)
        messages.append(userMessage)

        isLoading = true
        startFlushTimer()

        sendTask = Task {
            do {
                let response = try await agent.chat(
                    messages: messages,
                    onToken: { [weak self] token in
                        self?.appendToken(token)
                    },
                    onToolCall: { [weak self] name, args in
                        DispatchQueue.main.async {
                            self?.events.append(AgentEvent(kind: .toolStart, toolName: name, toolArgs: args, toolResult: nil, iteration: nil))
                        }
                    },
                    onToolResult: { [weak self] name, result in
                        DispatchQueue.main.async {
                            self?.events.append(AgentEvent(kind: .toolResult, toolName: name, toolArgs: nil, toolResult: result, iteration: nil))
                        }
                    },
                    onIteration: { [weak self] iter in
                        DispatchQueue.main.async {
                            self?.events.append(AgentEvent(kind: .iteration, toolName: nil, toolArgs: nil, toolResult: nil, iteration: iter))
                        }
                    }
                )
                if Task.isCancelled { return }
                flushTokens()
                stopFlushTimer()
                streamingText = ""
                messages.append(response)
                updateCurrentSession()
            } catch {
                if Task.isCancelled { return }
                flushTokens()
                stopFlushTimer()
                streamingText = ""
                errorMessage = error.localizedDescription
            }

            isLoading = false
        }
    }

    func stop() {
        sendTask?.cancel()
        sendTask = nil
        flushTokens()
        stopFlushTimer()
        streamingText = ""
        isLoading = false
    }

    private func appendToken(_ token: String) {
        tokenBuffer += token
    }

    private func startFlushTimer() {
        flushTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            DispatchQueue.main.async {
                self?.flushTokens()
            }
        }
    }

    private func stopFlushTimer() {
        flushTimer?.invalidate()
        flushTimer = nil
    }

    private func flushTokens() {
        guard !tokenBuffer.isEmpty else { return }
        streamingText += tokenBuffer
        tokenBuffer = ""
    }

    func endSession() async {
        guard !messages.isEmpty else { return }
        isSavingMemory = true
        await agent.endSession(messages: messages)
        isSavingMemory = false
    }

    func clearHistory() {
        messages.removeAll()
        errorMessage = nil
        events = []
        updateCurrentSession()
    }

    // MARK: - Session Management

    func newSession() {
        let session = AgentSession()
        sessions.insert(session, at: 0)
        currentSessionId = session.id
        messages = []
        errorMessage = nil
        events = []
        saveSessions()
    }

    func switchToSession(_ id: UUID) {
        guard let session = sessions.first(where: { $0.id == id }) else { return }
        currentSessionId = id
        messages = session.messages
        errorMessage = nil
        events = []
    }

    func deleteSession(_ id: UUID) {
        sessions.removeAll { $0.id == id }
        if currentSessionId == id {
            if let first = sessions.first {
                currentSessionId = first.id
                messages = first.messages
            } else {
                newSession()
            }
        }
        saveSessions()
    }

    private func updateCurrentSession() {
        guard let id = currentSessionId,
              let idx = sessions.firstIndex(where: { $0.id == id }) else { return }
        sessions[idx].messages = messages
        sessions[idx].updatedAt = Date()
        if sessions[idx].title == "New Chat", let firstUser = messages.first(where: { $0.role == "user" }), let content = firstUser.content {
            sessions[idx].title = String(content.prefix(30))
        }
        saveSessions()
    }

    // MARK: - Persistence

    private func loadSessions() {
        guard let url = sessionsURL,
              let data = try? Data(contentsOf: url),
              let decoded = try? JSONDecoder().decode([AgentSession].self, from: data) else { return }
        sessions = decoded
    }

    private func saveSessions() {
        guard let url = sessionsURL else { return }
        let dir = url.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(sessions) {
            try? data.write(to: url, options: .atomic)
        }
    }

    func copyLastResponse() {
        guard let lastAssistant = messages.last(where: { $0.role == "assistant" }),
              let content = lastAssistant.content else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(content, forType: .string)
    }
}

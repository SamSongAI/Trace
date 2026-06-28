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

@MainActor
final class AgentChatViewModel: ObservableObject {
    @Published var messages: [AgentMessage] = []
    @Published var inputText: String = ""
    @Published var isLoading: Bool = false
    @Published var errorMessage: String?
    @Published var streamingText: String = ""
    @Published var events: [AgentEvent] = []
    @Published var isSavingMemory: Bool = false

    private let agent: AgentCore
    private let memory: AgentMemory
    private var tokenBuffer = ""
    private var flushTimer: Timer?

    init(settings: AppSettings) {
        self.memory = AgentMemory(vaultPath: settings.vaultPath)
        self.agent = AgentCore(settings: settings)

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

        do {
            let response = try await agent.chat(
                messages: messages,
                onToken: { [weak self] token in
                    self?.appendToken(token)
                },
                onToolCall: { [weak self] name in
                    DispatchQueue.main.async {
                        self?.events.append(AgentEvent(kind: .toolStart, toolName: name, toolArgs: nil, toolResult: nil, iteration: nil))
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
            flushTokens()
            stopFlushTimer()
            streamingText = ""
            messages.append(response)
        } catch {
            flushTokens()
            stopFlushTimer()
            streamingText = ""
            errorMessage = error.localizedDescription
        }

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
    }
}

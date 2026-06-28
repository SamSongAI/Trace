import Foundation
import SwiftUI

@MainActor
final class AgentChatViewModel: ObservableObject {
    @Published var messages: [AgentMessage] = []
    @Published var inputText: String = ""
    @Published var isLoading: Bool = false
    @Published var errorMessage: String?
    @Published var streamingText: String = ""
    @Published var activeToolCall: String?
    @Published var isSavingMemory: Bool = false

    private let agent: AgentCore
    private let memory: AgentMemory

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

        let userMessage = AgentMessage(role: "user", content: text)
        messages.append(userMessage)

        isLoading = true

        do {
            let response = try await agent.chat(
                messages: messages,
                onToken: { token in
                    Task { @MainActor in
                        self.streamingText += token
                    }
                },
                onToolCall: { name in
                    Task { @MainActor in
                        self.activeToolCall = name
                    }
                }
            )
            self.activeToolCall = nil
            self.streamingText = ""
            messages.append(response)
        } catch {
            self.activeToolCall = nil
            self.streamingText = ""
            errorMessage = error.localizedDescription
        }

        isLoading = false
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
    }
}

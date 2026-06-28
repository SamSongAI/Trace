import Foundation
import SwiftUI

@MainActor
final class AgentChatViewModel: ObservableObject {
    @Published var messages: [AgentMessage] = []
    @Published var inputText: String = ""
    @Published var isLoading: Bool = false
    @Published var errorMessage: String?

    private let agent: AgentCore

    init(settings: AppSettings) {
        self.agent = AgentCore(settings: settings)
        agent.registerTool(ReadFileTool(vaultPath: settings.vaultPath))
        agent.registerTool(WriteFileTool(vaultPath: settings.vaultPath))
        agent.registerTool(ListDirectoryTool(vaultPath: settings.vaultPath))
        agent.registerTool(CreateDirectoryTool(vaultPath: settings.vaultPath))
        agent.registerTool(GetConfigTool(settings: settings))
        agent.registerTool(UpdateConfigTool(settings: settings))
    }

    var isConfigured: Bool {
        agent.isConfigured
    }

    func send() async {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, !isLoading else { return }

        inputText = ""
        errorMessage = nil

        let userMessage = AgentMessage(role: "user", content: text)
        messages.append(userMessage)

        isLoading = true

        do {
            let response = try await agent.chat(messages: messages)
            messages.append(response)
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    func clearHistory() {
        messages.removeAll()
        errorMessage = nil
    }
}

import Foundation

struct AgentMessage: Identifiable, Codable, Equatable {
    let id: UUID
    var role: String
    var content: String
    var toolCalls: [AgentToolCall]?
    var toolCallId: String?
    var name: String?

    init(id: UUID = UUID(), role: String, content: String, toolCalls: [AgentToolCall]? = nil, toolCallId: String? = nil, name: String? = nil) {
        self.id = id
        self.role = role
        self.content = content
        self.toolCalls = toolCalls
        self.toolCallId = toolCallId
        self.name = name
    }

    func toAPIDict() -> [String: Any] {
        var dict: [String: Any] = ["role": role, "content": content]
        if let toolCalls, !toolCalls.isEmpty {
            dict["tool_calls"] = toolCalls.map { $0.toAPIDict() }
        }
        if let toolCallId {
            dict["tool_call_id"] = toolCallId
        }
        if let name {
            dict["name"] = name
        }
        return dict
    }
}

struct AgentToolCall: Codable, Equatable {
    let id: String
    let type: String
    let function: AgentToolFunction
}

struct AgentToolFunction: Codable, Equatable {
    let name: String
    let arguments: String
}

extension AgentToolCall {
    func toAPIDict() -> [String: Any] {
        return [
            "id": id,
            "type": type,
            "function": [
                "name": function.name,
                "arguments": function.arguments
            ]
        ]
    }
}

struct AgentToolDefinition {
    let type: String
    let function: AgentFunctionDefinition
}

struct AgentFunctionDefinition {
    let name: String
    let description: String
    let parameters: [String: Any]

    func toDict() -> [String: Any] {
        return [
            "name": name,
            "description": description,
            "parameters": parameters
        ]
    }
}

protocol AgentTool {
    var name: String { get }
    var description: String { get }
    var parameterSchema: [String: Any] { get }
    func execute(arguments: [String: Any]) throws -> String
}

extension AgentTool {
    var definition: AgentToolDefinition {
        AgentToolDefinition(type: "function", function: AgentFunctionDefinition(name: name, description: description, parameters: parameterSchema))
    }
}

enum AgentError: LocalizedError {
    case notConfigured
    case invalidEndpoint
    case apiError(String)
    case noResponse
    case maxIterationsReached

    var errorDescription: String? {
        switch self {
        case .notConfigured: return "AI is not configured. Please set API key in Settings."
        case .invalidEndpoint: return "Invalid AI endpoint URL."
        case .apiError(let message): return "AI API error: \(message)"
        case .noResponse: return "No response from AI."
        case .maxIterationsReached: return "Agent reached maximum iterations."
        }
    }
}

final class AgentCore {
    private let settings: AppSettings
    private var tools: [AgentTool] = []
    private let maxIterations = 10

    init(settings: AppSettings) {
        self.settings = settings
    }

    func registerTool(_ tool: AgentTool) {
        tools.append(tool)
    }

    func clearTools() {
        tools.removeAll()
    }

    var isConfigured: Bool {
        settings.aiEnabled && !settings.aiApiKey.isEmpty && !settings.aiEndpoint.isEmpty
    }

    var systemPrompt: String {
        """
        You are Trace Agent, a personal AI assistant embedded in the Trace app — a private, local-first \
        context capture tool for macOS. The user uses Trace to capture thoughts, ideas, and context \
        into their local Markdown vault.

        Your capabilities:
        - Read and write files in the user's vault
        - List directory contents
        - Help configure the vault structure (folders, sections, threads)
        - Summarize and synthesize the user's captures
        - Answer questions about their stored content

        The user's vault is at: \(settings.vaultPath)
        Daily folder: \(settings.dailyFolderName)
        Current write mode: \(settings.noteWriteMode.rawValue)

        Always respond concisely. When you need to take action, use the available tools. \
        When asking the user questions, keep them short and specific.
        """
    }

    func chat(messages: [AgentMessage], onToken: ((String) -> Void)? = nil) async throws -> AgentMessage {
        guard isConfigured else { throw AgentError.notConfigured }

        var conversationMessages = messages
        if !conversationMessages.contains(where: { $0.role == "system" }) {
            conversationMessages.insert(AgentMessage(role: "system", content: systemPrompt), at: 0)
        }

        for iteration in 0..<maxIterations {
            let response = try await callAPI(messages: conversationMessages)
            onToken?(response.content)

            if let toolCalls = response.toolCalls, !toolCalls.isEmpty {
                conversationMessages.append(response)

                for toolCall in toolCalls {
                    let result = try await executeToolCall(toolCall)
                    conversationMessages.append(AgentMessage(
                        role: "tool",
                        content: result,
                        toolCallId: toolCall.id,
                        name: toolCall.function.name
                    ))
                }
            } else {
                return response
            }
        }

        throw AgentError.maxIterationsReached
    }

    private func callAPI(messages: [AgentMessage]) async throws -> AgentMessage {
        let endpoint = settings.aiEndpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let url = URL(string: "\(endpoint)/chat/completions") else {
            throw AgentError.invalidEndpoint
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("Bearer \(settings.aiApiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        var body: [String: Any] = [
            "model": settings.aiModel,
            "messages": messages.map { $0.toAPIDict() }
        ]

        if !tools.isEmpty {
            body["tools"] = tools.map { ["type": "function", "function": $0.definition.function.toDict()] }
            body["tool_choice"] = "auto"
        }

        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (data, response) = try await URLSession.shared.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw AgentError.noResponse
        }

        guard httpResponse.statusCode == 200 else {
            let errorBody = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw AgentError.apiError("HTTP \(httpResponse.statusCode): \(errorBody)")
        }

        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let choices = json["choices"] as? [[String: Any]],
              let firstChoice = choices.first,
              let message = firstChoice["message"] as? [String: Any] else {
            throw AgentError.noResponse
        }

        let content = message["content"] as? String ?? ""
        var toolCalls: [AgentToolCall] = []

        if let rawToolCalls = message["tool_calls"] as? [[String: Any]] {
            for rawToolCall in rawToolCalls {
                guard let id = rawToolCall["id"] as? String,
                      let type = rawToolCall["type"] as? String,
                      let function = rawToolCall["function"] as? [String: Any],
                      let name = function["name"] as? String,
                      let arguments = function["arguments"] as? String else { continue }
                toolCalls.append(AgentToolCall(
                    id: id,
                    type: type,
                    function: AgentToolFunction(name: name, arguments: arguments)
                ))
            }
        }

        return AgentMessage(
            role: message["role"] as? String ?? "assistant",
            content: content,
            toolCalls: toolCalls.isEmpty ? nil : toolCalls
        )
    }

    private func executeToolCall(_ toolCall: AgentToolCall) async throws -> String {
        let toolName = toolCall.function.name
        guard let tool = tools.first(where: { $0.name == toolName }) else {
            return "Error: Unknown tool '\(toolName)'"
        }

        guard let argsData = toolCall.function.arguments.data(using: .utf8),
              let args = try JSONSerialization.jsonObject(with: argsData) as? [String: Any] else {
            return "Error: Invalid tool arguments"
        }

        do {
            return try tool.execute(arguments: args)
        } catch {
            return "Error: \(error.localizedDescription)"
        }
    }
}

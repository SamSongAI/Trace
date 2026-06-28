import Foundation
import os.log

private let agentLog = Logger(subsystem: "com.trace.app", category: "AgentCore")

struct AgentMessage: Identifiable, Codable, Equatable {
    let id: UUID
    var role: String
    var content: String?
    var toolCalls: [AgentToolCall]?
    var toolCallId: String?
    var name: String?

    init(id: UUID = UUID(), role: String, content: String? = nil, toolCalls: [AgentToolCall]? = nil, toolCallId: String? = nil, name: String? = nil) {
        self.id = id
        self.role = role
        self.content = content
        self.toolCalls = toolCalls
        self.toolCallId = toolCallId
        self.name = name
    }

    func toAPIDict() -> [String: Any] {
        var dict: [String: Any] = ["role": role]
        if let content, !content.isEmpty {
            dict["content"] = content
        } else {
            dict["content"] = NSNull()
        }
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
    var function: AgentToolFunction
}

struct AgentToolFunction: Codable, Equatable {
    var name: String
    var arguments: String
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
    private let memory: AgentMemory
    private var tools: [AgentTool] = []
    private let maxIterations = 10
    private let compactionThreshold = 16000
    private let maxTokens = 4096
    private let maxRetries = 3
    private let toolTimeout: TimeInterval = 30
    private var cachedSystemPrompt: String?

    init(settings: AppSettings) {
        self.settings = settings
        self.memory = AgentMemory(vaultPath: settings.vaultPath)
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
        if let cached = cachedSystemPrompt {
            return cached
        }
        let memoryBlock = memory.buildContextBlock()
        let prompt = """
        You are Trace Agent, a personal AI assistant embedded in the Trace app — a private, local-first \
        context capture tool for macOS. The user uses Trace to capture thoughts, ideas, and context \
        into their local Markdown vault.

        Your capabilities:
        - Read and write files in the user's vault
        - Route user captures to the right section/thread
        - Summarize and synthesize the user's captures
        - Maintain a memory layer that grows over time
        - Help configure the vault structure

        The user's vault is at: \(settings.vaultPath)
        Daily folder: \(settings.dailyFolderName)
        Current write mode: \(settings.noteWriteMode.rawValue)

        Architecture:
        - context/ layer: user's original captures. You READ this to understand context.
        - agent/ layer: your memory. You WRITE here to remember across sessions.

        Always respond concisely. When you need to take action, use the available tools. \
        When asking the user questions, keep them short and specific.
        \(memoryBlock)
        """
        cachedSystemPrompt = prompt
        return prompt
    }

    func chat(messages: [AgentMessage], onToken: ((String) -> Void)? = nil, onToolCall: ((String) -> Void)? = nil, onToolResult: ((String, String) -> Void)? = nil, onIteration: ((Int) -> Void)? = nil) async throws -> AgentMessage {
        guard isConfigured else { throw AgentError.notConfigured }

        var conversationMessages = messages
        if !conversationMessages.contains(where: { $0.role == "system" }) {
            conversationMessages.insert(AgentMessage(role: "system", content: systemPrompt), at: 0)
        }

        agentLog.info("Chat started with \(conversationMessages.count) messages")

        for iteration in 0..<maxIterations {
            agentLog.info("Iteration \(iteration)")
            onIteration?(iteration)

            // Compaction: if conversation is too long, summarize older messages
            conversationMessages = try await compactIfNeeded(conversationMessages)

            let response = try await callAPIStreaming(messages: conversationMessages, onToken: onToken)

            agentLog.info("Response: content=\(response.content?.count ?? 0) chars, toolCalls=\(response.toolCalls?.count ?? 0)")

            if let toolCalls = response.toolCalls, !toolCalls.isEmpty {
                conversationMessages.append(response)

                // Execute tool calls in parallel (industry standard: OpenAI/Claude SDK)
                let toolResults = await executeToolCallsParallel(toolCalls, onToolCall: onToolCall, onToolResult: onToolResult)

                for result in toolResults {
                    conversationMessages.append(AgentMessage(
                        role: "tool",
                        content: result.content,
                        toolCallId: result.toolCallId,
                        name: result.name
                    ))
                }
            } else {
                agentLog.info("Chat complete")
                return response
            }
        }

        agentLog.error("Max iterations reached")
        throw AgentError.maxIterationsReached
    }

    func endSession(messages: [AgentMessage]) async {
        guard isConfigured else { return }
        let prompt = AgentMessage(role: "user", content: memory.sessionEndPrompt())
        var sessionMessages = messages
        sessionMessages.append(prompt)
        _ = try? await chat(messages: sessionMessages)
    }

    private func compactIfNeeded(_ messages: [AgentMessage]) async throws -> [AgentMessage] {
        let totalLength = messages.reduce(0) { $0 + ($1.content?.count ?? 0) }
        guard totalLength > compactionThreshold, messages.count > 6 else { return messages }

        // Keep system prompt + last 4 messages, compact the rest
        let systemMessages = messages.filter { $0.role == "system" }
        let nonSystem = messages.filter { $0.role != "system" }
        let toCompact = Array(nonSystem.dropLast(4))
        let recent = Array(nonSystem.suffix(4))

        guard !toCompact.isEmpty else { return messages }

        // Build a structured summary preserving key info
        var summaryParts: [String] = []
        for msg in toCompact {
            let role = msg.role
            let content = msg.content ?? ""
            if role == "tool" {
                // Keep tool results but truncate more aggressively
                summaryParts.append("[tool:\(msg.name ?? "")] → \(content.prefix(200))")
            } else if let toolCalls = msg.toolCalls, !toolCalls.isEmpty {
                let callNames = toolCalls.map { $0.function.name }.joined(separator: ", ")
                summaryParts.append("[\(role) called: \(callNames)] \(content.prefix(100))")
            } else {
                summaryParts.append("[\(role)] \(content.prefix(300))")
            }
        }

        let summaryMessage = AgentMessage(
            role: "system",
            content: "[Compacted earlier conversation]\n" + summaryParts.joined(separator: "\n").prefix(2000)
        )

        return systemMessages + [summaryMessage] + recent
    }

    private func truncateResult(_ result: String, max: Int = 4000) -> String {
        if result.count <= max { return result }
        let head = result.prefix(max / 2)
        let tail = result.suffix(max / 2)
        return "\(head)\n\n... (truncated, \(result.count) chars total) ...\n\n\(tail)"
    }

    private func callAPIStreaming(messages: [AgentMessage], onToken: ((String) -> Void)?) async throws -> AgentMessage {
        let endpoint = settings.aiEndpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let url = URL(string: "\(endpoint)/chat/completions") else {
            throw AgentError.invalidEndpoint
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("Bearer \(settings.aiApiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.timeoutInterval = 120

        var body: [String: Any] = [
            "model": settings.aiModel,
            "messages": messages.map { $0.toAPIDict() },
            "stream": true,
            "max_tokens": maxTokens
        ]

        if !tools.isEmpty {
            body["tools"] = tools.map { ["type": "function", "function": $0.definition.function.toDict()] }
            body["tool_choice"] = "auto"
            body["parallel_tool_calls"] = true
        }

        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        agentLog.info("API request to \(url.absoluteString), model=\(self.settings.aiModel)")
        let requestStart = Date()

        // Retry with exponential backoff for 5xx errors
        var lastError: Error?
        for attempt in 0..<maxRetries {
            do {
                let (bytes, response) = try await URLSession.shared.bytes(for: request)

                guard let httpResponse = response as? HTTPURLResponse else {
                    throw AgentError.noResponse
                }

                if httpResponse.statusCode >= 500 && attempt < maxRetries - 1 {
                    let delay = pow(2.0, Double(attempt)) * 0.5
                    agentLog.warning("HTTP \(httpResponse.statusCode), retrying in \(delay)s (attempt \(attempt + 1)/\(self.maxRetries))")
                    try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
                    continue
                }

                guard httpResponse.statusCode == 200 else {
                    var errorBody = ""
                    for try await line in bytes.lines {
                        errorBody += line
                    }
                    agentLog.error("API error: HTTP \(httpResponse.statusCode), body=\(errorBody)")
                    throw AgentError.apiError("HTTP \(httpResponse.statusCode): \(errorBody)")
                }

                agentLog.info("Stream started, \(Date().timeIntervalSince(requestStart))s")

                var content = ""
                var toolCalls: [String: AgentToolCall] = [:]
                var tokenCount = 0

                for try await line in bytes.lines {
                    guard line.hasPrefix("data: ") else { continue }
                    let data = String(line.dropFirst(6))
                    if data == "[DONE]" { break }

                    guard let lineData = data.data(using: .utf8),
                          let json = try? JSONSerialization.jsonObject(with: lineData) as? [String: Any],
                          let choices = json["choices"] as? [[String: Any]],
                          let delta = choices.first?["delta"] as? [String: Any] else { continue }

                    if let text = delta["content"] as? String, !text.isEmpty {
                        content += text
                        tokenCount += 1
                        onToken?(text)
                    }

                    if let rawToolCalls = delta["tool_calls"] as? [[String: Any]] {
                        for rawToolCall in rawToolCalls {
                            let id = rawToolCall["id"] as? String ?? ""
                            let index = rawToolCall["index"] as? Int ?? 0
                            let key = id.isEmpty ? "idx_\(index)" : id

                            if let function = rawToolCall["function"] as? [String: Any] {
                                let name = function["name"] as? String ?? ""
                                let arguments = function["arguments"] as? String ?? ""

                                if var existing = toolCalls[key] {
                                    existing.function.arguments += arguments
                                    if !name.isEmpty { existing.function.name = name }
                                    toolCalls[key] = existing
                                } else if !name.isEmpty {
                                    toolCalls[key] = AgentToolCall(
                                        id: id,
                                        type: rawToolCall["type"] as? String ?? "function",
                                        function: AgentToolFunction(name: name, arguments: arguments)
                                    )
                                }
                            }
                        }
                    }
                }

                agentLog.info("Stream done: \(tokenCount) tokens, \(content.count) chars, \(Date().timeIntervalSince(requestStart))s total")

                let finalToolCalls = Array(toolCalls.values)
                return AgentMessage(
                    role: "assistant",
                    content: content.isEmpty ? nil : content,
                    toolCalls: finalToolCalls.isEmpty ? nil : finalToolCalls
                )
            } catch {
                lastError = error
                if attempt < maxRetries - 1 {
                    let delay = pow(2.0, Double(attempt)) * 0.5
                    agentLog.warning("Request failed: \(error.localizedDescription), retrying in \(delay)s")
                    try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
                }
            }
        }

        throw lastError ?? AgentError.noResponse
    }

    // MARK: - Parallel Tool Execution

    private struct ToolResult {
        let toolCallId: String
        let name: String
        let content: String
    }

    private func executeToolCallsParallel(_ toolCalls: [AgentToolCall], onToolCall: ((String) -> Void)?, onToolResult: ((String, String) -> Void)?) async -> [ToolResult] {
        var results: [ToolResult] = []

        await withTaskGroup(of: ToolResult?.self) { group in
            for toolCall in toolCalls {
                let toolName = toolCall.function.name
                let toolArgs = toolCall.function.arguments
                agentLog.info("Tool call: \(toolName)(\(toolArgs))")
                onToolCall?(toolName)

                group.addTask { [self] in
                    let result = await self.executeToolWithTimeout(toolCall)
                    let truncated = self.truncateResult(result)
                    agentLog.info("Tool result [\(toolName)]: \(result.count) chars")
                    onToolResult?(toolName, truncated)
                    return ToolResult(toolCallId: toolCall.id, name: toolName, content: truncated)
                }
            }

            for await result in group {
                if let result {
                    results.append(result)
                }
            }
        }

        // Sort results to match original tool call order
        results.sort { a, b in
            let aIdx = toolCalls.firstIndex(where: { $0.id == a.toolCallId }) ?? 0
            let bIdx = toolCalls.firstIndex(where: { $0.id == b.toolCallId }) ?? 0
            return aIdx < bIdx
        }

        return results
    }

    private func executeToolWithTimeout(_ toolCall: AgentToolCall) async -> String {
        let toolName = toolCall.function.name
        guard let tool = tools.first(where: { $0.name == toolName }) else {
            return "Error: Unknown tool '\(toolName)'"
        }

        guard let argsData = toolCall.function.arguments.data(using: .utf8),
              let args = try? JSONSerialization.jsonObject(with: argsData) as? [String: Any] else {
            return "Error: Invalid tool arguments for '\(toolName)'"
        }

        // Execute with timeout
        let result = await withTaskGroup(of: String?.self) { group in
            group.addTask {
                do {
                    return try tool.execute(arguments: args)
                } catch {
                    return "Error: \(error.localizedDescription)"
                }
            }

            group.addTask {
                try? await Task.sleep(nanoseconds: UInt64(self.toolTimeout * 1_000_000_000))
                return nil
            }

            let first = await group.next()
            group.cancelAll()
            return first ?? "Error: Tool '\(toolName)' timed out after \(Int(self.toolTimeout))s"
        }

        return result ?? "Error: Tool '\(toolName)' returned no result"
    }

    private func executeToolCall(_ toolCall: AgentToolCall) async throws -> String {
        return await executeToolWithTimeout(toolCall)
    }
}

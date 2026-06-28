import SwiftUI

struct AgentChatView: View {
    @ObservedObject var viewModel: AgentChatViewModel
    @ObservedObject var settings: AppSettings
    let theme: TraceTheme.CapturePalette
    @Binding var isPresented: Bool

    var body: some View {
        if !viewModel.isConfigured {
            notConfiguredView
        } else {
            chatView
        }
    }

    private var notConfiguredView: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 28))
                .foregroundStyle(theme.accent)

            Text(L10n.aiNotConfigured)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(theme.textSecondary)
                .multilineTextAlignment(.center)

            Button {
                isPresented = false
                NotificationCenter.default.post(name: .traceOpenSettings, object: nil)
            } label: {
                Text(L10n.openSettings)
                    .font(.system(size: 12, weight: .semibold))
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .background(theme.accent.opacity(0.15))
                    .foregroundStyle(theme.accent)
                    .clipShape(Capsule())
            }
            .buttonStyle(.plain)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(24)
    }

    private var chatView: some View {
        VStack(spacing: 0) {
            messageList
            inputBar
        }
    }

    private var messageList: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 14) {
                    ForEach(viewModel.messages) { msg in
                        AgentMessageBubble(message: msg, theme: theme)
                            .id(msg.id)
                    }

                    if viewModel.isLoading && !viewModel.streamingText.isEmpty {
                        AgentStreamingBubble(text: viewModel.streamingText, theme: theme)
                            .id("streaming")
                    }

                    if let toolName = viewModel.activeToolCall {
                        AgentToolCallIndicator(toolName: toolName, theme: theme)
                            .id("toolcall")
                    }

                    if viewModel.isLoading && viewModel.streamingText.isEmpty && viewModel.activeToolCall == nil {
                        AgentTypingIndicator(theme: theme)
                            .id("typing")
                    }

                    if viewModel.isSavingMemory {
                        HStack(spacing: 6) {
                            ProgressView()
                                .scaleEffect(0.6)
                                .frame(width: 12, height: 12)
                            Text("Saving memory...")
                                .font(.system(size: 11, weight: .medium))
                        }
                        .foregroundStyle(theme.textSecondary)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                        .background(theme.surface.opacity(0.4))
                        .clipShape(Capsule())
                        .padding(.horizontal, 52)
                        .id("saving")
                    }

                    if let error = viewModel.errorMessage {
                        Text(error)
                            .font(.system(size: 11, weight: .medium))
                            .foregroundStyle(theme.accentStrong)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                            .id("error")
                    }
                }
                .padding(.vertical, 12)
            }
            .onChange(of: viewModel.messages.count) { _ in
                scrollToBottom(proxy)
            }
            .onChange(of: viewModel.streamingText) { _ in
                scrollToBottom(proxy)
            }
        }
    }

    private func scrollToBottom(_ proxy: ScrollViewProxy) {
        let targetId: String = viewModel.streamingText.isEmpty
            ? (viewModel.messages.last?.id.uuidString ?? "typing")
            : "streaming"
        withAnimation(.easeOut(duration: 0.15)) {
            proxy.scrollTo(targetId, anchor: .bottom)
        }
    }

    private var inputBar: some View {
        HStack(alignment: .bottom, spacing: 8) {
            TextField(L10n.aiInputPlaceholder, text: $viewModel.inputText, axis: .vertical)
                .font(.system(size: 13))
                .textFieldStyle(.plain)
                .lineLimit(1...5)
                .foregroundStyle(theme.textPrimary)
                .onSubmit {
                    Task { await viewModel.send() }
                }

            Button {
                Task { await viewModel.send() }
            } label: {
                Image(systemName: viewModel.isLoading ? "stop.fill" : "arrow.up.circle.fill")
                    .font(.system(size: 22))
                    .foregroundStyle(canSend ? theme.accent : theme.textSecondary.opacity(0.4))
            }
            .buttonStyle(.plain)
            .disabled(!canSend)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(theme.surface.opacity(0.5))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(theme.border.opacity(0.5), lineWidth: 0.5)
        )
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .padding(.horizontal, 12)
        .padding(.bottom, 12)
        .background(theme.chromeBackground)
    }

    private var canSend: Bool {
        !viewModel.inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !viewModel.isLoading
    }
}

// MARK: - Message Bubble with Markdown

private struct AgentMessageBubble: View {
    let message: AgentMessage
    let theme: TraceTheme.CapturePalette

    var body: some View {
        if message.role == "user" {
            userBubble
        } else if message.role == "assistant" {
            assistantBubble
        } else {
            EmptyView()
        }
    }

    private var userBubble: some View {
        HStack {
            Spacer(minLength: 40)
            Text(message.content)
                .font(.system(size: 13))
                .foregroundStyle(theme.textPrimary)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(theme.accent.opacity(0.12))
                .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        }
        .padding(.horizontal, 16)
    }

    private var assistantBubble: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: "sparkles")
                .font(.system(size: 10))
                .foregroundStyle(theme.accent)
                .frame(width: 22, height: 22)
                .background(theme.accent.opacity(0.12))
                .clipShape(Circle())

            VStack(alignment: .leading, spacing: 6) {
                if !message.content.isEmpty {
                    AgentMarkdownText(text: message.content, theme: theme)
                }
                if let toolCalls = message.toolCalls, !toolCalls.isEmpty {
                    ForEach(toolCalls, id: \.id) { tc in
                        HStack(spacing: 4) {
                            Image(systemName: "wrench.and.screwdriver")
                                .font(.system(size: 9))
                            Text(tc.function.name)
                                .font(.system(size: 10, weight: .medium))
                        }
                        .foregroundStyle(theme.textSecondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(theme.surface.opacity(0.4))
                        .clipShape(Capsule())
                    }
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(theme.surface.opacity(0.5))
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))

            Spacer(minLength: 30)
        }
        .padding(.horizontal, 16)
    }
}

// MARK: - Streaming Bubble

private struct AgentStreamingBubble: View {
    let text: String
    let theme: TraceTheme.CapturePalette

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: "sparkles")
                .font(.system(size: 10))
                .foregroundStyle(theme.accent)
                .frame(width: 22, height: 22)
                .background(theme.accent.opacity(0.12))
                .clipShape(Circle())

            AgentMarkdownText(text: text, theme: theme)
                .padding(.horizontal, 12)
                .padding(.vertical, 10)
                .background(theme.surface.opacity(0.5))
                .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .stroke(theme.accent.opacity(0.2), lineWidth: 0.5)
                )

            Spacer(minLength: 30)
        }
        .padding(.horizontal, 16)
    }
}

// MARK: - Tool Call Indicator

private struct AgentToolCallIndicator: View {
    let toolName: String
    let theme: TraceTheme.CapturePalette

    var body: some View {
        HStack(spacing: 6) {
            ProgressView()
                .scaleEffect(0.6)
                .frame(width: 12, height: 12)

            Image(systemName: "wrench.and.screwdriver")
                .font(.system(size: 9))

            Text(toolName)
                .font(.system(size: 11, weight: .medium))
        }
        .foregroundStyle(theme.textSecondary)
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(theme.surface.opacity(0.4))
        .clipShape(Capsule())
        .padding(.horizontal, 52)
    }
}

// MARK: - Markdown Text Renderer

private struct AgentMarkdownText: View {
    let text: String
    let theme: TraceTheme.CapturePalette

    var body: some View {
        Text(attributedText)
            .font(.system(size: 13))
            .foregroundStyle(theme.textPrimary)
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var attributedText: AttributedString {
        var att = AttributedString(text)

        // Bold **text**
        if let regex = try? NSRegularExpression(pattern: #"\*\*(.+?)\*\*"#) {
            let nsString = text as NSString
            let matches = regex.matches(in: text, range: NSRange(location: 0, length: nsString.length))
            for m in matches.reversed() {
                let fullRange = nsString.substring(with: m.range)
                if let range = att.range(of: fullRange) {
                    let inner = nsString.substring(with: NSRange(location: m.range.location + 2, length: m.range.length - 4))
                    var bold = AttributedString(inner)
                    bold.font = .system(size: 13, weight: .semibold)
                    att.replaceSubrange(range, with: bold)
                }
            }
        }

        // Inline code `text`
        if let regex = try? NSRegularExpression(pattern: #"`([^`]+)`"#) {
            let current = String(att.characters)
            let nsCurrent = current as NSString
            let matches = regex.matches(in: current, range: NSRange(location: 0, length: nsCurrent.length))
            for m in matches.reversed() {
                let fullRange = nsCurrent.substring(with: m.range)
                if let range = att.range(of: fullRange) {
                    let inner = nsCurrent.substring(with: NSRange(location: m.range.location + 1, length: m.range.length - 2))
                    var code = AttributedString(inner)
                    code.font = .system(size: 12, design: .monospaced)
                    code.foregroundColor = theme.accent
                    att.replaceSubrange(range, with: code)
                }
            }
        }

        return att
    }
}

// MARK: - Typing Indicator

private struct AgentTypingIndicator: View {
    let theme: TraceTheme.CapturePalette
    @State private var animate = false

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "sparkles")
                .font(.system(size: 10))
                .foregroundStyle(theme.accent)
                .frame(width: 22, height: 22)
                .background(theme.accent.opacity(0.12))
                .clipShape(Circle())

            HStack(spacing: 5) {
                ForEach(0..<3, id: \.self) { i in
                    Circle()
                        .fill(theme.textSecondary.opacity(0.5))
                        .frame(width: 6, height: 6)
                        .scaleEffect(animate ? 1.0 : 0.5)
                        .animation(
                            .easeInOut(duration: 0.5)
                            .repeatForever()
                            .delay(Double(i) * 0.15),
                            value: animate
                        )
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(theme.surface.opacity(0.5))
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        }
        .padding(.horizontal, 16)
        .onAppear { animate = true }
    }
}

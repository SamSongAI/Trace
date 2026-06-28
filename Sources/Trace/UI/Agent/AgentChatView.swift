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

                    if viewModel.isLoading && !viewModel.events.isEmpty {
                        AgentEventFlowView(events: viewModel.events, theme: theme)
                            .id("events")
                    }

                    if viewModel.isLoading && viewModel.streamingText.isEmpty && viewModel.events.isEmpty {
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
                .padding(.vertical, 10)
            }
            .onChange(of: viewModel.messages.count) { _ in
                scrollToBottom(proxy)
            }
            .onChange(of: viewModel.streamingText) { _ in
                scrollToBottom(proxy)
            }
            .onChange(of: viewModel.events.count) { _ in
                scrollToBottom(proxy)
            }
        }
    }

    private func scrollToBottom(_ proxy: ScrollViewProxy) {
        let targetId: String
        if !viewModel.streamingText.isEmpty {
            targetId = "streaming"
        } else if !viewModel.events.isEmpty {
            targetId = "events"
        } else {
            targetId = viewModel.messages.last?.id.uuidString ?? "typing"
        }
        withAnimation(.easeOut(duration: 0.15)) {
            proxy.scrollTo(targetId, anchor: .bottom)
        }
    }

    @State private var inputFocused = false

    private var inputBar: some View {
        HStack(alignment: .center, spacing: 8) {
            ZStack(alignment: .leading) {
                if viewModel.inputText.isEmpty {
                    Text(L10n.aiInputPlaceholder)
                        .font(.system(size: 13))
                        .foregroundStyle(theme.textSecondary.opacity(0.5))
                        .allowsHitTesting(false)
                }

                TextField("", text: $viewModel.inputText, axis: .vertical)
                    .font(.system(size: 13))
                    .textFieldStyle(.plain)
                    .lineLimit(1...5)
                    .foregroundStyle(theme.textPrimary)
                    .tint(theme.accent)
                    .onSubmit {
                        Task { await viewModel.send() }
                    }
            }

            Button {
                Task { await viewModel.send() }
            } label: {
                Image(systemName: viewModel.isLoading ? "stop.fill" : "arrow.up.circle.fill")
                    .font(.system(size: 22))
                    .foregroundStyle(canSend ? theme.accent : theme.textSecondary.opacity(0.3))
            }
            .buttonStyle(.plain)
            .disabled(!canSend)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(theme.surface)
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(theme.border, lineWidth: 1)
        )
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .padding(.horizontal, 12)
        .padding(.top, 6)
        .padding(.bottom, 10)
        .background(theme.panelBackground)
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
            Text(message.content ?? "")
                .font(.system(size: 13))
                .foregroundStyle(theme.textPrimary)
                .padding(.horizontal, 10)
                .padding(.vertical, 7)
                .background(theme.accent.opacity(0.1))
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .padding(.horizontal, 12)
    }

    private var assistantBubble: some View {
        HStack(alignment: .top, spacing: 6) {
            Image(systemName: "sparkles")
                .font(.system(size: 9))
                .foregroundStyle(theme.accent)
                .frame(width: 18, height: 18)
                .background(theme.accent.opacity(0.1))
                .clipShape(Circle())

            VStack(alignment: .leading, spacing: 4) {
                if let content = message.content, !content.isEmpty {
                    AgentMarkdownText(text: content, theme: theme)
                }
                if let toolCalls = message.toolCalls, !toolCalls.isEmpty {
                    ForEach(toolCalls, id: \.id) { tc in
                        HStack(spacing: 3) {
                            Image(systemName: "wrench.and.screwdriver")
                                .font(.system(size: 8))
                            Text(tc.function.name)
                                .font(.system(size: 9, weight: .medium))
                        }
                        .foregroundStyle(theme.textSecondary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(theme.surface.opacity(0.3))
                        .clipShape(Capsule())
                    }
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(theme.surface)
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))

            Spacer(minLength: 30)
        }
        .padding(.horizontal, 12)
    }
}

// MARK: - Streaming Bubble

private struct AgentStreamingBubble: View {
    let text: String
    let theme: TraceTheme.CapturePalette

    var body: some View {
        HStack(alignment: .top, spacing: 6) {
            Image(systemName: "sparkles")
                .font(.system(size: 9))
                .foregroundStyle(theme.accent)
                .frame(width: 18, height: 18)
                .background(theme.accent.opacity(0.1))
                .clipShape(Circle())

            AgentMarkdownText(text: text, theme: theme)
                .padding(.horizontal, 10)
                .padding(.vertical, 7)
                .background(theme.surface)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .stroke(theme.accent.opacity(0.15), lineWidth: 0.5)
                )

            Spacer(minLength: 30)
        }
        .padding(.horizontal, 12)
    }
}

// MARK: - Event Flow View

private struct AgentEventFlowView: View {
    let events: [AgentEvent]
    let theme: TraceTheme.CapturePalette

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            ForEach(events) { event in
                switch event.kind {
                case .iteration:
                    HStack(spacing: 4) {
                        Image(systemName: "arrow.triangle.2.circlepath")
                            .font(.system(size: 8))
                        Text("Round \(event.iteration.map { $0 + 1 } ?? 1)")
                            .font(.system(size: 9, weight: .medium))
                    }
                    .foregroundStyle(theme.textSecondary.opacity(0.6))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)

                case .toolStart:
                    HStack(spacing: 4) {
                        ProgressView()
                            .scaleEffect(0.5)
                            .frame(width: 10, height: 10)
                        Image(systemName: "wrench.and.screwdriver")
                            .font(.system(size: 8))
                        Text(event.toolName ?? "tool")
                            .font(.system(size: 10, weight: .medium))
                    }
                    .foregroundStyle(theme.accent)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(theme.accent.opacity(0.08))
                    .clipShape(Capsule())

                case .toolResult:
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: 4) {
                            Image(systemName: "checkmark.circle.fill")
                                .font(.system(size: 8))
                            Text(event.toolName ?? "tool")
                                .font(.system(size: 10, weight: .medium))
                            Text("done")
                                .font(.system(size: 9))
                                .foregroundStyle(theme.textSecondary.opacity(0.5))
                        }
                        .foregroundStyle(theme.textSecondary)

                        if let result = event.toolResult, !result.isEmpty {
                            Text(result.prefix(120))
                                .font(.system(size: 9, design: .monospaced))
                                .foregroundStyle(theme.textSecondary.opacity(0.6))
                                .lineLimit(2)
                                .padding(.leading, 16)
                        }
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                }
            }
        }
        .padding(.horizontal, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
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
        HStack(spacing: 6) {
            Image(systemName: "sparkles")
                .font(.system(size: 9))
                .foregroundStyle(theme.accent)
                .frame(width: 18, height: 18)
                .background(theme.accent.opacity(0.1))
                .clipShape(Circle())

            HStack(spacing: 4) {
                ForEach(0..<3, id: \.self) { i in
                    Circle()
                        .fill(theme.textSecondary.opacity(0.5))
                        .frame(width: 5, height: 5)
                        .scaleEffect(animate ? 1.0 : 0.5)
                        .animation(
                            .easeInOut(duration: 0.45)
                            .repeatForever()
                            .delay(Double(i) * 0.12),
                            value: animate
                        )
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(theme.surface)
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .padding(.horizontal, 12)
        .onAppear { animate = true }
    }
}

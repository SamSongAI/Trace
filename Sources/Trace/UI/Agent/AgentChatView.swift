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
            Divider().overlay(theme.border)
            inputBar
        }
    }

    private var messageList: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 12) {
                    ForEach(viewModel.messages) { msg in
                        AgentMessageBubble(message: msg, theme: theme)
                            .id(msg.id)
                    }

                    if viewModel.isLoading {
                        AgentTypingIndicator(theme: theme)
                    }

                    if let error = viewModel.errorMessage {
                        Text(error)
                            .font(.system(size: 11, weight: .medium))
                            .foregroundStyle(theme.accentStrong)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                    }
                }
                .padding(.vertical, 12)
            }
            .onChange(of: viewModel.messages.count) { _ in
                if let last = viewModel.messages.last {
                    withAnimation(.easeOut(duration: 0.2)) {
                        proxy.scrollTo(last.id, anchor: .bottom)
                    }
                }
            }
        }
    }

    private var inputBar: some View {
        HStack(spacing: 8) {
            TextField(L10n.aiInputPlaceholder, text: $viewModel.inputText, axis: .vertical)
                .font(.system(size: 13))
                .textFieldStyle(.plain)
                .lineLimit(1...4)
                .onSubmit {
                    Task { await viewModel.send() }
                }

            Button {
                Task { await viewModel.send() }
            } label: {
                Image(systemName: viewModel.isLoading ? "stop.fill" : "arrow.up.circle.fill")
                    .font(.system(size: 20))
                    .foregroundStyle(viewModel.inputText.isEmpty ? theme.textSecondary : theme.accent)
            }
            .buttonStyle(.plain)
            .disabled(viewModel.inputText.isEmpty || viewModel.isLoading)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(theme.chromeBackground)
    }
}

private struct AgentMessageBubble: View {
    let message: AgentMessage
    let theme: TraceTheme.CapturePalette

    var body: some View {
        if message.role == "user" {
            HStack {
                Spacer(minLength: 40)
                Text(message.content)
                    .font(.system(size: 13))
                    .foregroundStyle(theme.textPrimary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(theme.accent.opacity(0.12))
                    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            }
            .padding(.horizontal, 16)
        } else if message.role == "assistant" {
            HStack(alignment: .top) {
                Image(systemName: "sparkles")
                    .font(.system(size: 10))
                    .foregroundStyle(theme.accent)
                    .frame(width: 20, height: 20)
                    .background(theme.accent.opacity(0.12))
                    .clipShape(Circle())

                Text(message.content.isEmpty ? "..." : message.content)
                    .font(.system(size: 13))
                    .foregroundStyle(theme.textPrimary)
                    .textSelection(.enabled)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(theme.surface.opacity(0.6))
                    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))

                Spacer(minLength: 40)
            }
            .padding(.horizontal, 16)
        }
    }
}

private struct AgentTypingIndicator: View {
    let theme: TraceTheme.CapturePalette

    var body: some View {
        HStack(spacing: 6) {
            ForEach(0..<3, id: \.self) { index in
                Circle()
                    .fill(theme.textSecondary.opacity(0.5))
                    .frame(width: 6, height: 6)
                    .scaleEffect(1.0)
                    .animation(
                        .easeInOut(duration: 0.6)
                        .repeatForever()
                        .delay(Double(index) * 0.2),
                        value: true
                    )
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

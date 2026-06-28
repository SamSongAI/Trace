import SwiftUI

struct AgentChatView: View {
    @ObservedObject var viewModel: AgentChatViewModel
    @ObservedObject var settings: AppSettings
    let theme: TraceTheme.CapturePalette
    @Binding var isPresented: Bool
    @FocusState private var inputFocused: Bool

    var body: some View {
        if !viewModel.isConfigured {
            notConfiguredView
        } else {
            chatView
        }
    }

    private var notConfiguredView: some View {
        VStack(spacing: 16) {
            Image(systemName: "brain.head.profile")
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
        .onAppear {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                inputFocused = true
            }
        }
    }

    private var inputBar: some View {
        HStack(alignment: .center, spacing: 8) {
            TextField(L10n.aiInputPlaceholder, text: $viewModel.inputText, axis: .vertical)
                .font(.system(size: 13))
                .textFieldStyle(.plain)
                .lineLimit(1...5)
                .foregroundStyle(theme.textPrimary)
                .tint(theme.accent)
                .focused($inputFocused)
                .onSubmit {
                    Task { await viewModel.send() }
                }

            Button {
                if viewModel.isLoading {
                    viewModel.stop()
                } else {
                    Task { await viewModel.send() }
                }
            } label: {
                Image(systemName: viewModel.isLoading ? "stop.fill" : "arrow.up.circle.fill")
                    .font(.system(size: 22))
                    .foregroundStyle(viewModel.isLoading ? theme.accentStrong : (canSend ? theme.accent : theme.textSecondary.opacity(0.3)))
            }
            .buttonStyle(.plain)
            .disabled(!canSend && !canStop)
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

    private func scrollToBottom(_ proxy: ScrollViewProxy, animated: Bool = true) {
        let targetId: String
        if viewModel.isSavingMemory {
            targetId = "saving"
        } else if !viewModel.streamingText.isEmpty {
            targetId = "streaming"
        } else if !viewModel.events.isEmpty {
            targetId = "events"
        } else if viewModel.isLoading {
            targetId = "typing"
        } else {
            targetId = viewModel.messages.last?.id.uuidString ?? "typing"
        }
        if animated {
            withAnimation(.easeOut(duration: 0.2)) {
                proxy.scrollTo(targetId, anchor: .bottom)
            }
        } else {
            proxy.scrollTo(targetId, anchor: .bottom)
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
                scrollToBottom(proxy, animated: true)
            }
            .onChange(of: viewModel.isLoading) { loading in
                if loading {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                        scrollToBottom(proxy, animated: true)
                    }
                }
            }
            .onChange(of: viewModel.streamingText) { _ in
                scrollToBottom(proxy, animated: false)
            }
            .onChange(of: viewModel.events.count) { _ in
                scrollToBottom(proxy, animated: false)
            }
        }
    }

    private var canSend: Bool {
        !viewModel.inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !viewModel.isLoading
    }

    private var canStop: Bool {
        viewModel.isLoading
    }
}

// MARK: - Message Bubble with Markdown

private struct AgentMessageBubble: View {
    let message: AgentMessage
    let theme: TraceTheme.CapturePalette
    @State private var isHovering = false
    @State private var copied = false

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
                .overlay(alignment: .topTrailing) {
                    if isHovering {
                        copyButton
                            .padding(4)
                    }
                }
        }
        .padding(.horizontal, 12)
        .onHover { isHovering = $0 }
    }

    private var assistantBubble: some View {
        HStack(alignment: .top, spacing: 6) {
            Image(systemName: "brain.head.profile")
                .font(.system(size: 10))
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
            .overlay(alignment: .topTrailing) {
                if isHovering {
                    copyButton
                        .padding(4)
                }
            }

            Spacer(minLength: 30)
        }
        .padding(.horizontal, 12)
        .onHover { isHovering = $0 }
    }

    private var copyButton: some View {
        Button {
            if let content = message.content {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(content, forType: .string)
                copied = true
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                    copied = false
                }
            }
        } label: {
            Image(systemName: copied ? "checkmark" : "doc.on.doc")
                .font(.system(size: 9))
                .foregroundStyle(copied ? theme.accent : theme.textSecondary)
                .padding(3)
                .background(theme.panelBackground.opacity(0.9))
                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Streaming Bubble

private struct AgentStreamingBubble: View {
    let text: String
    let theme: TraceTheme.CapturePalette

    var body: some View {
        HStack(alignment: .top, spacing: 6) {
            Image(systemName: "brain.head.profile")
                .font(.system(size: 10))
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
    @State private var expandedTools: Set<UUID> = []

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
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
                    toolRow(event)

                case .toolResult:
                    EmptyView()
                }
            }
        }
        .padding(.horizontal, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func toolRow(_ event: AgentEvent) -> some View {
        let isExpanded = expandedTools.contains(event.id)
        let resultEvent = events.first(where: { $0.kind == .toolResult && $0.toolName == event.toolName })

        return VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 4) {
                Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                    .font(.system(size: 7))
                    .foregroundStyle(theme.textSecondary.opacity(0.4))

                ProgressView()
                    .scaleEffect(0.5)
                    .frame(width: 10, height: 10)

                Image(systemName: "wrench.and.screwdriver")
                    .font(.system(size: 8))

                Text(event.toolName ?? "tool")
                    .font(.system(size: 10, weight: .medium))

                if resultEvent != nil {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 8))
                        .foregroundStyle(theme.textSecondary.opacity(0.5))
                }
            }
            .foregroundStyle(theme.accent)
            .contentShape(Rectangle())
            .onTapGesture {
                if isExpanded {
                    expandedTools.remove(event.id)
                } else {
                    expandedTools.insert(event.id)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(theme.accent.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))

            if isExpanded {
                VStack(alignment: .leading, spacing: 4) {
                    if let args = event.toolArgs, !args.isEmpty {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("args")
                                .font(.system(size: 8, weight: .medium))
                                .foregroundStyle(theme.textSecondary.opacity(0.4))
                            Text(args)
                                .font(.system(size: 9, design: .monospaced))
                                .foregroundStyle(theme.textSecondary.opacity(0.6))
                                .lineLimit(4)
                        }
                    }
                    if let result = resultEvent?.toolResult, !result.isEmpty {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("result")
                                .font(.system(size: 8, weight: .medium))
                                .foregroundStyle(theme.textSecondary.opacity(0.4))
                            Text(result)
                                .font(.system(size: 9, design: .monospaced))
                                .foregroundStyle(theme.textSecondary.opacity(0.6))
                                .lineLimit(6)
                        }
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
                .padding(.leading, 20)
            }
        }
    }
}

// MARK: - Markdown Text Renderer

private struct AgentMarkdownText: View {
    let text: String
    let theme: TraceTheme.CapturePalette

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(parseBlocks().enumerated()), id: \.offset) { _, block in
                renderBlock(block)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contextMenu {
            Button("Copy") {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(text, forType: .string)
            }
        }
    }

    private enum Block {
        case paragraph(AttributedString)
        case codeBlock(String)
        case bulletList([AttributedString])
        case header(AttributedString, Int)
    }

    private func parseBlocks() -> [Block] {
        var blocks: [Block] = []
        let lines = text.components(separatedBy: "\n")
        var i = 0

        while i < lines.count {
            let line = lines[i]

            // Code block ```
            if line.hasPrefix("```") {
                var codeLines: [String] = []
                i += 1
                while i < lines.count && !lines[i].hasPrefix("```") {
                    codeLines.append(lines[i])
                    i += 1
                }
                i += 1
                blocks.append(.codeBlock(codeLines.joined(separator: "\n")))
                continue
            }

            // Headers
            if line.hasPrefix("### ") {
                blocks.append(.header(parseInline(String(line.dropFirst(4))), 3))
                i += 1
                continue
            }
            if line.hasPrefix("## ") {
                blocks.append(.header(parseInline(String(line.dropFirst(3))), 2))
                i += 1
                continue
            }
            if line.hasPrefix("# ") {
                blocks.append(.header(parseInline(String(line.dropFirst(2))), 1))
                i += 1
                continue
            }

            // Bullet list
            if line.hasPrefix("- ") || line.hasPrefix("* ") {
                var items: [AttributedString] = []
                while i < lines.count && (lines[i].hasPrefix("- ") || lines[i].hasPrefix("* ")) {
                    let item = lines[i].dropFirst(2)
                    items.append(parseInline(String(item)))
                    i += 1
                }
                blocks.append(.bulletList(items))
                continue
            }

            // Empty line
            if line.trimmingCharacters(in: .whitespaces).isEmpty {
                i += 1
                continue
            }

            // Paragraph (collect consecutive non-empty, non-special lines)
            var paraLines: [String] = []
            while i < lines.count {
                let l = lines[i]
                if l.trimmingCharacters(in: .whitespaces).isEmpty || l.hasPrefix("```") || l.hasPrefix("# ") || l.hasPrefix("## ") || l.hasPrefix("### ") || l.hasPrefix("- ") || l.hasPrefix("* ") {
                    break
                }
                paraLines.append(l)
                i += 1
            }
            if !paraLines.isEmpty {
                blocks.append(.paragraph(parseInline(paraLines.joined(separator: "\n"))))
            }
        }

        return blocks
    }

    private func renderBlock(_ block: Block) -> some View {
        switch block {
        case .paragraph(let att):
            return AnyView(
                Text(att)
                    .font(.system(size: 13))
                    .foregroundStyle(theme.textPrimary)
                    .textSelection(.enabled)
            )

        case .codeBlock(let code):
            return AnyView(
                Text(code)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(theme.textPrimary)
                    .textSelection(.enabled)
                    .padding(8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme.surface.opacity(0.6))
                    .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .contextMenu {
                        Button("Copy") {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(code, forType: .string)
                        }
                    }
            )

        case .bulletList(let items):
            return AnyView(
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(Array(items.enumerated()), id: \.offset) { _, item in
                        HStack(alignment: .top, spacing: 6) {
                            Text("\u{2022}")
                                .font(.system(size: 13))
                                .foregroundStyle(theme.textSecondary)
                            Text(item)
                                .font(.system(size: 13))
                                .foregroundStyle(theme.textPrimary)
                                .textSelection(.enabled)
                        }
                    }
                }
            )

        case .header(let att, let level):
            let size: CGFloat = level == 1 ? 16 : (level == 2 ? 14 : 13)
            return AnyView(
                Text(att)
                    .font(.system(size: size, weight: .bold))
                    .foregroundStyle(theme.textPrimary)
                    .textSelection(.enabled)
            )
        }
    }

    private func parseInline(_ text: String) -> AttributedString {
        var att = AttributedString(text)

        // File paths: /Users/... or ~/... or vault-relative .md/.txt files
        if let regex = try? NSRegularExpression(pattern: #"(?:~/[\w./-]+|/Users/[\w./-]+|/tmp/[\w./-]+)"#) {
            let nsString = text as NSString
            let matches = regex.matches(in: text, range: NSRange(location: 0, length: nsString.length))
            for m in matches.reversed() {
                let path = nsString.substring(with: m.range)
                if let range = att.range(of: path) {
                    var link = AttributedString(path)
                    link.foregroundColor = theme.accent
                    link.underlineStyle = .single
                    let expandedPath = (path as NSString).expandingTildeInPath
                    link.link = URL(fileURLWithPath: expandedPath)
                    att.replaceSubrange(range, with: link)
                }
            }
        }

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
            Image(systemName: "brain.head.profile")
                .font(.system(size: 10))
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

// MARK: - Sidebar

struct AgentSidebarView: View {
    @ObservedObject var viewModel: AgentChatViewModel
    let theme: TraceTheme.CapturePalette
    var onClose: () -> Void = {}

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 6) {
                Text("History")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(theme.textSecondary)

                Spacer()

                Button {
                    viewModel.newSession()
                } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(theme.textSecondary)
                }
                .buttonStyle(.plain)
                .help("New chat")

                Button {
                    onClose()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 9))
                        .foregroundStyle(theme.textSecondary.opacity(0.6))
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)

            Divider().overlay(theme.border)

            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(viewModel.sessions) { session in
                        sessionRow(session)
                    }
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
            }
        }
        .frame(width: 180)
    }

    private func sessionRow(_ session: AgentSession) -> some View {
        let isActive = session.id == viewModel.currentSessionId

        return HStack(spacing: 6) {
            VStack(alignment: .leading, spacing: 2) {
                Text(session.title)
                    .font(.system(size: 11, weight: isActive ? .semibold : .regular))
                    .foregroundStyle(isActive ? theme.textPrimary : theme.textSecondary)
                    .lineLimit(1)

                Text(session.preview)
                    .font(.system(size: 9))
                    .foregroundStyle(theme.textSecondary.opacity(0.5))
                    .lineLimit(1)
            }

            Spacer()

            Button {
                viewModel.deleteSession(session.id)
            } label: {
                Image(systemName: "trash")
                    .font(.system(size: 8))
                    .foregroundStyle(theme.textSecondary.opacity(0.4))
            }
            .buttonStyle(.plain)
            .opacity(isActive ? 1 : 0)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(isActive ? theme.accent.opacity(0.08) : Color.clear)
        .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
        .contentShape(Rectangle())
        .onTapGesture {
            viewModel.switchToSession(session.id)
        }
    }
}

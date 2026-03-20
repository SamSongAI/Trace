import AppKit
import SwiftUI

private typealias CaptureTheme = TraceTheme.CapturePalette

struct CaptureView: View {
    @ObservedObject var viewModel: CaptureViewModel
    @ObservedObject var settings: AppSettings
    let onPasteImage: ((NSImage) -> String?)?

    @State private var inputFocused = false

    private var theme: CaptureTheme {
        settings.appTheme.capture
    }

    private var editorTheme: CaptureTextEditor.Theme {
        settings.appTheme.editor
    }

    private var editorPlaceholder: String {
        switch settings.noteWriteMode {
        case .dimension:
            return "输入笔记内容..."
        case .file:
            return "输入文档内容..."
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider().overlay(theme.border)
            editor
            switch settings.noteWriteMode {
            case .dimension:
                modeFooter
            case .file:
                documentFooter
            }
        }
        .frame(minWidth: 360, minHeight: 220)
        .background(theme.panelBackground)
        .onAppear {
            focusInputSoon()
        }
        .onReceive(NotificationCenter.default.publisher(for: .traceFocusInput)) { _ in
            focusInputSoon()
        }
    }

    private var header: some View {
        HStack(spacing: 8) {
            VStack(alignment: .leading, spacing: 1) {
                Text(BrandAssets.displayName)
                    .font(.custom("Lora", size: 14))
                    .fontWeight(.bold)
                    .foregroundStyle(theme.textPrimary)
                    .lineLimit(1)

                Text(BrandAssets.slogan)
                    .font(.custom("Lora", size: 10))
                    .fontWeight(.semibold)
                    .foregroundStyle(theme.caption)
                    .lineLimit(1)
            }

            Spacer()

            Button {
                viewModel.pinned.toggle()
            } label: {
                Image(systemName: viewModel.pinned ? "pin.fill" : "pin")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(viewModel.pinned ? theme.accent : theme.iconMuted)
            }
            .buttonStyle(.plain)
            .help("Pin 模式")
        }
        .padding(.horizontal, 16)
        .frame(height: 52)
        .background(theme.chromeBackground)
    }

    private var editor: some View {
        CaptureTextEditor(
            text: $viewModel.text,
            isFocused: $inputFocused,
            placeholder: editorPlaceholder,
            theme: editorTheme,
            onPasteImage: onPasteImage
        )
        .background(theme.panelBackground)
    }

    private var documentFooter: some View {
        VStack(spacing: 0) {
            Divider().overlay(theme.border)

            documentTitleField
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
        }
        .background(theme.chromeBackground)
    }

    private var documentTitleField: some View {
        ZStack(alignment: .leading) {
            if viewModel.fileTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                Text("文档标题（可选）")
                    .font(.custom("Lora", size: 13))
                    .foregroundStyle(theme.caption)
            }

            TextField("", text: $viewModel.fileTitle)
                .textFieldStyle(.plain)
                .font(.custom("Lora", size: 13))
                .foregroundStyle(theme.textPrimary)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .background(theme.surface.opacity(0.92))
        .overlay {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .stroke(theme.border, lineWidth: 1)
        }
        .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private var modeFooter: some View {
        sectionBar
            .background(theme.chromeBackground)
    }

    private var sectionBar: some View {
        VStack(spacing: 0) {
            Divider().overlay(theme.border)

            sectionButtons
                .padding(.horizontal, 16)
                .padding(.vertical, 10)
        }
    }

    private var sectionButtons: some View {
        HStack(spacing: 10) {
            ForEach(NoteSection.allCases) { section in
                Button {
                    viewModel.selectedSection = section
                } label: {
                    Text(settings.title(for: section))
                        .lineLimit(1)
                        .font(.custom("Lora", size: 13))
                        .fontWeight(.bold)
                        .foregroundStyle(viewModel.selectedSection == section ? theme.selectedText : theme.textSecondary)
                        .frame(maxWidth: .infinity)
                        .frame(height: 38)
                        .background(viewModel.selectedSection == section ? theme.accentStrong : theme.surface)
                        .overlay {
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(
                                    viewModel.selectedSection == section ? theme.accent : theme.border,
                                    lineWidth: 1
                                )
                        }
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func focusInputSoon() {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
            inputFocused = true
        }
    }
}

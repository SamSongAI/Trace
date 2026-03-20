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
            Text(BrandAssets.displayName)
                .font(.custom("Lora", size: 13))
                .fontWeight(.bold)
                .foregroundStyle(theme.textPrimary)
                .lineLimit(1)

            Spacer()

            Button {
                viewModel.pinned.toggle()
            } label: {
                Image(systemName: viewModel.pinned ? "pin.fill" : "pin")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(viewModel.pinned ? theme.accent : theme.iconMuted)
            }
            .buttonStyle(.plain)
            .help("Pin")
        }
        .padding(.horizontal, 16)
        .frame(height: 36)
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
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
        }
        .background(theme.chromeBackground)
    }

    private var documentTitleField: some View {
        ZStack(alignment: .leading) {
            if viewModel.fileTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                Text("标题（可选）")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(theme.caption)
            }

            TextField("", text: $viewModel.fileTitle)
                .textFieldStyle(.plain)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(theme.textPrimary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(theme.surface.opacity(0.8))
        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
    }

    private var modeFooter: some View {
        sectionBar
            .background(theme.chromeBackground)
    }

    private var sectionBar: some View {
        VStack(spacing: 0) {
            Divider().overlay(theme.border)

            sectionButtons
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
        }
    }

    private var sectionButtons: some View {
        HStack(spacing: 6) {
            ForEach(NoteSection.allCases) { section in
                Button {
                    viewModel.selectedSection = section
                } label: {
                    Text(settings.title(for: section))
                        .lineLimit(1)
                        .font(.system(size: 12, weight: viewModel.selectedSection == section ? .semibold : .medium))
                        .foregroundStyle(viewModel.selectedSection == section ? theme.selectedText : theme.textSecondary)
                        .frame(maxWidth: .infinity)
                        .frame(height: 30)
                        .background(viewModel.selectedSection == section ? theme.accentStrong : theme.surface.opacity(0.6))
                        .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
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

import AppKit
import SwiftUI

private typealias CaptureTheme = TraceTheme.CapturePalette

private struct SectionGridWidthPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}

struct CaptureView: View {
    @ObservedObject var viewModel: CaptureViewModel
    @ObservedObject var settings: AppSettings
    let onPasteImage: ((NSImage) -> String?)?

    @State private var inputFocused = false
    @State private var sectionGridWidth: CGFloat = 0

    private let sectionGridSpacing: CGFloat = 6
    private let minimumSectionButtonWidth: CGFloat = 92
    private let maximumSectionRows = 3

    private var theme: CaptureTheme {
        settings.appTheme.capture
    }

    private var editorTheme: CaptureTextEditor.Theme {
        settings.appTheme.editor
    }

    private var editorPlaceholder: String {
        switch settings.noteWriteMode {
        case .dimension: return L10n.notePlaceholder
        case .file: return L10n.documentPlaceholder
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
        .overlay(alignment: .bottom) {
            if let message = viewModel.toastMessage {
                Text(message)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(theme.textSecondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(theme.surface.opacity(0.95))
                    .clipShape(Capsule())
                    .shadow(color: .black.opacity(0.15), radius: 4, y: 2)
                    .padding(.bottom, 12)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                    .animation(.easeInOut(duration: 0.2), value: viewModel.toastMessage)
            }
        }
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
            .help(L10n.pinPanelHelp)

            Button {
                NotificationCenter.default.post(name: .traceOpenSettings, object: nil)
            } label: {
                Image(systemName: "gearshape")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(theme.iconMuted)
            }
            .buttonStyle(.plain)
            .help(L10n.settingsTooltip)
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
        TextField(L10n.documentTitlePlaceholder, text: $viewModel.fileTitle)
            .textFieldStyle(.plain)
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(theme.textPrimary)
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
        .onPreferenceChange(SectionGridWidthPreferenceKey.self) { width in
            sectionGridWidth = width
        }
    }

    private var sectionButtons: some View {
        LazyVGrid(columns: sectionGridColumns, spacing: sectionGridSpacing) {
            ForEach(settings.sections) { section in
                Button {
                    viewModel.selectedSection = section
                } label: {
                    Text(settings.title(for: section))
                        .multilineTextAlignment(.center)
                        .lineLimit(2)
                        .fixedSize(horizontal: false, vertical: true)
                        .font(.system(size: 12, weight: viewModel.selectedSection == section ? .semibold : .medium))
                        .foregroundStyle(viewModel.selectedSection == section ? theme.selectedText : theme.textSecondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 8)
                        .frame(maxWidth: .infinity)
                        .frame(minHeight: 34)
                        .background(viewModel.selectedSection == section ? theme.accentStrong : theme.surface.opacity(0.6))
                        .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
                }
                .buttonStyle(.plain)
            }
        }
        .background(
            GeometryReader { proxy in
                Color.clear.preference(key: SectionGridWidthPreferenceKey.self, value: proxy.size.width)
            }
        )
    }

    private var sectionGridColumns: [GridItem] {
        let columnCount = sectionColumnCount(for: sectionGridWidth, itemCount: settings.sections.count)
        return Array(
            repeating: GridItem(.flexible(), spacing: sectionGridSpacing, alignment: .top),
            count: columnCount
        )
    }

    private func sectionColumnCount(for width: CGFloat, itemCount: Int) -> Int {
        guard itemCount > 0 else { return 1 }

        let minimumColumnsForThreeRows = Int(ceil(Double(itemCount) / Double(maximumSectionRows)))
        guard width > 0 else { return min(itemCount, max(1, minimumColumnsForThreeRows)) }

        let widthBasedColumns = max(
            1,
            Int((width + sectionGridSpacing) / (minimumSectionButtonWidth + sectionGridSpacing))
        )
        return min(itemCount, max(minimumColumnsForThreeRows, widthBasedColumns))
    }

    private func focusInputSoon() {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
            inputFocused = true
        }
    }
}

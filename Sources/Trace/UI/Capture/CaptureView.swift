import AppKit
import SwiftUI

private typealias CaptureTheme = TraceTheme.CapturePalette

private struct SectionGridWidthPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}

private struct ThreadGridWidthPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}

struct CaptureView: View {
    @ObservedObject var viewModel: CaptureViewModel
    @ObservedObject var settings: AppSettings
    let onPasteImages: (([NSImage]) -> [String])?

    @State private var inputFocused = false
    @State private var sectionGridWidth: CGFloat = 0
    @State private var threadGridWidth: CGFloat = 0
    @State private var previewImagePath: String?

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
        case .thread: return L10n.threadPlaceholder
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider().overlay(theme.border)
            editor
            if !viewModel.pastedImagePaths.isEmpty {
                thumbnailStrip
            }
            switch settings.noteWriteMode {
            case .dimension:
                modeFooter
            case .thread:
                threadFooter
            }
        }
        .frame(minWidth: 360, minHeight: 220)
        .background(theme.panelBackground)
        .overlay(alignment: .center) {
            if let message = viewModel.toastMessage, previewImagePath == nil {
                Text(message)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(theme.textSecondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(theme.surface.opacity(0.95))
                    .clipShape(Capsule())
                    .shadow(color: .black.opacity(0.15), radius: 4, y: 2)
                    .transition(.opacity)
                    .animation(.easeInOut(duration: 0.2), value: viewModel.toastMessage)
            }
        }
        .overlay {
            if let path = previewImagePath, let nsImage = NSImage(contentsOfFile: path) {
                ZStack {
                    Color.black.opacity(0.6)
                        .ignoresSafeArea()
                        .onTapGesture {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                previewImagePath = nil
                            }
                        }

                    Image(nsImage: nsImage)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .padding(20)
                        .shadow(color: .black.opacity(0.3), radius: 8, y: 4)
                        .onTapGesture {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                previewImagePath = nil
                            }
                        }
                }
                .transition(.opacity)
            }
        }
        .onAppear {
            focusInputSoon()
        }
        .onChange(of: viewModel.text) { newText in
            viewModel.syncThumbnailsWithText(newText)
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
            onPasteImages: onPasteImages
        )
        .background(theme.panelBackground)
        .opacity(viewModel.isSending ? 0.3 : 1.0)
        .animation(.easeOut(duration: 0.2), value: viewModel.isSending)
    }

    private var thumbnailStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(viewModel.pastedImagePaths.indices, id: \.self) { index in
                    let path = viewModel.pastedImagePaths[index]
                    if let nsImage = NSImage(contentsOfFile: path) {
                        ZStack(alignment: .topTrailing) {
                            Image(nsImage: nsImage)
                                .resizable()
                                .aspectRatio(contentMode: .fill)
                                .frame(width: 48, height: 48)
                                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                                        .stroke(theme.border, lineWidth: 1)
                                )
                                .help(path as String)
                                .onTapGesture {
                                    withAnimation(.easeInOut(duration: 0.2)) {
                                        previewImagePath = path
                                    }
                                }

                            Button {
                                removeThumbnail(at: index)
                            } label: {
                                Image(systemName: "xmark.circle.fill")
                                    .font(.system(size: 12))
                                    .foregroundStyle(theme.textSecondary, theme.surface)
                            }
                            .buttonStyle(.plain)
                            .padding(2)
                        }
                    }
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
        }
        .frame(height: 60)
        .background(theme.surface.opacity(0.4))
        .overlay(Divider().overlay(theme.border), alignment: .top)
    }

    private func removeThumbnail(at index: Int) {
        guard viewModel.pastedImagePaths.indices.contains(index) else { return }
        let markdown = viewModel.pastedImageMarkdowns[index]
        viewModel.pastedImagePaths.remove(at: index)
        viewModel.pastedImageMarkdowns.remove(at: index)
        if !markdown.isEmpty {
            viewModel.text = viewModel.text
                .replacingOccurrences(of: markdown + "\n", with: "")
                .replacingOccurrences(of: markdown, with: "")
        }
    }

    private var modeFooter: some View {
        sectionBar
            .background(theme.chromeBackground)
    }

    private var threadFooter: some View {
        threadBar
            .background(theme.chromeBackground)
    }

    private var threadBar: some View {
        VStack(spacing: 0) {
            Divider().overlay(theme.border)

            threadButtons
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
        }
        .onPreferenceChange(ThreadGridWidthPreferenceKey.self) { width in
            threadGridWidth = width
        }
    }

    private var threadButtons: some View {
        LazyVGrid(columns: threadGridColumns, spacing: sectionGridSpacing) {
            ForEach(Array(settings.threadConfigs.sorted(by: { $0.order < $1.order }).enumerated()), id: \.element.id) { index, thread in
                Button {
                    viewModel.selectedThread = thread
                } label: {
                    Text(thread.name)
                        .lineLimit(1)
                    .font(.system(size: 12, weight: viewModel.selectedThread?.id == thread.id ? .semibold : .medium))
                    .foregroundStyle(viewModel.selectedThread?.id == thread.id ? theme.selectedText : theme.textSecondary)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 8)
                    .frame(maxWidth: .infinity)
                    .frame(minHeight: 34)
                    .background(viewModel.selectedThread?.id == thread.id ? theme.accentStrong : theme.surface.opacity(0.6))
                    .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
                }
                .buttonStyle(.plain)
            }
        }
        .background(
            GeometryReader { proxy in
                Color.clear.preference(key: ThreadGridWidthPreferenceKey.self, value: proxy.size.width)
            }
        )
    }

    private var threadGridColumns: [GridItem] {
        let columnCount = threadColumnCount(for: threadGridWidth, itemCount: settings.threadConfigs.count)
        return Array(
            repeating: GridItem(.flexible(), spacing: sectionGridSpacing, alignment: .top),
            count: columnCount
        )
    }

    private func threadColumnCount(for width: CGFloat, itemCount: Int) -> Int {
        guard itemCount > 0 else { return 1 }

        let minimumColumnsForThreeRows = Int(ceil(Double(itemCount) / Double(maximumSectionRows)))
        guard width > 0 else { return min(itemCount, max(1, minimumColumnsForThreeRows)) }

        let widthBasedColumns = max(
            1,
            Int((width + sectionGridSpacing) / (minimumSectionButtonWidth + sectionGridSpacing))
        )
        return min(itemCount, max(minimumColumnsForThreeRows, widthBasedColumns))
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


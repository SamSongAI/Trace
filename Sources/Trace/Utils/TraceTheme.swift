import AppKit
import SwiftUI

enum AppThemePreset: String, CaseIterable, Identifiable {
    case light
    case dark
    case paper
    case dune

    static let defaultValue: AppThemePreset = .dark

    var id: String { rawValue }

    var title: String {
        switch self {
        case .light:
            return "Light"
        case .dark:
            return "Dark"
        case .paper:
            return "Paper"
        case .dune:
            return "Dune"
        }
    }

    var summary: String {
        switch self {
        case .light:
            return "参考 Obsidian Light 的灰白底色和紫色强调，更干净、更通用。"
        case .dark:
            return "黑白灰夜间主题，保留足够对比度，避免紫色品牌偏移。"
        case .paper:
            return "纸张米白与墨黑正文，适合长时间阅读、整理和静态编辑。"
        case .dune:
            return "燕麦底与陶土橙强调，整体更暖、更柔和，也更有材料感。"
        }
    }

    var iconName: String {
        switch self {
        case .light:
            return "sun.max"
        case .dark:
            return "moon.stars"
        case .paper:
            return "doc.text"
        case .dune:
            return "sun.haze"
        }
    }

    static func resolved(fromStoredRawValue rawValue: String?) -> AppThemePreset? {
        switch rawValue {
        case AppThemePreset.light.rawValue, "classicLight":
            return .light
        case AppThemePreset.dark.rawValue, "classicDark", "obsidian", "linear", "cursor":
            return .dark
        case AppThemePreset.paper.rawValue, "notion":
            return .paper
        case AppThemePreset.dune.rawValue, "anthropic":
            return .dune
        default:
            return nil
        }
    }

    static func migrated(fromLegacyCaptureAppearance rawValue: String?) -> AppThemePreset? {
        switch rawValue {
        case "light":
            return .light
        case "dark":
            return .dark
        default:
            return nil
        }
    }

    var theme: TraceTheme {
        TraceTheme.make(for: self)
    }
}

struct TraceTheme {
    struct CapturePalette {
        let panelBackground: Color
        let chromeBackground: Color
        let surface: Color
        let border: Color
        let textPrimary: Color
        let textSecondary: Color
        let caption: Color
        let iconMuted: Color
        let accent: Color
        let accentStrong: Color
        let selectedSurface: Color
        let selectedText: Color
    }

    struct SettingsPalette {
        let shellTop: Color
        let shellMiddle: Color
        let shellBottom: Color
        let shellPrimaryGlow: Color
        let shellSecondaryGlow: Color
        let shellPanel: Color
        let shellPanelBorder: Color
        let cardBackground: Color
        let cardBorder: Color
        let cardShadow: Color
        let headerEyebrow: Color
        let headerTitle: Color
        let headerSubtitle: Color
        let sectionTitle: Color
        let sectionDescription: Color
        let rowLabel: Color
        let fieldBackground: Color
        let fieldBorder: Color
        let fieldText: Color
        let chipBackground: Color
        let chipText: Color
        let accent: Color
        let accentStrong: Color
        let primaryButtonText: Color
        let secondaryButtonBackground: Color
        let secondaryButtonBorder: Color
        let secondaryButtonText: Color
        let mutedText: Color
        let warningText: Color
    }

    let preset: AppThemePreset
    let capture: CapturePalette
    let settings: SettingsPalette
    let editor: CaptureTextEditor.Theme
    let previewSwatches: [Color]

    static func make(for preset: AppThemePreset) -> TraceTheme {
        switch preset {
        case .light:
            return TraceTheme(
                preset: preset,
                capture: CapturePalette(
                    panelBackground: Color(hex: 0xF4F3F8),
                    chromeBackground: Color(hex: 0xF8F7FC),
                    surface: Color(hex: 0xFFFFFF),
                    border: Color(hex: 0xD9D3E8),
                    textPrimary: Color(hex: 0x1D1A26),
                    textSecondary: Color(hex: 0x5C546E),
                    caption: Color(hex: 0x877E99),
                    iconMuted: Color(hex: 0x6F6684),
                    accent: Color(hex: 0xA079FF),
                    accentStrong: Color(hex: 0x6C31E3),
                    selectedSurface: Color(hex: 0x6C31E3),
                    selectedText: .white
                ),
                settings: SettingsPalette(
                    shellTop: Color(hex: 0xFBF9FF),
                    shellMiddle: Color(hex: 0xF1ECFA),
                    shellBottom: Color(hex: 0xE6DFF5),
                    shellPrimaryGlow: Color(hex: 0xA079FF, alpha: 0.22),
                    shellSecondaryGlow: Color(hex: 0xF5B8D4, alpha: 0.18),
                    shellPanel: Color.white.opacity(0.82),
                    shellPanelBorder: Color(hex: 0xD6CEE8, alpha: 0.8),
                    cardBackground: Color.white.opacity(0.96),
                    cardBorder: Color(hex: 0xDDD4EE),
                    cardShadow: Color.black.opacity(0.08),
                    headerEyebrow: Color(hex: 0x7E58D8),
                    headerTitle: Color(hex: 0x1E1930),
                    headerSubtitle: Color(hex: 0x625B77),
                    sectionTitle: Color(hex: 0x211B31),
                    sectionDescription: Color(hex: 0x615875),
                    rowLabel: Color(hex: 0x463E5B),
                    fieldBackground: Color.white,
                    fieldBorder: Color(hex: 0xD7CEE8),
                    fieldText: Color(hex: 0x201A2F),
                    chipBackground: Color(hex: 0xEFE8FA),
                    chipText: Color(hex: 0x4A3B74),
                    accent: Color(hex: 0xA079FF),
                    accentStrong: Color(hex: 0x6C31E3),
                    primaryButtonText: .white,
                    secondaryButtonBackground: Color.white.opacity(0.74),
                    secondaryButtonBorder: Color(hex: 0xD4CAE8, alpha: 0.92),
                    secondaryButtonText: Color(hex: 0x241D36),
                    mutedText: Color(hex: 0x7B7390),
                    warningText: Color(hex: 0x6C31E3)
                ),
                editor: makeEditorTheme(
                    text: 0x1F2328,
                    placeholder: 0xA79FBC,
                    insertion: 0x1F2328
                ),
                previewSwatches: [
                    Color(hex: 0xF8F7FC),
                    Color(hex: 0xA079FF),
                    Color(hex: 0x6C31E3),
                    Color(hex: 0x1D1A26)
                ]
            )
        case .dark:
            return TraceTheme(
                preset: preset,
                capture: CapturePalette(
                    panelBackground: Color(hex: 0x101010),
                    chromeBackground: Color(hex: 0x141414),
                    surface: Color(hex: 0x1B1B1B),
                    border: Color(hex: 0x343434),
                    textPrimary: Color(hex: 0xF5F5F5),
                    textSecondary: Color(hex: 0xDFDFDF),
                    caption: Color(hex: 0xA8A8A8),
                    iconMuted: Color(hex: 0xC8C8C8),
                    accent: Color(hex: 0xF5F5F5),
                    accentStrong: Color(hex: 0xFFFFFF),
                    selectedSurface: Color(hex: 0xF5F5F5),
                    selectedText: Color(hex: 0x101010)
                ),
                settings: SettingsPalette(
                    shellTop: Color(hex: 0x111111),
                    shellMiddle: Color(hex: 0x0C0C0C),
                    shellBottom: Color(hex: 0x080808),
                    shellPrimaryGlow: Color.white.opacity(0.06),
                    shellSecondaryGlow: Color.white.opacity(0.03),
                    shellPanel: Color(hex: 0x131313, alpha: 0.86),
                    shellPanelBorder: Color(hex: 0x2C2C2C, alpha: 0.96),
                    cardBackground: Color(hex: 0x171717),
                    cardBorder: Color(hex: 0x292929),
                    cardShadow: Color.black.opacity(0.34),
                    headerEyebrow: Color(hex: 0xD5D5D5),
                    headerTitle: Color(hex: 0xF5F5F5),
                    headerSubtitle: Color(hex: 0xCFCFCF),
                    sectionTitle: Color(hex: 0xF0F0F0),
                    sectionDescription: Color(hex: 0xBABABA),
                    rowLabel: Color(hex: 0xE2E2E2),
                    fieldBackground: Color(hex: 0x1A1A1A),
                    fieldBorder: Color(hex: 0x343434),
                    fieldText: Color(hex: 0xF5F5F5),
                    chipBackground: Color(hex: 0x202020),
                    chipText: Color(hex: 0xEEEEEE),
                    accent: Color(hex: 0xF5F5F5),
                    accentStrong: Color(hex: 0xFFFFFF),
                    primaryButtonText: Color(hex: 0x101010),
                    secondaryButtonBackground: Color.white.opacity(0.07),
                    secondaryButtonBorder: Color(hex: 0x3A3A3A, alpha: 0.96),
                    secondaryButtonText: Color(hex: 0xEFEFEF),
                    mutedText: Color(hex: 0xA8A8A8),
                    warningText: Color(hex: 0xF5F5F5)
                ),
                editor: makeEditorTheme(
                    text: 0xF5F5F5,
                    placeholder: 0x8F8F8F,
                    insertion: 0xFFFFFF
                ),
                previewSwatches: [
                    Color(hex: 0x101010),
                    Color(hex: 0x1A1A1A),
                    Color(hex: 0x7E7E7E),
                    Color(hex: 0xF5F5F5)
                ]
            )
        case .paper:
            return TraceTheme(
                preset: preset,
                capture: CapturePalette(
                    panelBackground: Color(hex: 0xF7F6F3),
                    chromeBackground: Color(hex: 0xFAF9F6),
                    surface: Color(hex: 0xFFFCF7),
                    border: Color(hex: 0xE7E1D8),
                    textPrimary: Color(hex: 0x191919),
                    textSecondary: Color(hex: 0x4F4A45),
                    caption: Color(hex: 0x7A746C),
                    iconMuted: Color(hex: 0x68635D),
                    accent: Color(hex: 0x383836),
                    accentStrong: Color(hex: 0x191919),
                    selectedSurface: Color(hex: 0x191919),
                    selectedText: Color(hex: 0xF0EFED)
                ),
                settings: SettingsPalette(
                    shellTop: Color(hex: 0xFCFBF8),
                    shellMiddle: Color(hex: 0xF4F1EA),
                    shellBottom: Color(hex: 0xECE7DE),
                    shellPrimaryGlow: Color(hex: 0xD6CEC2, alpha: 0.24),
                    shellSecondaryGlow: Color(hex: 0xF0EEE6, alpha: 0.18),
                    shellPanel: Color.white.opacity(0.78),
                    shellPanelBorder: Color(hex: 0xE0D9CF, alpha: 0.9),
                    cardBackground: Color(hex: 0xFFFCF7),
                    cardBorder: Color(hex: 0xE5DED5),
                    cardShadow: Color.black.opacity(0.06),
                    headerEyebrow: Color(hex: 0x383836),
                    headerTitle: Color(hex: 0x1B1B1A),
                    headerSubtitle: Color(hex: 0x5D5852),
                    sectionTitle: Color(hex: 0x1C1B1A),
                    sectionDescription: Color(hex: 0x635D56),
                    rowLabel: Color(hex: 0x3F3B36),
                    fieldBackground: Color.white,
                    fieldBorder: Color(hex: 0xE2DBD0),
                    fieldText: Color(hex: 0x1C1B1A),
                    chipBackground: Color(hex: 0xF3EEE6),
                    chipText: Color(hex: 0x3A3733),
                    accent: Color(hex: 0x383836),
                    accentStrong: Color(hex: 0x191919),
                    primaryButtonText: .white,
                    secondaryButtonBackground: Color.white.opacity(0.82),
                    secondaryButtonBorder: Color(hex: 0xDDD5CB, alpha: 0.96),
                    secondaryButtonText: Color(hex: 0x24211E),
                    mutedText: Color(hex: 0x7E776F),
                    warningText: Color(hex: 0x191919)
                ),
                editor: makeEditorTheme(
                    text: 0x191919,
                    placeholder: 0x9E968E,
                    insertion: 0x191919
                ),
                previewSwatches: [
                    Color(hex: 0xFFFCF7),
                    Color(hex: 0xF3EEE6),
                    Color(hex: 0x383836),
                    Color(hex: 0x191919)
                ]
            )
        case .dune:
            return TraceTheme(
                preset: preset,
                capture: CapturePalette(
                    panelBackground: Color(hex: 0xF0EEE6),
                    chromeBackground: Color(hex: 0xF5F2EA),
                    surface: Color(hex: 0xFFFBF3),
                    border: Color(hex: 0xDED6C8),
                    textPrimary: Color(hex: 0x2F2D29),
                    textSecondary: Color(hex: 0x5D5952),
                    caption: Color(hex: 0x87837B),
                    iconMuted: Color(hex: 0x706C65),
                    accent: Color(hex: 0xD97757),
                    accentStrong: Color(hex: 0xB85F3F),
                    selectedSurface: Color(hex: 0xB85F3F),
                    selectedText: .white
                ),
                settings: SettingsPalette(
                    shellTop: Color(hex: 0xF7F3E9),
                    shellMiddle: Color(hex: 0xEFE8DC),
                    shellBottom: Color(hex: 0xE5DCCD),
                    shellPrimaryGlow: Color(hex: 0xD97757, alpha: 0.22),
                    shellSecondaryGlow: Color(hex: 0xE8E0D0, alpha: 0.18),
                    shellPanel: Color.white.opacity(0.74),
                    shellPanelBorder: Color(hex: 0xDDD2C0, alpha: 0.9),
                    cardBackground: Color(hex: 0xFFF9EF),
                    cardBorder: Color(hex: 0xE3D8C7),
                    cardShadow: Color.black.opacity(0.08),
                    headerEyebrow: Color(hex: 0xC4694B),
                    headerTitle: Color(hex: 0x2F2B25),
                    headerSubtitle: Color(hex: 0x6A645C),
                    sectionTitle: Color(hex: 0x302C27),
                    sectionDescription: Color(hex: 0x6C665F),
                    rowLabel: Color(hex: 0x4C463F),
                    fieldBackground: Color.white.opacity(0.96),
                    fieldBorder: Color(hex: 0xE2D7C6),
                    fieldText: Color(hex: 0x302C27),
                    chipBackground: Color(hex: 0xF3E7D8),
                    chipText: Color(hex: 0x5B4D43),
                    accent: Color(hex: 0xD97757),
                    accentStrong: Color(hex: 0xB85F3F),
                    primaryButtonText: .white,
                    secondaryButtonBackground: Color.white.opacity(0.82),
                    secondaryButtonBorder: Color(hex: 0xDCCFBE, alpha: 0.95),
                    secondaryButtonText: Color(hex: 0x342F28),
                    mutedText: Color(hex: 0x898277),
                    warningText: Color(hex: 0xB85F3F)
                ),
                editor: makeEditorTheme(
                    text: 0x2F2D29,
                    placeholder: 0xA6A197,
                    insertion: 0xD97757
                ),
                previewSwatches: [
                    Color(hex: 0xFFF9EF),
                    Color(hex: 0xF0EEE6),
                    Color(hex: 0xD97757),
                    Color(hex: 0xB85F3F)
                ]
            )
        }
    }
}

private func makeEditorTheme(
    text: UInt32,
    placeholder: UInt32,
    insertion: UInt32
) -> CaptureTextEditor.Theme {
    CaptureTextEditor.Theme(
        textColor: nsColor(text),
        placeholderColor: nsColor(placeholder),
        insertionPointColor: nsColor(insertion),
        editorFont: NSFont(name: "Lora-Regular", size: 15)
            ?? NSFont(name: "Lora", size: 15)
            ?? NSFont.systemFont(ofSize: 15, weight: .regular)
    )
}

private func nsColor(_ hex: UInt32, alpha: CGFloat = 1.0) -> NSColor {
    let red = CGFloat((hex >> 16) & 0xff) / 255.0
    let green = CGFloat((hex >> 8) & 0xff) / 255.0
    let blue = CGFloat(hex & 0xff) / 255.0
    return NSColor(srgbRed: red, green: green, blue: blue, alpha: alpha)
}

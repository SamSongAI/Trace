//! Platform-independent color palettes for the four Trace theme presets.
//!
//! The hex values and alpha channels in this module are transcribed verbatim
//! from the canonical Swift source at `Sources/Trace/Utils/TraceTheme.swift`.
//! Keeping the two in sync is a deliberate policy so the Windows port stays
//! visually identical to macOS on a per-pixel basis.
//!
//! This module is intentionally UI-framework agnostic. The `trace-ui` crate is
//! responsible for converting [`TraceColor`] values into whatever native color
//! type its rendering backend expects (e.g. `iced::Color`).
//!
//! # Deliberately out of scope
//!
//! * The AppKit-specific `CaptureTextEditor.Theme` (fonts + `NSColor`) is not
//!   ported here. Its three underlying hex values are exposed as the
//!   `editor_text`, `editor_placeholder` and `editor_insertion` fields of
//!   [`CapturePalette`] so the UI layer can assemble an editor theme later.
//! * Localized titles/summaries and SF Symbol icon names live in the UI layer.
//! * These palette types are pure runtime data derived from [`ThemePreset`]
//!   and are not serialized.

use crate::ThemePreset;

/// Platform-independent sRGB color expressed as 8-bit channels plus a
/// floating-point alpha.
///
/// Mirrors the shape of `SwiftUI.Color(hex:alpha:)` so palette values can be
/// transcribed from `TraceTheme.swift` without any semantic conversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceColor {
    /// 0-255 sRGB channel value.
    pub r: u8,
    /// 0-255 sRGB channel value.
    pub g: u8,
    /// 0-255 sRGB channel value.
    pub b: u8,
    /// 0.0-1.0 alpha. Swift uses `CGFloat` which maps cleanly to `f32` here.
    pub a: f32,
}

impl TraceColor {
    /// Builds a fully-opaque color from a 24-bit `0xRRGGBB` packed value.
    ///
    /// Mirrors Swift `Color(hex: 0xRRGGBB)` with default alpha `1.0`.
    pub const fn rgb(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as u8,
            g: ((hex >> 8) & 0xff) as u8,
            b: (hex & 0xff) as u8,
            a: 1.0,
        }
    }

    /// Builds a color with the given alpha from a 24-bit `0xRRGGBB` packed
    /// value.
    ///
    /// Mirrors Swift `Color(hex: 0xRRGGBB, alpha: α)`.
    pub const fn rgba(hex: u32, alpha: f32) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as u8,
            g: ((hex >> 8) & 0xff) as u8,
            b: (hex & 0xff) as u8,
            a: alpha,
        }
    }

    /// `Color.white.opacity(α)` — fully-white with the given alpha.
    pub const fn white(alpha: f32) -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: alpha,
        }
    }

    /// `Color.black.opacity(α)` — fully-black with the given alpha.
    pub const fn black(alpha: f32) -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: alpha,
        }
    }
}

/// Colors that paint the capture panel (the floating composer window).
///
/// Field order mirrors `TraceTheme.CapturePalette` in the Swift source. The
/// last three fields (`editor_*`) are the hex values that Swift passes into
/// `makeEditorTheme`; they are preserved here so the UI layer can derive its
/// editor theme without a second source of truth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapturePalette {
    pub panel_background: TraceColor,
    pub chrome_background: TraceColor,
    pub surface: TraceColor,
    pub border: TraceColor,
    pub text_primary: TraceColor,
    pub text_secondary: TraceColor,
    pub caption: TraceColor,
    pub icon_muted: TraceColor,
    pub accent: TraceColor,
    pub accent_strong: TraceColor,
    pub selected_surface: TraceColor,
    pub selected_text: TraceColor,
    /// Text color Swift passes into `CaptureTextEditor.Theme`. Phase 10 derives the iced editor style from this.
    pub editor_text: TraceColor,
    /// Placeholder color Swift passes into `CaptureTextEditor.Theme`. Phase 10 derives the iced editor style from this.
    pub editor_placeholder: TraceColor,
    /// Insertion-point (caret) color Swift passes into `CaptureTextEditor.Theme`. Phase 10 derives the iced editor style from this.
    pub editor_insertion: TraceColor,
}

/// Colors that paint the settings window. Field order mirrors
/// `TraceTheme.SettingsPalette` in the Swift source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingsPalette {
    pub shell_top: TraceColor,
    pub shell_middle: TraceColor,
    pub shell_bottom: TraceColor,
    pub shell_primary_glow: TraceColor,
    pub shell_secondary_glow: TraceColor,
    pub shell_panel: TraceColor,
    pub shell_panel_border: TraceColor,
    pub card_background: TraceColor,
    pub card_border: TraceColor,
    pub card_shadow: TraceColor,
    pub header_eyebrow: TraceColor,
    pub header_title: TraceColor,
    pub header_subtitle: TraceColor,
    pub section_title: TraceColor,
    pub section_description: TraceColor,
    pub row_label: TraceColor,
    pub field_background: TraceColor,
    pub field_border: TraceColor,
    pub field_text: TraceColor,
    pub chip_background: TraceColor,
    pub chip_text: TraceColor,
    pub accent: TraceColor,
    pub accent_strong: TraceColor,
    pub primary_button_text: TraceColor,
    pub secondary_button_background: TraceColor,
    pub secondary_button_border: TraceColor,
    pub secondary_button_text: TraceColor,
    pub muted_text: TraceColor,
    pub warning_text: TraceColor,
}

/// The complete resolved theme data for a single preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceTheme {
    pub preset: ThemePreset,
    pub capture: CapturePalette,
    pub settings: SettingsPalette,
    /// 4 swatches shown in the theme-picker preview strip. Order matches
    /// Swift `previewSwatches`.
    pub preview_swatches: [TraceColor; 4],
}

impl TraceTheme {
    /// Resolves the full palette for `preset`. Values are byte-identical to
    /// `TraceTheme.make(for:)` in the Swift source.
    pub const fn for_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Light => Self::light(),
            ThemePreset::Dark => Self::dark(),
            ThemePreset::Paper => Self::paper(),
            ThemePreset::Dune => Self::dune(),
        }
    }

    const fn light() -> Self {
        Self {
            preset: ThemePreset::Light,
            capture: CapturePalette {
                panel_background: TraceColor::rgb(0xF4F3F8),
                chrome_background: TraceColor::rgb(0xF8F7FC),
                surface: TraceColor::rgb(0xFFFFFF),
                border: TraceColor::rgb(0xD9D3E8),
                text_primary: TraceColor::rgb(0x1D1A26),
                text_secondary: TraceColor::rgb(0x5C546E),
                caption: TraceColor::rgb(0x877E99),
                icon_muted: TraceColor::rgb(0x6F6684),
                accent: TraceColor::rgb(0xA079FF),
                accent_strong: TraceColor::rgb(0x6C31E3),
                selected_surface: TraceColor::rgb(0x6C31E3),
                selected_text: TraceColor::white(1.0),
                editor_text: TraceColor::rgb(0x1F2328),
                editor_placeholder: TraceColor::rgb(0xA79FBC),
                editor_insertion: TraceColor::rgb(0x1F2328),
            },
            settings: SettingsPalette {
                shell_top: TraceColor::rgb(0xFBF9FF),
                shell_middle: TraceColor::rgb(0xF1ECFA),
                shell_bottom: TraceColor::rgb(0xE6DFF5),
                shell_primary_glow: TraceColor::rgba(0xA079FF, 0.22),
                shell_secondary_glow: TraceColor::rgba(0xF5B8D4, 0.18),
                shell_panel: TraceColor::white(0.82),
                shell_panel_border: TraceColor::rgba(0xD6CEE8, 0.8),
                card_background: TraceColor::white(0.96),
                card_border: TraceColor::rgb(0xDDD4EE),
                card_shadow: TraceColor::black(0.08),
                header_eyebrow: TraceColor::rgb(0x7E58D8),
                header_title: TraceColor::rgb(0x1E1930),
                header_subtitle: TraceColor::rgb(0x625B77),
                section_title: TraceColor::rgb(0x211B31),
                section_description: TraceColor::rgb(0x615875),
                row_label: TraceColor::rgb(0x463E5B),
                field_background: TraceColor::white(1.0),
                field_border: TraceColor::rgb(0xD7CEE8),
                field_text: TraceColor::rgb(0x201A2F),
                chip_background: TraceColor::rgb(0xEFE8FA),
                chip_text: TraceColor::rgb(0x4A3B74),
                accent: TraceColor::rgb(0xA079FF),
                accent_strong: TraceColor::rgb(0x6C31E3),
                primary_button_text: TraceColor::white(1.0),
                secondary_button_background: TraceColor::white(0.74),
                secondary_button_border: TraceColor::rgba(0xD4CAE8, 0.92),
                secondary_button_text: TraceColor::rgb(0x241D36),
                muted_text: TraceColor::rgb(0x7B7390),
                warning_text: TraceColor::rgb(0x6C31E3),
            },
            preview_swatches: [
                TraceColor::rgb(0xF8F7FC),
                TraceColor::rgb(0xA079FF),
                TraceColor::rgb(0x6C31E3),
                TraceColor::rgb(0x1D1A26),
            ],
        }
    }

    const fn dark() -> Self {
        Self {
            preset: ThemePreset::Dark,
            capture: CapturePalette {
                panel_background: TraceColor::rgb(0x101010),
                chrome_background: TraceColor::rgb(0x141414),
                surface: TraceColor::rgb(0x1B1B1B),
                border: TraceColor::rgb(0x343434),
                text_primary: TraceColor::rgb(0xF5F5F5),
                text_secondary: TraceColor::rgb(0xDFDFDF),
                caption: TraceColor::rgb(0xA8A8A8),
                icon_muted: TraceColor::rgb(0xC8C8C8),
                accent: TraceColor::rgb(0xF5F5F5),
                accent_strong: TraceColor::rgb(0xFFFFFF),
                selected_surface: TraceColor::rgb(0xF5F5F5),
                selected_text: TraceColor::rgb(0x101010),
                editor_text: TraceColor::rgb(0xF5F5F5),
                editor_placeholder: TraceColor::rgb(0xB0B0B0),
                editor_insertion: TraceColor::rgb(0xFFFFFF),
            },
            settings: SettingsPalette {
                shell_top: TraceColor::rgb(0x111111),
                shell_middle: TraceColor::rgb(0x0C0C0C),
                shell_bottom: TraceColor::rgb(0x080808),
                shell_primary_glow: TraceColor::white(0.06),
                shell_secondary_glow: TraceColor::white(0.03),
                shell_panel: TraceColor::rgba(0x131313, 0.86),
                shell_panel_border: TraceColor::rgba(0x2C2C2C, 0.96),
                card_background: TraceColor::rgb(0x171717),
                card_border: TraceColor::rgb(0x292929),
                card_shadow: TraceColor::black(0.34),
                header_eyebrow: TraceColor::rgb(0xD5D5D5),
                header_title: TraceColor::rgb(0xF5F5F5),
                header_subtitle: TraceColor::rgb(0xCFCFCF),
                section_title: TraceColor::rgb(0xF0F0F0),
                section_description: TraceColor::rgb(0xBABABA),
                row_label: TraceColor::rgb(0xE2E2E2),
                field_background: TraceColor::rgb(0x1A1A1A),
                field_border: TraceColor::rgb(0x343434),
                field_text: TraceColor::rgb(0xF5F5F5),
                chip_background: TraceColor::rgb(0x202020),
                chip_text: TraceColor::rgb(0xEEEEEE),
                accent: TraceColor::rgb(0xF5F5F5),
                accent_strong: TraceColor::rgb(0xFFFFFF),
                primary_button_text: TraceColor::rgb(0x101010),
                secondary_button_background: TraceColor::white(0.07),
                secondary_button_border: TraceColor::rgba(0x3A3A3A, 0.96),
                secondary_button_text: TraceColor::rgb(0xEFEFEF),
                muted_text: TraceColor::rgb(0xA8A8A8),
                warning_text: TraceColor::rgb(0xF5F5F5),
            },
            preview_swatches: [
                TraceColor::rgb(0x101010),
                TraceColor::rgb(0x1A1A1A),
                TraceColor::rgb(0x7E7E7E),
                TraceColor::rgb(0xF5F5F5),
            ],
        }
    }

    const fn paper() -> Self {
        Self {
            preset: ThemePreset::Paper,
            capture: CapturePalette {
                panel_background: TraceColor::rgb(0xF7F6F3),
                chrome_background: TraceColor::rgb(0xFAF9F6),
                surface: TraceColor::rgb(0xFFFCF7),
                border: TraceColor::rgb(0xE7E1D8),
                text_primary: TraceColor::rgb(0x191919),
                text_secondary: TraceColor::rgb(0x4F4A45),
                caption: TraceColor::rgb(0x7A746C),
                icon_muted: TraceColor::rgb(0x68635D),
                accent: TraceColor::rgb(0x383836),
                accent_strong: TraceColor::rgb(0x191919),
                selected_surface: TraceColor::rgb(0x191919),
                selected_text: TraceColor::rgb(0xF0EFED),
                editor_text: TraceColor::rgb(0x191919),
                editor_placeholder: TraceColor::rgb(0x9E968E),
                editor_insertion: TraceColor::rgb(0x191919),
            },
            settings: SettingsPalette {
                shell_top: TraceColor::rgb(0xFCFBF8),
                shell_middle: TraceColor::rgb(0xF4F1EA),
                shell_bottom: TraceColor::rgb(0xECE7DE),
                shell_primary_glow: TraceColor::rgba(0xD6CEC2, 0.24),
                shell_secondary_glow: TraceColor::rgba(0xF0EEE6, 0.18),
                shell_panel: TraceColor::white(0.78),
                shell_panel_border: TraceColor::rgba(0xE0D9CF, 0.9),
                card_background: TraceColor::rgb(0xFFFCF7),
                card_border: TraceColor::rgb(0xE5DED5),
                card_shadow: TraceColor::black(0.06),
                header_eyebrow: TraceColor::rgb(0x383836),
                header_title: TraceColor::rgb(0x1B1B1A),
                header_subtitle: TraceColor::rgb(0x5D5852),
                section_title: TraceColor::rgb(0x1C1B1A),
                section_description: TraceColor::rgb(0x635D56),
                row_label: TraceColor::rgb(0x3F3B36),
                field_background: TraceColor::white(1.0),
                field_border: TraceColor::rgb(0xE2DBD0),
                field_text: TraceColor::rgb(0x1C1B1A),
                chip_background: TraceColor::rgb(0xF3EEE6),
                chip_text: TraceColor::rgb(0x3A3733),
                accent: TraceColor::rgb(0x383836),
                accent_strong: TraceColor::rgb(0x191919),
                primary_button_text: TraceColor::white(1.0),
                secondary_button_background: TraceColor::white(0.82),
                secondary_button_border: TraceColor::rgba(0xDDD5CB, 0.96),
                secondary_button_text: TraceColor::rgb(0x24211E),
                muted_text: TraceColor::rgb(0x7E776F),
                warning_text: TraceColor::rgb(0x191919),
            },
            preview_swatches: [
                TraceColor::rgb(0xFFFCF7),
                TraceColor::rgb(0xF3EEE6),
                TraceColor::rgb(0x383836),
                TraceColor::rgb(0x191919),
            ],
        }
    }

    const fn dune() -> Self {
        Self {
            preset: ThemePreset::Dune,
            capture: CapturePalette {
                panel_background: TraceColor::rgb(0xF0EEE6),
                chrome_background: TraceColor::rgb(0xF5F2EA),
                surface: TraceColor::rgb(0xFFFBF3),
                border: TraceColor::rgb(0xDED6C8),
                text_primary: TraceColor::rgb(0x2F2D29),
                text_secondary: TraceColor::rgb(0x5D5952),
                caption: TraceColor::rgb(0x87837B),
                icon_muted: TraceColor::rgb(0x706C65),
                accent: TraceColor::rgb(0xD97757),
                accent_strong: TraceColor::rgb(0xB85F3F),
                selected_surface: TraceColor::rgb(0xB85F3F),
                selected_text: TraceColor::white(1.0),
                editor_text: TraceColor::rgb(0x2F2D29),
                editor_placeholder: TraceColor::rgb(0xA6A197),
                editor_insertion: TraceColor::rgb(0xD97757),
            },
            settings: SettingsPalette {
                shell_top: TraceColor::rgb(0xF7F3E9),
                shell_middle: TraceColor::rgb(0xEFE8DC),
                shell_bottom: TraceColor::rgb(0xE5DCCD),
                shell_primary_glow: TraceColor::rgba(0xD97757, 0.22),
                shell_secondary_glow: TraceColor::rgba(0xE8E0D0, 0.18),
                shell_panel: TraceColor::white(0.74),
                shell_panel_border: TraceColor::rgba(0xDDD2C0, 0.9),
                card_background: TraceColor::rgb(0xFFF9EF),
                card_border: TraceColor::rgb(0xE3D8C7),
                card_shadow: TraceColor::black(0.08),
                header_eyebrow: TraceColor::rgb(0xC4694B),
                header_title: TraceColor::rgb(0x2F2B25),
                header_subtitle: TraceColor::rgb(0x6A645C),
                section_title: TraceColor::rgb(0x302C27),
                section_description: TraceColor::rgb(0x6C665F),
                row_label: TraceColor::rgb(0x4C463F),
                field_background: TraceColor::white(0.96),
                field_border: TraceColor::rgb(0xE2D7C6),
                field_text: TraceColor::rgb(0x302C27),
                chip_background: TraceColor::rgb(0xF3E7D8),
                chip_text: TraceColor::rgb(0x5B4D43),
                accent: TraceColor::rgb(0xD97757),
                accent_strong: TraceColor::rgb(0xB85F3F),
                primary_button_text: TraceColor::white(1.0),
                secondary_button_background: TraceColor::white(0.82),
                secondary_button_border: TraceColor::rgba(0xDCCFBE, 0.95),
                secondary_button_text: TraceColor::rgb(0x342F28),
                muted_text: TraceColor::rgb(0x898277),
                warning_text: TraceColor::rgb(0xB85F3F),
            },
            preview_swatches: [
                TraceColor::rgb(0xFFF9EF),
                TraceColor::rgb(0xF0EEE6),
                TraceColor::rgb(0xD97757),
                TraceColor::rgb(0xB85F3F),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_color_rgb_unpacks_hex_correctly() {
        let c = TraceColor::rgb(0xF4F3F8);
        assert_eq!(c.r, 244);
        assert_eq!(c.g, 243);
        assert_eq!(c.b, 248);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn trace_color_rgba_preserves_alpha() {
        let c = TraceColor::rgba(0xA079FF, 0.22);
        assert_eq!(c.r, 160);
        assert_eq!(c.g, 121);
        assert_eq!(c.b, 255);
        assert_eq!(c.a, 0.22);
    }

    #[test]
    fn trace_color_white_black_helpers() {
        let w = TraceColor::white(0.5);
        assert_eq!(w.r, 255);
        assert_eq!(w.g, 255);
        assert_eq!(w.b, 255);
        assert_eq!(w.a, 0.5);

        let k = TraceColor::black(0.3);
        assert_eq!(k.r, 0);
        assert_eq!(k.g, 0);
        assert_eq!(k.b, 0);
        assert_eq!(k.a, 0.3);
    }

    #[test]
    fn light_preset_capture_palette() {
        let c = TraceTheme::for_preset(ThemePreset::Light).capture;
        assert_eq!(c.panel_background, TraceColor::rgb(0xF4F3F8));
        assert_eq!(c.chrome_background, TraceColor::rgb(0xF8F7FC));
        assert_eq!(c.surface, TraceColor::rgb(0xFFFFFF));
        assert_eq!(c.border, TraceColor::rgb(0xD9D3E8));
        assert_eq!(c.text_primary, TraceColor::rgb(0x1D1A26));
        assert_eq!(c.text_secondary, TraceColor::rgb(0x5C546E));
        assert_eq!(c.caption, TraceColor::rgb(0x877E99));
        assert_eq!(c.icon_muted, TraceColor::rgb(0x6F6684));
        assert_eq!(c.accent, TraceColor::rgb(0xA079FF));
        assert_eq!(c.accent_strong, TraceColor::rgb(0x6C31E3));
        assert_eq!(c.selected_surface, TraceColor::rgb(0x6C31E3));
        assert_eq!(c.selected_text, TraceColor::white(1.0));
        assert_eq!(c.editor_text, TraceColor::rgb(0x1F2328));
        assert_eq!(c.editor_placeholder, TraceColor::rgb(0xA79FBC));
        assert_eq!(c.editor_insertion, TraceColor::rgb(0x1F2328));
    }

    #[test]
    fn dark_preset_capture_palette() {
        let c = TraceTheme::for_preset(ThemePreset::Dark).capture;
        assert_eq!(c.panel_background, TraceColor::rgb(0x101010));
        assert_eq!(c.chrome_background, TraceColor::rgb(0x141414));
        assert_eq!(c.surface, TraceColor::rgb(0x1B1B1B));
        assert_eq!(c.border, TraceColor::rgb(0x343434));
        assert_eq!(c.text_primary, TraceColor::rgb(0xF5F5F5));
        assert_eq!(c.text_secondary, TraceColor::rgb(0xDFDFDF));
        assert_eq!(c.caption, TraceColor::rgb(0xA8A8A8));
        assert_eq!(c.icon_muted, TraceColor::rgb(0xC8C8C8));
        assert_eq!(c.accent, TraceColor::rgb(0xF5F5F5));
        assert_eq!(c.accent_strong, TraceColor::rgb(0xFFFFFF));
        assert_eq!(c.selected_surface, TraceColor::rgb(0xF5F5F5));
        assert_eq!(c.selected_text, TraceColor::rgb(0x101010));
        assert_eq!(c.editor_text, TraceColor::rgb(0xF5F5F5));
        assert_eq!(c.editor_placeholder, TraceColor::rgb(0xB0B0B0));
        assert_eq!(c.editor_insertion, TraceColor::rgb(0xFFFFFF));
    }

    #[test]
    fn paper_preset_capture_palette() {
        let c = TraceTheme::for_preset(ThemePreset::Paper).capture;
        assert_eq!(c.panel_background, TraceColor::rgb(0xF7F6F3));
        assert_eq!(c.chrome_background, TraceColor::rgb(0xFAF9F6));
        assert_eq!(c.surface, TraceColor::rgb(0xFFFCF7));
        assert_eq!(c.border, TraceColor::rgb(0xE7E1D8));
        assert_eq!(c.text_primary, TraceColor::rgb(0x191919));
        assert_eq!(c.text_secondary, TraceColor::rgb(0x4F4A45));
        assert_eq!(c.caption, TraceColor::rgb(0x7A746C));
        assert_eq!(c.icon_muted, TraceColor::rgb(0x68635D));
        assert_eq!(c.accent, TraceColor::rgb(0x383836));
        assert_eq!(c.accent_strong, TraceColor::rgb(0x191919));
        assert_eq!(c.selected_surface, TraceColor::rgb(0x191919));
        assert_eq!(c.selected_text, TraceColor::rgb(0xF0EFED));
        assert_eq!(c.editor_text, TraceColor::rgb(0x191919));
        assert_eq!(c.editor_placeholder, TraceColor::rgb(0x9E968E));
        assert_eq!(c.editor_insertion, TraceColor::rgb(0x191919));
    }

    #[test]
    fn dune_preset_capture_palette() {
        let c = TraceTheme::for_preset(ThemePreset::Dune).capture;
        assert_eq!(c.panel_background, TraceColor::rgb(0xF0EEE6));
        assert_eq!(c.chrome_background, TraceColor::rgb(0xF5F2EA));
        assert_eq!(c.surface, TraceColor::rgb(0xFFFBF3));
        assert_eq!(c.border, TraceColor::rgb(0xDED6C8));
        assert_eq!(c.text_primary, TraceColor::rgb(0x2F2D29));
        assert_eq!(c.text_secondary, TraceColor::rgb(0x5D5952));
        assert_eq!(c.caption, TraceColor::rgb(0x87837B));
        assert_eq!(c.icon_muted, TraceColor::rgb(0x706C65));
        assert_eq!(c.accent, TraceColor::rgb(0xD97757));
        assert_eq!(c.accent_strong, TraceColor::rgb(0xB85F3F));
        assert_eq!(c.selected_surface, TraceColor::rgb(0xB85F3F));
        assert_eq!(c.selected_text, TraceColor::white(1.0));
        assert_eq!(c.editor_text, TraceColor::rgb(0x2F2D29));
        assert_eq!(c.editor_placeholder, TraceColor::rgb(0xA6A197));
        assert_eq!(c.editor_insertion, TraceColor::rgb(0xD97757));
    }

    #[test]
    fn light_preset_settings_palette() {
        let s = TraceTheme::for_preset(ThemePreset::Light).settings;
        assert_eq!(s.shell_top, TraceColor::rgb(0xFBF9FF));
        assert_eq!(s.shell_middle, TraceColor::rgb(0xF1ECFA));
        assert_eq!(s.shell_bottom, TraceColor::rgb(0xE6DFF5));
        assert_eq!(s.shell_primary_glow, TraceColor::rgba(0xA079FF, 0.22));
        assert_eq!(s.shell_secondary_glow, TraceColor::rgba(0xF5B8D4, 0.18));
        assert_eq!(s.shell_panel, TraceColor::white(0.82));
        assert_eq!(s.shell_panel_border, TraceColor::rgba(0xD6CEE8, 0.8));
        assert_eq!(s.card_background, TraceColor::white(0.96));
        assert_eq!(s.card_border, TraceColor::rgb(0xDDD4EE));
        assert_eq!(s.card_shadow, TraceColor::black(0.08));
        assert_eq!(s.header_eyebrow, TraceColor::rgb(0x7E58D8));
        assert_eq!(s.header_title, TraceColor::rgb(0x1E1930));
        assert_eq!(s.header_subtitle, TraceColor::rgb(0x625B77));
        assert_eq!(s.section_title, TraceColor::rgb(0x211B31));
        assert_eq!(s.section_description, TraceColor::rgb(0x615875));
        assert_eq!(s.row_label, TraceColor::rgb(0x463E5B));
        assert_eq!(s.field_background, TraceColor::white(1.0));
        assert_eq!(s.field_border, TraceColor::rgb(0xD7CEE8));
        assert_eq!(s.field_text, TraceColor::rgb(0x201A2F));
        assert_eq!(s.chip_background, TraceColor::rgb(0xEFE8FA));
        assert_eq!(s.chip_text, TraceColor::rgb(0x4A3B74));
        assert_eq!(s.accent, TraceColor::rgb(0xA079FF));
        assert_eq!(s.accent_strong, TraceColor::rgb(0x6C31E3));
        assert_eq!(s.primary_button_text, TraceColor::white(1.0));
        assert_eq!(s.secondary_button_background, TraceColor::white(0.74));
        assert_eq!(s.secondary_button_border, TraceColor::rgba(0xD4CAE8, 0.92));
        assert_eq!(s.secondary_button_text, TraceColor::rgb(0x241D36));
        assert_eq!(s.muted_text, TraceColor::rgb(0x7B7390));
        assert_eq!(s.warning_text, TraceColor::rgb(0x6C31E3));
    }

    #[test]
    fn dark_preset_settings_palette() {
        let s = TraceTheme::for_preset(ThemePreset::Dark).settings;
        assert_eq!(s.shell_top, TraceColor::rgb(0x111111));
        assert_eq!(s.shell_middle, TraceColor::rgb(0x0C0C0C));
        assert_eq!(s.shell_bottom, TraceColor::rgb(0x080808));
        assert_eq!(s.shell_primary_glow, TraceColor::white(0.06));
        assert_eq!(s.shell_secondary_glow, TraceColor::white(0.03));
        assert_eq!(s.shell_panel, TraceColor::rgba(0x131313, 0.86));
        assert_eq!(s.shell_panel_border, TraceColor::rgba(0x2C2C2C, 0.96));
        assert_eq!(s.card_background, TraceColor::rgb(0x171717));
        assert_eq!(s.card_border, TraceColor::rgb(0x292929));
        assert_eq!(s.card_shadow, TraceColor::black(0.34));
        assert_eq!(s.header_eyebrow, TraceColor::rgb(0xD5D5D5));
        assert_eq!(s.header_title, TraceColor::rgb(0xF5F5F5));
        assert_eq!(s.header_subtitle, TraceColor::rgb(0xCFCFCF));
        assert_eq!(s.section_title, TraceColor::rgb(0xF0F0F0));
        assert_eq!(s.section_description, TraceColor::rgb(0xBABABA));
        assert_eq!(s.row_label, TraceColor::rgb(0xE2E2E2));
        assert_eq!(s.field_background, TraceColor::rgb(0x1A1A1A));
        assert_eq!(s.field_border, TraceColor::rgb(0x343434));
        assert_eq!(s.field_text, TraceColor::rgb(0xF5F5F5));
        assert_eq!(s.chip_background, TraceColor::rgb(0x202020));
        assert_eq!(s.chip_text, TraceColor::rgb(0xEEEEEE));
        assert_eq!(s.accent, TraceColor::rgb(0xF5F5F5));
        assert_eq!(s.accent_strong, TraceColor::rgb(0xFFFFFF));
        assert_eq!(s.primary_button_text, TraceColor::rgb(0x101010));
        assert_eq!(s.secondary_button_background, TraceColor::white(0.07));
        assert_eq!(s.secondary_button_border, TraceColor::rgba(0x3A3A3A, 0.96));
        assert_eq!(s.secondary_button_text, TraceColor::rgb(0xEFEFEF));
        assert_eq!(s.muted_text, TraceColor::rgb(0xA8A8A8));
        assert_eq!(s.warning_text, TraceColor::rgb(0xF5F5F5));
    }

    #[test]
    fn paper_preset_settings_palette() {
        let s = TraceTheme::for_preset(ThemePreset::Paper).settings;
        assert_eq!(s.shell_top, TraceColor::rgb(0xFCFBF8));
        assert_eq!(s.shell_middle, TraceColor::rgb(0xF4F1EA));
        assert_eq!(s.shell_bottom, TraceColor::rgb(0xECE7DE));
        assert_eq!(s.shell_primary_glow, TraceColor::rgba(0xD6CEC2, 0.24));
        assert_eq!(s.shell_secondary_glow, TraceColor::rgba(0xF0EEE6, 0.18));
        assert_eq!(s.shell_panel, TraceColor::white(0.78));
        assert_eq!(s.shell_panel_border, TraceColor::rgba(0xE0D9CF, 0.9));
        assert_eq!(s.card_background, TraceColor::rgb(0xFFFCF7));
        assert_eq!(s.card_border, TraceColor::rgb(0xE5DED5));
        assert_eq!(s.card_shadow, TraceColor::black(0.06));
        assert_eq!(s.header_eyebrow, TraceColor::rgb(0x383836));
        assert_eq!(s.header_title, TraceColor::rgb(0x1B1B1A));
        assert_eq!(s.header_subtitle, TraceColor::rgb(0x5D5852));
        assert_eq!(s.section_title, TraceColor::rgb(0x1C1B1A));
        assert_eq!(s.section_description, TraceColor::rgb(0x635D56));
        assert_eq!(s.row_label, TraceColor::rgb(0x3F3B36));
        assert_eq!(s.field_background, TraceColor::white(1.0));
        assert_eq!(s.field_border, TraceColor::rgb(0xE2DBD0));
        assert_eq!(s.field_text, TraceColor::rgb(0x1C1B1A));
        assert_eq!(s.chip_background, TraceColor::rgb(0xF3EEE6));
        assert_eq!(s.chip_text, TraceColor::rgb(0x3A3733));
        assert_eq!(s.accent, TraceColor::rgb(0x383836));
        assert_eq!(s.accent_strong, TraceColor::rgb(0x191919));
        assert_eq!(s.primary_button_text, TraceColor::white(1.0));
        assert_eq!(s.secondary_button_background, TraceColor::white(0.82));
        assert_eq!(s.secondary_button_border, TraceColor::rgba(0xDDD5CB, 0.96));
        assert_eq!(s.secondary_button_text, TraceColor::rgb(0x24211E));
        assert_eq!(s.muted_text, TraceColor::rgb(0x7E776F));
        assert_eq!(s.warning_text, TraceColor::rgb(0x191919));
    }

    #[test]
    fn dune_preset_settings_palette() {
        let s = TraceTheme::for_preset(ThemePreset::Dune).settings;
        assert_eq!(s.shell_top, TraceColor::rgb(0xF7F3E9));
        assert_eq!(s.shell_middle, TraceColor::rgb(0xEFE8DC));
        assert_eq!(s.shell_bottom, TraceColor::rgb(0xE5DCCD));
        assert_eq!(s.shell_primary_glow, TraceColor::rgba(0xD97757, 0.22));
        assert_eq!(s.shell_secondary_glow, TraceColor::rgba(0xE8E0D0, 0.18));
        assert_eq!(s.shell_panel, TraceColor::white(0.74));
        assert_eq!(s.shell_panel_border, TraceColor::rgba(0xDDD2C0, 0.9));
        assert_eq!(s.card_background, TraceColor::rgb(0xFFF9EF));
        assert_eq!(s.card_border, TraceColor::rgb(0xE3D8C7));
        assert_eq!(s.card_shadow, TraceColor::black(0.08));
        assert_eq!(s.header_eyebrow, TraceColor::rgb(0xC4694B));
        assert_eq!(s.header_title, TraceColor::rgb(0x2F2B25));
        assert_eq!(s.header_subtitle, TraceColor::rgb(0x6A645C));
        assert_eq!(s.section_title, TraceColor::rgb(0x302C27));
        assert_eq!(s.section_description, TraceColor::rgb(0x6C665F));
        assert_eq!(s.row_label, TraceColor::rgb(0x4C463F));
        assert_eq!(s.field_background, TraceColor::white(0.96));
        assert_eq!(s.field_border, TraceColor::rgb(0xE2D7C6));
        assert_eq!(s.field_text, TraceColor::rgb(0x302C27));
        assert_eq!(s.chip_background, TraceColor::rgb(0xF3E7D8));
        assert_eq!(s.chip_text, TraceColor::rgb(0x5B4D43));
        assert_eq!(s.accent, TraceColor::rgb(0xD97757));
        assert_eq!(s.accent_strong, TraceColor::rgb(0xB85F3F));
        assert_eq!(s.primary_button_text, TraceColor::white(1.0));
        assert_eq!(s.secondary_button_background, TraceColor::white(0.82));
        assert_eq!(s.secondary_button_border, TraceColor::rgba(0xDCCFBE, 0.95));
        assert_eq!(s.secondary_button_text, TraceColor::rgb(0x342F28));
        assert_eq!(s.muted_text, TraceColor::rgb(0x898277));
        assert_eq!(s.warning_text, TraceColor::rgb(0xB85F3F));
    }

    #[test]
    fn preview_swatches_per_preset() {
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Light).preview_swatches,
            [
                TraceColor::rgb(0xF8F7FC),
                TraceColor::rgb(0xA079FF),
                TraceColor::rgb(0x6C31E3),
                TraceColor::rgb(0x1D1A26),
            ]
        );
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Dark).preview_swatches,
            [
                TraceColor::rgb(0x101010),
                TraceColor::rgb(0x1A1A1A),
                TraceColor::rgb(0x7E7E7E),
                TraceColor::rgb(0xF5F5F5),
            ]
        );
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Paper).preview_swatches,
            [
                TraceColor::rgb(0xFFFCF7),
                TraceColor::rgb(0xF3EEE6),
                TraceColor::rgb(0x383836),
                TraceColor::rgb(0x191919),
            ]
        );
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Dune).preview_swatches,
            [
                TraceColor::rgb(0xFFF9EF),
                TraceColor::rgb(0xF0EEE6),
                TraceColor::rgb(0xD97757),
                TraceColor::rgb(0xB85F3F),
            ]
        );
    }

    #[test]
    fn default_preset_is_dark() {
        let default_theme = TraceTheme::for_preset(ThemePreset::default());
        let dark_theme = TraceTheme::for_preset(ThemePreset::Dark);
        assert_eq!(default_theme, dark_theme);
    }

    #[test]
    fn theme_preset_field_matches_input() {
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Light).preset,
            ThemePreset::Light
        );
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Dark).preset,
            ThemePreset::Dark
        );
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Paper).preset,
            ThemePreset::Paper
        );
        assert_eq!(
            TraceTheme::for_preset(ThemePreset::Dune).preset,
            ThemePreset::Dune
        );
    }

    #[test]
    fn for_preset_is_usable_in_const_context() {
        const DARK: TraceTheme = TraceTheme::for_preset(ThemePreset::Dark);
        assert_eq!(DARK, TraceTheme::for_preset(ThemePreset::Dark));
    }
}

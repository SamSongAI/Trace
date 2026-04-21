//! Bridge between [`trace_core::TraceTheme`] and the `iced` theme system.
//!
//! `iced::Theme::Custom` only exposes a five-slot [`iced::theme::Palette`]
//! (background / text / primary / success / danger), which cannot carry the
//! fifteen capture-panel roles defined in [`trace_core::CapturePalette`]. The
//! Phase 10 strategy is therefore:
//!
//! 1. [`to_iced_theme`] maps the *base* three roles (panel background, primary
//!    text, accent) onto the `iced::Theme::Custom` palette so built-in widget
//!    defaults (text color, container background) inherit Trace colors without
//!    extra plumbing.
//! 2. Per-widget style functions in this module convert the remaining
//!    `CapturePalette` fields into the matching `iced::widget::*::Style`
//!    structs. Callers pass these functions to `widget::button(...).style(...)`
//!    et al. in `view()`.
//!
//! The functions are pure — no filesystem, no environment — so they can be
//! unit-tested in isolation and called from `const`-friendly paths where iced
//! allows.

use iced::border::Border;
use iced::widget::{button, container, text_editor, text_input};
use iced::{Background, Color, Theme};
use trace_core::{CapturePalette, TraceColor, TraceTheme};

/// Converts a [`TraceColor`] (0-255 sRGB + f32 alpha) into an [`iced::Color`].
///
/// Const so callers can embed static palettes without runtime cost.
pub const fn trace_color_to_iced(color: TraceColor) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}

/// Converts a [`trace_core::TraceTheme`] into an [`iced::Theme::Custom`].
///
/// The iced palette's `background` is bound to [`CapturePalette::panel_background`],
/// `text` to [`CapturePalette::text_primary`], and `primary` to
/// [`CapturePalette::accent_strong`]. `success` and `danger` have no natural
/// counterpart in the capture palette so they are filled with `accent_strong`
/// and `text_secondary` respectively — they only surface when a widget explicitly
/// styles itself as success/danger, which the capture panel does not.
pub fn to_iced_theme(theme: &TraceTheme) -> Theme {
    let capture = theme.capture;
    let palette = iced::theme::Palette {
        background: trace_color_to_iced(capture.panel_background),
        text: trace_color_to_iced(capture.text_primary),
        primary: trace_color_to_iced(capture.accent_strong),
        success: trace_color_to_iced(capture.accent_strong),
        warning: trace_color_to_iced(capture.accent),
        danger: trace_color_to_iced(capture.text_secondary),
    };
    // `name` is only used for efficient change detection and debug display.
    // Routing through the preset enum means toggling presets also toggles
    // the underlying identity, keeping iced's internal caches correct.
    let name = match theme.preset {
        trace_core::ThemePreset::Light => "Trace Light",
        trace_core::ThemePreset::Dark => "Trace Dark",
        trace_core::ThemePreset::Paper => "Trace Paper",
        trace_core::ThemePreset::Dune => "Trace Dune",
    };
    Theme::custom(name, palette)
}

/// Returns a `container::StyleFn`-compatible closure painting a panel
/// background using [`CapturePalette::panel_background`] and the primary text
/// color.
pub fn panel_container_style(
    palette: CapturePalette,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default()
            .background(Background::Color(trace_color_to_iced(palette.panel_background)))
            .color(trace_color_to_iced(palette.text_primary))
    }
}

/// Returns a `container::StyleFn`-compatible closure painting a chrome
/// background (header/footer) using [`CapturePalette::chrome_background`] and
/// the muted icon color.
pub fn chrome_container_style(
    palette: CapturePalette,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default()
            .background(Background::Color(trace_color_to_iced(
                palette.chrome_background,
            )))
            .color(trace_color_to_iced(palette.text_primary))
    }
}

/// Returns a `container::StyleFn`-compatible closure painting a one-pixel
/// horizontal separator line using [`CapturePalette::border`].
pub fn separator_container_style(
    palette: CapturePalette,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default().background(Background::Color(trace_color_to_iced(
            palette.border,
        )))
    }
}

/// Builds a button style function for section/thread chips.
///
/// `selected` branches onto the `selected_surface` + `selected_text` colors;
/// the inactive state uses a translucent overlay over `surface` matching the
/// Swift `surface.opacity(0.6)` rule. The returned closure ignores the iced
/// button `Status` — Phase 10 keeps hover/press identical to active, and
/// Phase 11 will layer interactions on top.
pub fn chip_button_style(
    palette: CapturePalette,
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, _status: button::Status| {
        let (background, text_color) = if selected {
            (
                trace_color_to_iced(palette.selected_surface),
                trace_color_to_iced(palette.selected_text),
            )
        } else {
            // Mirror SwiftUI's `theme.surface.opacity(0.6)` — compose alpha by
            // scaling the surface color's opacity rather than mutating the
            // palette source.
            let mut surface = trace_color_to_iced(palette.surface);
            surface.a *= 0.6;
            (surface, trace_color_to_iced(palette.text_secondary))
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                radius: 7.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..button::Style::default()
        }
    }
}

/// Builds a transparent, icon-only button style for header buttons (pin /
/// settings). Text color branches on `active` so the pin button can go full
/// accent color when pinned.
pub fn header_icon_button_style(
    palette: CapturePalette,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, _status: button::Status| {
        let text_color = if active {
            trace_color_to_iced(palette.accent)
        } else {
            trace_color_to_iced(palette.icon_muted)
        };
        button::Style {
            background: None,
            text_color,
            border: Border::default(),
            ..button::Style::default()
        }
    }
}

/// Style for the document-mode title text input. Renders on the surface color
/// with editor text / placeholder / insertion colors drawn from the palette.
pub fn document_title_input_style(
    palette: CapturePalette,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme: &Theme, _status: text_input::Status| text_input::Style {
        background: Background::Color(trace_color_to_iced(palette.surface)),
        border: Border {
            radius: 7.0.into(),
            width: 1.0,
            color: trace_color_to_iced(palette.border),
        },
        icon: trace_color_to_iced(palette.icon_muted),
        placeholder: trace_color_to_iced(palette.editor_placeholder),
        value: trace_color_to_iced(palette.editor_text),
        selection: trace_color_to_iced(palette.accent),
    }
}

/// Style for the multi-line capture text editor. Background matches the panel
/// so it appears seamless with the shell; value/placeholder/insertion mirror
/// the Swift `CaptureTextEditor.Theme`.
pub fn capture_editor_style(
    palette: CapturePalette,
) -> impl Fn(&Theme, text_editor::Status) -> text_editor::Style {
    move |_theme: &Theme, _status: text_editor::Status| text_editor::Style {
        background: Background::Color(trace_color_to_iced(palette.panel_background)),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        placeholder: trace_color_to_iced(palette.editor_placeholder),
        value: trace_color_to_iced(palette.editor_text),
        selection: trace_color_to_iced(palette.accent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::ThemePreset;

    fn expected_iced_color(color: TraceColor) -> Color {
        Color::from_rgba8(color.r, color.g, color.b, color.a)
    }

    #[test]
    fn trace_color_conversion_preserves_channels() {
        let trace = TraceColor::rgb(0xF4F3F8);
        let iced = trace_color_to_iced(trace);
        let expected = Color::from_rgba8(0xF4, 0xF3, 0xF8, 1.0);
        assert!((iced.r - expected.r).abs() < f32::EPSILON);
        assert!((iced.g - expected.g).abs() < f32::EPSILON);
        assert!((iced.b - expected.b).abs() < f32::EPSILON);
        assert!((iced.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn trace_color_conversion_preserves_alpha() {
        let trace = TraceColor::rgba(0xA079FF, 0.22);
        let iced = trace_color_to_iced(trace);
        assert!((iced.a - 0.22).abs() < f32::EPSILON);
    }

    /// All four presets must round-trip their panel background through
    /// [`to_iced_theme`]. This is the core sanity check for the theme bridge.
    #[test]
    fn all_presets_map_background_to_iced_palette() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let trace_theme = TraceTheme::for_preset(preset);
            let iced_theme = to_iced_theme(&trace_theme);
            let palette = iced_theme.palette();
            let expected = expected_iced_color(trace_theme.capture.panel_background);
            assert_eq!(
                palette.background, expected,
                "preset {:?} background mismatch",
                preset
            );
        }
    }

    #[test]
    fn all_presets_map_text_primary_to_iced_palette_text() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let trace_theme = TraceTheme::for_preset(preset);
            let iced_theme = to_iced_theme(&trace_theme);
            let palette = iced_theme.palette();
            let expected = expected_iced_color(trace_theme.capture.text_primary);
            assert_eq!(palette.text, expected, "preset {:?} text mismatch", preset);
        }
    }

    #[test]
    fn all_presets_map_accent_strong_to_iced_palette_primary() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let trace_theme = TraceTheme::for_preset(preset);
            let iced_theme = to_iced_theme(&trace_theme);
            let palette = iced_theme.palette();
            let expected = expected_iced_color(trace_theme.capture.accent_strong);
            assert_eq!(palette.primary, expected);
        }
    }

    #[test]
    fn theme_names_are_preset_specific() {
        assert_eq!(
            to_iced_theme(&TraceTheme::for_preset(ThemePreset::Light)).to_string(),
            "Trace Light"
        );
        assert_eq!(
            to_iced_theme(&TraceTheme::for_preset(ThemePreset::Dark)).to_string(),
            "Trace Dark"
        );
        assert_eq!(
            to_iced_theme(&TraceTheme::for_preset(ThemePreset::Paper)).to_string(),
            "Trace Paper"
        );
        assert_eq!(
            to_iced_theme(&TraceTheme::for_preset(ThemePreset::Dune)).to_string(),
            "Trace Dune"
        );
    }

    #[test]
    fn panel_container_style_paints_panel_background() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let style_fn = panel_container_style(palette);
        let style = style_fn(&Theme::Dark);
        let expected = Background::Color(expected_iced_color(palette.panel_background));
        assert_eq!(style.background, Some(expected));
    }

    #[test]
    fn chrome_container_style_paints_chrome_background() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let style_fn = chrome_container_style(palette);
        let style = style_fn(&Theme::Light);
        let expected = Background::Color(expected_iced_color(palette.chrome_background));
        assert_eq!(style.background, Some(expected));
    }

    #[test]
    fn separator_container_style_paints_border_color() {
        let palette = TraceTheme::for_preset(ThemePreset::Paper).capture;
        let style_fn = separator_container_style(palette);
        let style = style_fn(&Theme::Light);
        let expected = Background::Color(expected_iced_color(palette.border));
        assert_eq!(style.background, Some(expected));
    }

    #[test]
    fn chip_button_style_selected_uses_selected_surface() {
        let palette = TraceTheme::for_preset(ThemePreset::Dune).capture;
        let style_fn = chip_button_style(palette, true);
        let style = style_fn(&Theme::Light, button::Status::Active);
        let expected_bg = Background::Color(expected_iced_color(palette.selected_surface));
        assert_eq!(style.background, Some(expected_bg));
        assert_eq!(
            style.text_color,
            expected_iced_color(palette.selected_text)
        );
    }

    #[test]
    fn chip_button_style_unselected_uses_text_secondary() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let style_fn = chip_button_style(palette, false);
        let style = style_fn(&Theme::Light, button::Status::Active);
        assert_eq!(
            style.text_color,
            expected_iced_color(palette.text_secondary)
        );
    }

    #[test]
    fn chip_button_style_has_rounded_corners() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let style_fn = chip_button_style(palette, false);
        let style = style_fn(&Theme::Light, button::Status::Active);
        // Corner radius uniformly 7pt per Mac reference.
        assert_eq!(style.border.radius.top_left, 7.0);
    }

    #[test]
    fn header_icon_button_active_uses_accent() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let style_fn = header_icon_button_style(palette, true);
        let style = style_fn(&Theme::Light, button::Status::Active);
        assert_eq!(style.text_color, expected_iced_color(palette.accent));
    }

    #[test]
    fn header_icon_button_inactive_uses_icon_muted() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let style_fn = header_icon_button_style(palette, false);
        let style = style_fn(&Theme::Light, button::Status::Active);
        assert_eq!(style.text_color, expected_iced_color(palette.icon_muted));
    }

    #[test]
    fn document_title_input_style_uses_surface_background() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let style_fn = document_title_input_style(palette);
        let style = style_fn(&Theme::Dark, text_input::Status::Active);
        let expected_bg = Background::Color(expected_iced_color(palette.surface));
        assert_eq!(style.background, expected_bg);
        assert_eq!(style.value, expected_iced_color(palette.editor_text));
        assert_eq!(
            style.placeholder,
            expected_iced_color(palette.editor_placeholder)
        );
    }

    #[test]
    fn capture_editor_style_uses_panel_background() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let style_fn = capture_editor_style(palette);
        let style = style_fn(&Theme::Dark, text_editor::Status::Active);
        let expected_bg = Background::Color(expected_iced_color(palette.panel_background));
        assert_eq!(style.background, expected_bg);
        assert_eq!(style.value, expected_iced_color(palette.editor_text));
    }
}

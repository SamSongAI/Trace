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
use trace_core::{CapturePalette, SettingsPalette, TraceColor, TraceTheme};

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

/// Container style for the toast pill. Uses the darker [`CapturePalette::chrome_background`]
/// so the overlay reads over any editor content, with an accent-tinted border
/// for contrast and the primary text color for the message glyphs.
pub fn toast_container_style(palette: CapturePalette) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default()
            .background(Background::Color(trace_color_to_iced(
                palette.chrome_background,
            )))
            .color(trace_color_to_iced(palette.text_primary))
            .border(Border {
                radius: 10.0.into(),
                width: 1.0,
                color: trace_color_to_iced(palette.border),
            })
    }
}

/// Outermost container for the settings window. Paints the whole surface with
/// [`SettingsPalette::shell_middle`] — the middle stop of the Mac reference's
/// vertical gradient. iced 0.14's stable container API can't render a
/// multi-stop gradient without widgets-overdraw tricks, so Phase 12 locks a
/// single mid-tone to keep contrast consistent across presets. The text color
/// is seeded from [`SettingsPalette::header_title`] so nested `text` widgets
/// that don't set their own color inherit a legible default.
pub fn settings_shell_style(palette: SettingsPalette) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default()
            .background(Background::Color(trace_color_to_iced(palette.shell_middle)))
            .color(trace_color_to_iced(palette.header_title))
    }
}

/// Container style for a settings card (the rounded rectangle that wraps a
/// related group of rows). Background is [`SettingsPalette::card_background`],
/// with a hairline border tuned by preset (`card_border`) and the Mac
/// reference's 16-pt corner radius.
pub fn card_container_style(palette: SettingsPalette) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default()
            .background(Background::Color(trace_color_to_iced(
                palette.card_background,
            )))
            .color(trace_color_to_iced(palette.section_title))
            .border(Border {
                radius: 16.0.into(),
                width: 1.0,
                color: trace_color_to_iced(palette.card_border),
            })
    }
}

/// Shared text-input style for every settings field (vault path, filename
/// template, hotkey display, …). Uses [`SettingsPalette::field_background`] /
/// `field_border` / `field_text` so the field reads as its own sunken surface
/// against the card. Placeholder and selection are tied to the palette's
/// `muted_text` and `accent` so light/dark presets stay balanced.
pub fn settings_field_style(
    palette: SettingsPalette,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme: &Theme, _status: text_input::Status| text_input::Style {
        background: Background::Color(trace_color_to_iced(palette.field_background)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: trace_color_to_iced(palette.field_border),
        },
        icon: trace_color_to_iced(palette.muted_text),
        placeholder: trace_color_to_iced(palette.muted_text),
        value: trace_color_to_iced(palette.field_text),
        selection: trace_color_to_iced(palette.accent),
    }
}

/// Primary (filled) button style used for affirmative actions in the settings
/// window (e.g. "Choose Folder"). Background tracks
/// [`SettingsPalette::accent_strong`], text tracks `primary_button_text`.
/// Hover/press states are flattened with the active state in Phase 12 — the
/// interaction polish lands in a later sub-task.
pub fn settings_primary_button_style(
    palette: SettingsPalette,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, _status: button::Status| button::Style {
        background: Some(Background::Color(trace_color_to_iced(
            palette.accent_strong,
        ))),
        text_color: trace_color_to_iced(palette.primary_button_text),
        border: Border {
            radius: 8.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..button::Style::default()
    }
}

/// Secondary (outlined) button style for low-emphasis actions — e.g. "Reset"
/// or "Cancel". Background uses [`SettingsPalette::secondary_button_background`]
/// with the matching `secondary_button_border` outline and text color.
pub fn settings_secondary_button_style(
    palette: SettingsPalette,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, _status: button::Status| button::Style {
        background: Some(Background::Color(trace_color_to_iced(
            palette.secondary_button_background,
        ))),
        text_color: trace_color_to_iced(palette.secondary_button_text),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: trace_color_to_iced(palette.secondary_button_border),
        },
        ..button::Style::default()
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

    #[test]
    fn settings_shell_style_paints_shell_middle() {
        // The outer shell uses `shell_middle` on every preset so a drift in
        // the palette (or in the closure) is caught immediately.
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let style_fn = settings_shell_style(palette);
            let style = style_fn(&Theme::Light);
            let expected = Background::Color(expected_iced_color(palette.shell_middle));
            assert_eq!(
                style.background,
                Some(expected),
                "preset {:?} shell background mismatch",
                preset
            );
        }
    }

    #[test]
    fn settings_shell_style_inherits_header_title_color() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).settings;
        let style_fn = settings_shell_style(palette);
        let style = style_fn(&Theme::Dark);
        assert_eq!(
            style.text_color,
            Some(expected_iced_color(palette.header_title))
        );
    }

    #[test]
    fn card_container_style_uses_card_background_and_border() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let style_fn = card_container_style(palette);
            let style = style_fn(&Theme::Light);
            let expected_bg = Background::Color(expected_iced_color(palette.card_background));
            assert_eq!(
                style.background,
                Some(expected_bg),
                "preset {:?} card background mismatch",
                preset
            );
            assert_eq!(
                style.border.color,
                expected_iced_color(palette.card_border),
                "preset {:?} card border mismatch",
                preset
            );
        }
    }

    #[test]
    fn card_container_style_uses_sixteen_point_radius() {
        // Phase 12 locks the 16-pt corner radius to match Mac
        // `SettingsView.swift`. A drift here would make cards look sharper
        // or softer than the reference.
        let palette = TraceTheme::for_preset(ThemePreset::Light).settings;
        let style_fn = card_container_style(palette);
        let style = style_fn(&Theme::Light);
        assert_eq!(style.border.radius.top_left, 16.0);
        assert_eq!(style.border.width, 1.0);
    }

    #[test]
    fn settings_field_style_uses_field_background_and_text() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let style_fn = settings_field_style(palette);
            let style = style_fn(&Theme::Light, text_input::Status::Active);
            let expected_bg = Background::Color(expected_iced_color(palette.field_background));
            assert_eq!(
                style.background, expected_bg,
                "preset {:?} field bg mismatch",
                preset
            );
            assert_eq!(
                style.value,
                expected_iced_color(palette.field_text),
                "preset {:?} field text mismatch",
                preset
            );
            assert_eq!(
                style.border.color,
                expected_iced_color(palette.field_border),
                "preset {:?} field border mismatch",
                preset
            );
        }
    }

    #[test]
    fn settings_field_style_uses_muted_placeholder_and_accent_selection() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).settings;
        let style_fn = settings_field_style(palette);
        let style = style_fn(&Theme::Light, text_input::Status::Active);
        assert_eq!(style.placeholder, expected_iced_color(palette.muted_text));
        assert_eq!(style.selection, expected_iced_color(palette.accent));
    }

    #[test]
    fn settings_primary_button_uses_accent_strong_background() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let style_fn = settings_primary_button_style(palette);
            let style = style_fn(&Theme::Light, button::Status::Active);
            let expected_bg = Background::Color(expected_iced_color(palette.accent_strong));
            assert_eq!(
                style.background,
                Some(expected_bg),
                "preset {:?} primary bg mismatch",
                preset
            );
            assert_eq!(
                style.text_color,
                expected_iced_color(palette.primary_button_text),
                "preset {:?} primary text mismatch",
                preset
            );
        }
    }

    #[test]
    fn settings_primary_button_has_no_border() {
        // The filled variant is borderless — width 0. Radius stays 8 pt.
        let palette = TraceTheme::for_preset(ThemePreset::Light).settings;
        let style_fn = settings_primary_button_style(palette);
        let style = style_fn(&Theme::Light, button::Status::Active);
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.radius.top_left, 8.0);
    }

    #[test]
    fn settings_secondary_button_uses_secondary_palette_slots() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let style_fn = settings_secondary_button_style(palette);
            let style = style_fn(&Theme::Light, button::Status::Active);
            let expected_bg =
                Background::Color(expected_iced_color(palette.secondary_button_background));
            assert_eq!(
                style.background,
                Some(expected_bg),
                "preset {:?} secondary bg mismatch",
                preset
            );
            assert_eq!(
                style.text_color,
                expected_iced_color(palette.secondary_button_text),
                "preset {:?} secondary text mismatch",
                preset
            );
            assert_eq!(
                style.border.color,
                expected_iced_color(palette.secondary_button_border),
                "preset {:?} secondary border mismatch",
                preset
            );
        }
    }

    #[test]
    fn settings_secondary_button_has_hairline_outline() {
        // Outlined variant: 1-pixel border, 8-pt radius.
        let palette = TraceTheme::for_preset(ThemePreset::Light).settings;
        let style_fn = settings_secondary_button_style(palette);
        let style = style_fn(&Theme::Light, button::Status::Active);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius.top_left, 8.0);
    }
}

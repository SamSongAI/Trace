//! Selection-tile widgets used inside settings cards (Phase 12 sub-task 3).
//!
//! The Mac reference (`SettingsView.swift`) draws three distinct tile shapes
//! inside the Language / Theme / Storage cards:
//!
//! | Shape | Used by | Layout |
//! | --- | --- | --- |
//! | [`language_chip`] | Language card | A pill-shaped button with a single label. |
//! | [`theme_preset_tile`] | Theme card | 28×28 glyph box, title on the left, 4 preview swatches on the right. |
//! | [`write_mode_tile`] | Storage → Write Mode row | 28×28 glyph box, compact title + destination caption, checkmark when selected. |
//!
//! Every factory is generic over the caller's message type (`Msg: Clone`) so
//! the Language card can hand in a `SettingsMessage::LanguageChanged`,
//! the Theme card can hand in `SettingsMessage::ThemePresetChanged`, and so
//! on — without this module depending on [`crate::settings::SettingsMessage`].
//!
//! # Why a separate module
//!
//! The sibling `widgets.rs` intentionally stays a "shell" module —
//! [`crate::settings::widgets::section_card`] / `setting_row` are thin
//! containers that carry no business knowledge of any specific card. The
//! tiles below, by contrast, embed palette-specific styling (accent border
//! when selected, icon box with its own background) and branching on
//! selection state. Keeping them next to each other but in separate files
//! lets the shell helpers stay reusable for cards that don't need a tile
//! (e.g. the Shortcuts card in later sub-tasks).
//!
//! # Layout reference
//!
//! Every tile shares:
//!
//! * [`TILE_PADDING_HORIZONTAL`] / [`TILE_PADDING_VERTICAL`] — the 12 / 10 pt
//!   inner padding from `SettingsView.swift` (`ThemePresetTile` / `WriteModeTile`).
//! * [`TILE_CORNER_RADIUS`] — 10 pt rounded rectangle.
//! * [`TILE_BORDER_WIDTH_SELECTED`] / [`TILE_BORDER_WIDTH_IDLE`] — 1.5 pt
//!   accent border when selected, 1.0 pt `card_border` otherwise.
//! * [`TILE_ICON_BOX_SIZE`] / [`TILE_ICON_BOX_RADIUS`] — 28×28 icon box with
//!   7 pt corners.
//!
//! Theme-specific and write-mode-specific metrics (glyph font size, swatch
//! spacing, checkmark size) are locked in nearby constants so tests can keep
//! them in sync with the Swift source.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Pixels, Theme};
use trace_core::{SettingsPalette, TraceColor};

use crate::theme::trace_color_to_iced;

/// Inner horizontal padding of every tile. Matches Mac
/// `ThemePresetTile` / `WriteModeTile` `.padding(.horizontal, 12)`.
pub const TILE_PADDING_HORIZONTAL: u16 = 12;
/// Inner vertical padding of every tile. Matches Mac
/// `.padding(.vertical, 10)`.
pub const TILE_PADDING_VERTICAL: u16 = 10;
/// Corner radius of every tile's rounded rectangle background. Matches Mac
/// `RoundedRectangle(cornerRadius: 10)`.
pub const TILE_CORNER_RADIUS: f32 = 10.0;
/// Border width when the tile is selected. Mac uses a slightly thicker accent
/// outline to make the selected state read at a glance.
pub const TILE_BORDER_WIDTH_SELECTED: f32 = 1.5;
/// Border width when the tile is idle.
pub const TILE_BORDER_WIDTH_IDLE: f32 = 1.0;
/// Pixel size of the 28×28 icon swatch drawn on the left of each tile.
pub const TILE_ICON_BOX_SIZE: f32 = 28.0;
/// Corner radius of the icon swatch. Matches Mac's `RoundedRectangle(cornerRadius: 7)`.
pub const TILE_ICON_BOX_RADIUS: f32 = 7.0;
/// Horizontal spacing between the icon swatch and the adjacent label column.
pub const TILE_ICON_BOX_SPACING: f32 = 10.0;
/// Font size of the glyph inside the icon box on a theme-preset tile. Slightly
/// larger than the write-mode glyph because the theme tile uses sharper iconography.
pub const THEME_TILE_ICON_FONT_SIZE: f32 = 13.0;
/// Font size of the glyph inside the icon box on a write-mode tile.
pub const WRITE_MODE_TILE_ICON_FONT_SIZE: f32 = 12.0;
/// Font size of the tile title (bold line). Matches Mac `.font(.system(size: 13, weight: .semibold))`.
pub const TILE_TITLE_FONT_SIZE: f32 = 13.0;
/// Font size of the destination caption under a write-mode tile's title.
pub const TILE_SUBTITLE_FONT_SIZE: f32 = 11.5;
/// Spacing between the title line and the destination caption inside a tile.
pub const TILE_TITLE_SUBTITLE_SPACING: f32 = 2.0;
/// Font size of the checkmark glyph shown on the selected write-mode tile.
pub const TILE_CHECKMARK_FONT_SIZE: f32 = 14.0;
/// Side length of a preview-swatch square inside the theme-preset tile. Matches
/// Mac `previewSwatches` sized at 12×12.
pub const THEME_TILE_PREVIEW_SWATCH_SIZE: f32 = 12.0;
/// Spacing between preview-swatch squares.
pub const THEME_TILE_PREVIEW_SWATCH_SPACING: f32 = 4.0;
/// Corner radius of each preview-swatch square.
pub const THEME_TILE_PREVIEW_SWATCH_RADIUS: f32 = 3.0;
/// Font size of the language chip label. Chosen to match the 13 pt bold row
/// label Mac `AppLanguage.segment` uses.
pub const LANGUAGE_CHIP_FONT_SIZE: f32 = 13.0;
/// Corner radius of the language chip's pill background.
pub const LANGUAGE_CHIP_CORNER_RADIUS: f32 = 8.0;
/// Inner horizontal padding of the language chip. A hair wider than the
/// generic tile padding so the pill shape reads as a chip rather than a tile.
pub const LANGUAGE_CHIP_PADDING_HORIZONTAL: u16 = 14;
/// Inner vertical padding of the language chip.
pub const LANGUAGE_CHIP_PADDING_VERTICAL: u16 = 8;

/// Builds a pill-shaped language chip.
///
/// `selected` branches onto the accent background (`accent_strong`) with the
/// primary-button text color; the idle state uses `chip_background` +
/// `chip_text`. Matches Mac `SettingsView.swift`'s `AppLanguage` segment
/// (lines 273-302).
///
/// The caller hands in a concrete message value — not a closure — so the chip
/// can stay zero-cost at construction time. The common call-site will pass
/// `SettingsMessage::LanguageChanged(language)`.
pub fn language_chip<'a, Msg>(
    palette: SettingsPalette,
    label: &'a str,
    selected: bool,
    on_press: Msg,
) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let (background, text_color) = if selected {
        (palette.accent_strong, palette.primary_button_text)
    } else {
        (palette.chip_background, palette.chip_text)
    };

    let label_widget = text(label)
        .size(Pixels(LANGUAGE_CHIP_FONT_SIZE))
        .color(trace_color_to_iced(text_color));

    button(label_widget)
        .padding([LANGUAGE_CHIP_PADDING_VERTICAL, LANGUAGE_CHIP_PADDING_HORIZONTAL])
        .on_press(on_press)
        .style(move |_theme: &Theme, _status: button::Status| button::Style {
            background: Some(Background::Color(trace_color_to_iced(background))),
            text_color: trace_color_to_iced(text_color),
            border: Border {
                radius: LANGUAGE_CHIP_CORNER_RADIUS.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..button::Style::default()
        })
        .into()
}

/// Builds a theme-preset tile: 28×28 icon box + English preset title + 4 preview swatches.
///
/// Mirrors Mac `ThemePresetTile` (lines 141-192 in `SettingsView.swift`). The
/// swatches are the Swift `previewSwatches` in declaration order — the caller
/// passes `TraceTheme::for_preset(preset).preview_swatches` unchanged.
///
/// Selected tiles draw a 1.5 pt accent border; idle tiles draw a 1 pt
/// `card_border` outline. Both states share the `chip_background` fill so the
/// tile reads as a distinct surface against the card.
pub fn theme_preset_tile<'a, Msg>(
    palette: SettingsPalette,
    title: &'a str,
    icon_glyph: &'a str,
    preview_swatches: [TraceColor; 4],
    selected: bool,
    on_press: Msg,
) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let icon_box = build_icon_box(palette, icon_glyph, THEME_TILE_ICON_FONT_SIZE);

    let title_widget = text(title)
        .size(Pixels(TILE_TITLE_FONT_SIZE))
        .color(trace_color_to_iced(palette.row_label));

    let mut swatch_row = row![].spacing(THEME_TILE_PREVIEW_SWATCH_SPACING);
    for swatch in preview_swatches {
        swatch_row = swatch_row.push(preview_swatch(swatch));
    }

    let body = row![
        icon_box,
        title_widget,
        Space::new().width(Length::Fill),
        swatch_row
    ]
    .spacing(TILE_ICON_BOX_SPACING)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    tile_button(palette, body.into(), selected, on_press)
}

/// Builds a write-mode tile: 28×28 icon box + compact title + destination caption
/// + trailing checkmark when selected.
///
/// Mirrors Mac `WriteModeTile` (lines 196-243 in `SettingsView.swift`).
pub fn write_mode_tile<'a, Msg>(
    palette: SettingsPalette,
    compact_title: &'a str,
    destination_title: &'a str,
    icon_glyph: &'a str,
    selected: bool,
    on_press: Msg,
) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let icon_box = build_icon_box(palette, icon_glyph, WRITE_MODE_TILE_ICON_FONT_SIZE);

    let title_widget = text(compact_title)
        .size(Pixels(TILE_TITLE_FONT_SIZE))
        .color(trace_color_to_iced(palette.row_label));
    let subtitle_widget = text(destination_title)
        .size(Pixels(TILE_SUBTITLE_FONT_SIZE))
        .color(trace_color_to_iced(palette.muted_text));

    let label_column = column![title_widget, subtitle_widget]
        .spacing(TILE_TITLE_SUBTITLE_SPACING)
        .width(Length::Fill);

    // Reserve the trailing checkmark slot even when idle so tiles don't
    // jitter horizontally as selection flips. Idle state draws a zero-size
    // space; selected state draws a single-glyph `text` widget.
    let trailing: Element<'a, Msg> = if selected {
        text("\u{2713}")
            .size(Pixels(TILE_CHECKMARK_FONT_SIZE))
            .color(trace_color_to_iced(palette.accent_strong))
            .into()
    } else {
        Space::new()
            .width(Length::Fixed(TILE_CHECKMARK_FONT_SIZE))
            .height(Length::Fixed(TILE_CHECKMARK_FONT_SIZE))
            .into()
    };

    let body = row![icon_box, label_column, trailing]
        .spacing(TILE_ICON_BOX_SPACING)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    tile_button(palette, body.into(), selected, on_press)
}

/// Internal helper that wraps an already-composed body row in the shared tile
/// button chrome (rounded rectangle, accent border on selected, on-press).
fn tile_button<'a, Msg>(
    palette: SettingsPalette,
    body: Element<'a, Msg>,
    selected: bool,
    on_press: Msg,
) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let background = palette.chip_background;
    let border_color = if selected {
        palette.accent_strong
    } else {
        palette.card_border
    };
    let border_width = if selected {
        TILE_BORDER_WIDTH_SELECTED
    } else {
        TILE_BORDER_WIDTH_IDLE
    };

    // Wrap the body in an inner container so the background/border styling
    // sits on the button's own surface. iced buttons render their border via
    // the button style function, which keeps hover/press states easy to layer
    // in later sub-tasks without reshaping the widget graph.
    button(
        container(body)
            .padding([TILE_PADDING_VERTICAL, TILE_PADDING_HORIZONTAL])
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(0)
    .on_press(on_press)
    .style(move |_theme: &Theme, _status: button::Status| button::Style {
        background: Some(Background::Color(trace_color_to_iced(background))),
        text_color: trace_color_to_iced(palette.row_label),
        border: Border {
            radius: TILE_CORNER_RADIUS.into(),
            width: border_width,
            color: trace_color_to_iced(border_color),
        },
        ..button::Style::default()
    })
    .into()
}

/// Builds the 28×28 icon-glyph box that sits on the left of every tile.
fn build_icon_box<'a, Msg>(
    palette: SettingsPalette,
    glyph: &'a str,
    font_size: f32,
) -> Element<'a, Msg>
where
    Msg: 'a,
{
    let glyph_widget = text(glyph)
        .size(Pixels(font_size))
        .color(trace_color_to_iced(palette.accent_strong))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    container(glyph_widget)
        .width(Length::Fixed(TILE_ICON_BOX_SIZE))
        .height(Length::Fixed(TILE_ICON_BOX_SIZE))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(trace_color_to_iced(palette.field_background))),
            border: Border {
                radius: TILE_ICON_BOX_RADIUS.into(),
                width: 1.0,
                color: trace_color_to_iced(palette.field_border),
            },
            ..container::Style::default()
        })
        .into()
}

/// Builds a single preview-swatch square used in the theme-preset tile.
fn preview_swatch<'a, Msg>(color: TraceColor) -> Element<'a, Msg>
where
    Msg: 'a,
{
    container(Space::new())
        .width(Length::Fixed(THEME_TILE_PREVIEW_SWATCH_SIZE))
        .height(Length::Fixed(THEME_TILE_PREVIEW_SWATCH_SIZE))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(trace_color_to_iced(color))),
            border: Border {
                radius: THEME_TILE_PREVIEW_SWATCH_RADIUS.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme, WriteMode};

    /// Distinct message type so the tests prove the factories are generic over
    /// the caller's enum, not coupled to [`crate::settings::SettingsMessage`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestMsg {
        Language(&'static str),
        Preset(ThemePreset),
        Mode(WriteMode),
    }

    fn all_presets() -> [ThemePreset; 4] {
        [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ]
    }

    #[test]
    fn language_chip_builds_in_both_states_across_all_presets() {
        for preset in all_presets() {
            let palette = TraceTheme::for_preset(preset).settings;
            let _idle: Element<'_, TestMsg> =
                language_chip(palette, "English", false, TestMsg::Language("en"));
            let _selected: Element<'_, TestMsg> =
                language_chip(palette, "中文", true, TestMsg::Language("zh"));
        }
    }

    #[test]
    fn language_chip_accepts_long_label() {
        // The "System default" option is the longest label rendered in the
        // language row — make sure the chip factory accepts it without
        // panicking on construction.
        let palette = TraceTheme::for_preset(ThemePreset::Dark).settings;
        let _chip: Element<'_, TestMsg> = language_chip(
            palette,
            "System default",
            false,
            TestMsg::Language("sys"),
        );
    }

    #[test]
    fn theme_preset_tile_builds_in_both_states_across_all_presets() {
        for preset in all_presets() {
            let theme = TraceTheme::for_preset(preset);
            let _idle: Element<'_, TestMsg> = theme_preset_tile(
                theme.settings,
                preset.title(),
                preset.icon_glyph(),
                theme.preview_swatches,
                false,
                TestMsg::Preset(preset),
            );
            let _selected: Element<'_, TestMsg> = theme_preset_tile(
                theme.settings,
                preset.title(),
                preset.icon_glyph(),
                theme.preview_swatches,
                true,
                TestMsg::Preset(preset),
            );
        }
    }

    #[test]
    fn write_mode_tile_builds_in_both_states_across_all_presets() {
        // Four presets × three write modes × two selection states — ensures no
        // combination panics during construction.
        for preset in all_presets() {
            let palette = TraceTheme::for_preset(preset).settings;
            for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
                let compact = mode.compact_title(trace_core::Language::En);
                let destination = mode.destination_title(trace_core::Language::En);
                let _idle: Element<'_, TestMsg> = write_mode_tile(
                    palette,
                    compact,
                    destination,
                    mode.icon_glyph(),
                    false,
                    TestMsg::Mode(mode),
                );
                let _selected: Element<'_, TestMsg> = write_mode_tile(
                    palette,
                    compact,
                    destination,
                    mode.icon_glyph(),
                    true,
                    TestMsg::Mode(mode),
                );
            }
        }
    }

    #[test]
    fn tile_metric_constants_match_mac_reference() {
        // Locked values from `SettingsView.swift`:
        //   - ThemePresetTile / WriteModeTile padding .horizontal 12 / .vertical 10
        //   - RoundedRectangle(cornerRadius: 10)
        //   - selected stroke 1.5, idle stroke 1.0
        //   - icon box 28×28, RoundedRectangle(cornerRadius: 7)
        assert_eq!(TILE_PADDING_HORIZONTAL, 12);
        assert_eq!(TILE_PADDING_VERTICAL, 10);
        assert_eq!(TILE_CORNER_RADIUS, 10.0);
        assert_eq!(TILE_BORDER_WIDTH_SELECTED, 1.5);
        assert_eq!(TILE_BORDER_WIDTH_IDLE, 1.0);
        assert_eq!(TILE_ICON_BOX_SIZE, 28.0);
        assert_eq!(TILE_ICON_BOX_RADIUS, 7.0);
    }

    #[test]
    fn theme_tile_preview_swatch_constants_match_mac_reference() {
        // Swift draws each preview swatch at 12×12 with 4 pt spacing.
        assert_eq!(THEME_TILE_PREVIEW_SWATCH_SIZE, 12.0);
        assert_eq!(THEME_TILE_PREVIEW_SWATCH_SPACING, 4.0);
        assert_eq!(THEME_TILE_PREVIEW_SWATCH_RADIUS, 3.0);
    }

    #[test]
    fn tile_title_font_sizes_are_stable() {
        assert_eq!(TILE_TITLE_FONT_SIZE, 13.0);
        assert_eq!(TILE_SUBTITLE_FONT_SIZE, 11.5);
        assert_eq!(TILE_CHECKMARK_FONT_SIZE, 14.0);
        assert_eq!(THEME_TILE_ICON_FONT_SIZE, 13.0);
        assert_eq!(WRITE_MODE_TILE_ICON_FONT_SIZE, 12.0);
    }
}

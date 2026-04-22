//! Shortcuts card: renders the four configurable keyboard shortcuts plus the
//! three fixed "informational" rows. Mirrors Mac `SettingsView.swift:504-525`.
//!
//! # Layout
//!
//! ```text
//! [category]                 [chip]                [Edit / Cancel]
//!  Name
//! ```
//!
//! Four rows render the configurable shortcuts (`Create`, `Send`, `Append`,
//! `ToggleMode`) each with an Edit button that emits
//! [`SettingsMessage::RecordingStarted`] to arm the recorder. While recording,
//! the chip reads "Recording..." (localized) and the trailing button becomes a
//! Cancel action. A 1-pt horizontal divider separates the configurable rows
//! from three fixed rows that describe panel-level reserved shortcuts (Esc /
//! ⌘P / ⌘1–9). A footer [`text`] surfaces the last validation error when the
//! recorder flagged one.
//!
//! # Shadow-only contract
//!
//! The card reads exclusively from `SettingsApp` shadow fields
//! (`global_hotkey` / `send_note_shortcut` / `append_note_shortcut` /
//! `mode_toggle_shortcut` / `recording_target` / `shortcut_recorder_message`).
//! The write-back to `Arc<AppSettings>` is deferred to sub-task 8 — the same
//! contract every other card in this module follows.

use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Background, Element, Font, Length, Padding, Pixels};
use trace_core::{L10n, Language, SettingsPalette, ShortcutSpec};

use crate::theme::trace_color_to_iced;

use super::widgets::section_card;
use super::{SettingsApp, SettingsMessage, ShortcutTarget};

// --- Layout constants ------------------------------------------------------
//
// 每一条都与 Mac `SettingsView.swift:564-626` 对齐。常量抽出来是为了让回归
// 测试能在编译期锁定这些数字,避免未来打磨时悄悄漂走。

/// Fixed width of the leading label column (category caption + target name).
/// Mac `.frame(width: 100, alignment: .leading)`.
const LABEL_COLUMN_WIDTH: f32 = 100.0;
/// Vertical padding inside a configurable shortcut row. Mac `.padding(.vertical, 8)`.
const ROW_VERTICAL_PADDING: u16 = 8;
/// Vertical padding inside a fixed informational row. Mac combines
/// `.padding(.vertical, 4)` and `.padding(.vertical, 3)` at 7 pt total;
/// we keep it as a single 4 pt padding on the Windows side to stay close
/// without duplicating the Swift double-padding quirk.
const FIXED_ROW_VERTICAL_PADDING: u16 = 4;
/// Category caption font size. Mac `.font(.system(size: 9, weight: .medium))`.
const CATEGORY_FONT_SIZE: f32 = 9.0;
/// Target name font size. Mac `.font(.system(size: 12, weight: .medium))`.
const TARGET_NAME_FONT_SIZE: f32 = 12.0;
/// Chip label font size. Mac `.font(.system(size: 12, weight: .semibold, design: .monospaced))`.
const SHORTCUT_CHIP_FONT_SIZE: f32 = 12.0;
/// Edit / Cancel button font size. Mac `.font(.system(size: 11, weight: .medium))`.
const EDIT_BUTTON_FONT_SIZE: f32 = 11.0;
/// Fixed row label + chip font size. Same as the configurable row so the two
/// blocks read as one card body.
const FIXED_ROW_LABEL_FONT_SIZE: f32 = 12.0;
/// Horizontal padding inside the chip. Mac `.padding(.horizontal, 8)`.
const CHIP_HORIZONTAL_PADDING: u16 = 8;
/// Vertical padding inside the chip. Mac `.padding(.vertical, 4)`.
const CHIP_VERTICAL_PADDING: u16 = 4;
/// Corner radius of the chip rounded rect. Mac `cornerRadius: 6`.
const CHIP_CORNER_RADIUS: f32 = 6.0;
/// Vertical gap between the category caption and the target name inside the
/// label column. Mac `VStack(spacing: 2)`.
const LABEL_COLUMN_SPACING: f32 = 2.0;
/// Footer font size for the recorder error message. Mac `.font(.system(size: 11, weight: .medium))`.
const ERROR_MESSAGE_FONT_SIZE: f32 = 11.0;
/// Vertical padding above the footer message. Mac `.padding(.top, 6)`.
const ERROR_MESSAGE_TOP_PADDING: u16 = 6;
/// Height of the divider rule. iced 0.14 has no `Divider` widget; we render a
/// 1-pt filled `Space` + colored container instead.
const DIVIDER_HEIGHT: f32 = 1.0;
/// Vertical padding flanking the divider. Mac `.padding(.vertical, 6)` on
/// `Divider()`.
const DIVIDER_VERTICAL_SPACING: u16 = 6;

/// Fixed-row chip label for the panel-section-switch shortcut. The `–` glyph
/// is U+2013 (EN DASH), matching the Mac reference's `⌘1–9` spelling. The
/// escape form keeps the character unambiguous when reading the source with
/// a dash-collapsing editor setup.
const FIXED_LABEL_SECTION_SWITCH: &str = "Ctrl+1\u{2013}9";

/// Monospaced font used for the shortcut chip. Matches the Mac reference's
/// `design: .monospaced` so ⌘/Ctrl glyphs align in a vertical stack.
fn chip_font() -> Font {
    Font {
        weight: Weight::Semibold,
        ..Font::MONOSPACE
    }
}

/// Medium-weight font used by the category caption and target name columns.
fn label_font() -> Font {
    Font {
        weight: Weight::Medium,
        ..Font::DEFAULT
    }
}

/// Builds the Shortcuts card element.
///
/// The card surfaces unconditionally under every write mode — shortcut
/// configuration is not scoped to a specific Storage layout. Called by
/// [`super::build_cards`] after the write-mode–specific branches.
pub(super) fn shortcuts_card<'a>(
    state: &'a SettingsApp,
    palette: SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;

    let mut body: Vec<Element<'a, SettingsMessage>> = Vec::with_capacity(9);

    for target in ShortcutTarget::ALL {
        body.push(configurable_row(state, palette, lang, target));
    }

    body.push(divider_rule(palette));

    // Fixed informational rows. Labels are derived in iced at paint time
    // from the L10n catalog so every language gets the same affordance.
    body.push(fixed_row(
        palette,
        lang,
        "Esc",
        L10n::shortcut_close_panel(lang),
    ));
    body.push(fixed_row(
        palette,
        lang,
        "Ctrl+P",
        L10n::shortcut_pin_panel(lang),
    ));
    body.push(fixed_row(
        palette,
        lang,
        FIXED_LABEL_SECTION_SWITCH,
        L10n::shortcut_switch_section(lang),
    ));

    // Footer: only rendered when a validation error is stored on the shadow.
    if let Some(message) = state.shortcut_recorder_message.as_deref() {
        body.push(
            container(
                text(message)
                    .size(Pixels(ERROR_MESSAGE_FONT_SIZE))
                    .color(trace_color_to_iced(palette.warning_text))
                    .font(label_font()),
            )
            .padding(Padding {
                top: ERROR_MESSAGE_TOP_PADDING as f32,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            })
            .width(Length::Fill)
            .into(),
        );
    }

    let stack = column(body).width(Length::Fill);

    section_card(palette, L10n::shortcuts(lang), stack.into())
}

/// Builds one configurable shortcut row. Reads the matching shadow
/// [`ShortcutSpec`] from `state` and branches on `recording_target` to swap
/// the chip text and trailing button appearance.
fn configurable_row<'a>(
    state: &'a SettingsApp,
    palette: SettingsPalette,
    lang: Language,
    target: ShortcutTarget,
) -> Element<'a, SettingsMessage> {
    let is_recording = state.recording_target == Some(target);
    let shortcut = state.shortcut_for(target);

    let category_label = text(target.category(lang))
        .size(Pixels(CATEGORY_FONT_SIZE))
        .color(trace_color_to_iced(palette.muted_text))
        .font(label_font());

    let target_label = text(target.name(lang))
        .size(Pixels(TARGET_NAME_FONT_SIZE))
        .color(trace_color_to_iced(palette.section_title))
        .font(label_font());

    let label_column = column![category_label, target_label]
        .spacing(LABEL_COLUMN_SPACING)
        .width(Length::Fixed(LABEL_COLUMN_WIDTH));

    let chip_text = if is_recording {
        L10n::recording(lang).to_string()
    } else {
        shortcut_display_label(shortcut)
    };

    let chip = shortcut_chip(palette, chip_text, is_recording);

    // Trailing button: the Edit variant arms recording; the Cancel variant
    // disarms it. We send the discriminated message straight from this row
    // so the card does not need to know about the target.
    let action: Element<'a, SettingsMessage> = if is_recording {
        button(
            text(L10n::cancel(lang))
                .size(Pixels(EDIT_BUTTON_FONT_SIZE))
                .color(trace_color_to_iced(palette.muted_text))
                .font(label_font()),
        )
        .on_press(SettingsMessage::RecordingCancelled)
        .style(plain_text_button_style())
        .padding(0)
        .into()
    } else {
        button(
            text(L10n::edit(lang))
                .size(Pixels(EDIT_BUTTON_FONT_SIZE))
                .color(trace_color_to_iced(palette.accent))
                .font(label_font()),
        )
        .on_press(SettingsMessage::RecordingStarted(target))
        .style(plain_text_button_style())
        .padding(0)
        .into()
    };

    let row_content = row![label_column, chip, Space::new().width(Length::Fill), action]
        .align_y(Vertical::Center)
        .width(Length::Fill);

    container(row_content)
        .padding(Padding {
            top: ROW_VERTICAL_PADDING as f32,
            right: 0.0,
            bottom: ROW_VERTICAL_PADDING as f32,
            left: 0.0,
        })
        .width(Length::Fill)
        .into()
}

/// Builds one fixed informational row (Esc / Ctrl+P / Ctrl+1-9).
fn fixed_row<'a>(
    palette: SettingsPalette,
    _lang: Language,
    chip_label: &'a str,
    body_label: &'a str,
) -> Element<'a, SettingsMessage> {
    let body = text(body_label)
        .size(Pixels(FIXED_ROW_LABEL_FONT_SIZE))
        .color(trace_color_to_iced(palette.muted_text))
        .font(label_font());

    let chip = text(chip_label)
        .size(Pixels(FIXED_ROW_LABEL_FONT_SIZE))
        .color(trace_color_to_iced(dim_color(palette.muted_text)))
        .font(chip_font());

    // Match the configurable-row column widths so the two blocks visually
    // align. The Mac reference uses the same 100 pt leading column.
    let row_content = row![
        container(body).width(Length::Fixed(LABEL_COLUMN_WIDTH)),
        container(chip).padding(chip_padding()),
        Space::new().width(Length::Fill),
    ]
    .align_y(Vertical::Center)
    .width(Length::Fill);

    container(row_content)
        .padding(Padding {
            top: FIXED_ROW_VERTICAL_PADDING as f32,
            right: 0.0,
            bottom: FIXED_ROW_VERTICAL_PADDING as f32,
            left: 0.0,
        })
        .width(Length::Fill)
        .into()
}

/// Padding wrapper for the chip background box. Extracted so both the
/// configurable-row and fixed-row chips share one definition.
fn chip_padding() -> Padding {
    Padding {
        top: CHIP_VERTICAL_PADDING as f32,
        right: CHIP_HORIZONTAL_PADDING as f32,
        bottom: CHIP_VERTICAL_PADDING as f32,
        left: CHIP_HORIZONTAL_PADDING as f32,
    }
}

/// Renders the shortcut chip (rounded background + monospaced label).
///
/// Uses [`SettingsPalette::chip_background`] at rest and an alpha-tinted
/// accent background while recording, matching Mac
/// `palette.accent.opacity(0.12)`. The recording text itself still reads in
/// `accent` so the chip is unambiguous.
///
/// Takes `label` by value so the caller can hand in a freshly built
/// `format!(..)` without needing to keep it alive on the stack — the view
/// tree owns its copy once the chip is built.
fn shortcut_chip<'a>(
    palette: SettingsPalette,
    label: String,
    is_recording: bool,
) -> Element<'a, SettingsMessage> {
    let (text_color, background) = if is_recording {
        (palette.accent, recording_chip_background(palette))
    } else {
        (palette.chip_text, palette.chip_background)
    };

    let chip_label = text(label)
        .size(Pixels(SHORTCUT_CHIP_FONT_SIZE))
        .color(trace_color_to_iced(text_color))
        .font(chip_font());

    container(chip_label)
        .padding(chip_padding())
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(trace_color_to_iced(background))),
            border: iced::border::Border {
                radius: CHIP_CORNER_RADIUS.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

/// Flat style for the Edit / Cancel buttons. iced 0.14 paints a default
/// surface fill + border on `button`; we neutralize both so the button reads
/// as inline text — matching Mac `.buttonStyle(.plain)`.
fn plain_text_button_style(
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |_theme: &iced::Theme, _status: iced::widget::button::Status| {
        iced::widget::button::Style {
            background: None,
            text_color: iced::Color::BLACK, // overridden by inner text(..).color(..)
            border: iced::border::Border {
                radius: 0.0.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            ..iced::widget::button::Style::default()
        }
    }
}

/// Computes the recording chip background: accent with the alpha reduced so
/// the chip looks tinted rather than solid. Mirrors Mac
/// `palette.accent.opacity(0.12)` using `TraceColor`'s public fields.
fn recording_chip_background(palette: SettingsPalette) -> trace_core::TraceColor {
    trace_core::TraceColor {
        r: palette.accent.r,
        g: palette.accent.g,
        b: palette.accent.b,
        a: 0.12,
    }
}

/// Dims a color by scaling its alpha channel down. Used by the fixed-row chip
/// text so the three info rows read as secondary compared to the configurable
/// rows. Mirrors Mac `palette.mutedText.opacity(0.7)`.
fn dim_color(color: trace_core::TraceColor) -> trace_core::TraceColor {
    trace_core::TraceColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a * 0.7,
    }
}

/// Formats a [`ShortcutSpec`] for the chip label. Delegates to
/// [`ShortcutSpec::display_label`] via `trace_core`'s public surface; kept as
/// a thin wrapper so tests in this module can pin the display format without
/// round-tripping through the full row.
fn shortcut_display_label(spec: ShortcutSpec) -> String {
    spec.display_label()
}

/// 1-pt horizontal rule between the configurable rows and the fixed rows.
/// Mac uses `Divider().overlay(palette.mutedText.opacity(0.15))`; the
/// Windows port renders the same visual shape with a thin tinted container.
fn divider_rule<'a>(palette: SettingsPalette) -> Element<'a, SettingsMessage> {
    let tinted_color = trace_core::TraceColor {
        r: palette.muted_text.r,
        g: palette.muted_text.g,
        b: palette.muted_text.b,
        a: palette.muted_text.a * 0.15,
    };

    let rule = container(Space::new().width(Length::Fill))
        .height(Length::Fixed(DIVIDER_HEIGHT))
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(trace_color_to_iced(tinted_color))),
            ..iced::widget::container::Style::default()
        });

    container(rule)
        .padding(Padding {
            top: DIVIDER_VERTICAL_SPACING as f32,
            right: 0.0,
            bottom: DIVIDER_VERTICAL_SPACING as f32,
            left: 0.0,
        })
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use trace_core::{AppSettings, ThemePreset, TraceTheme, MOD_CONTROL};

    fn fresh_app() -> SettingsApp {
        SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
        )
    }

    fn sample_palette() -> SettingsPalette {
        TraceTheme::for_preset(ThemePreset::Dark).settings
    }

    #[test]
    fn shortcuts_card_builds_at_rest() {
        let app = fresh_app();
        let _element: Element<'_, SettingsMessage> = shortcuts_card(&app, sample_palette());
    }

    #[test]
    fn shortcuts_card_builds_while_recording_every_target() {
        // Exercise every `ShortcutTarget` variant through the recording
        // branch so a dropped match arm in the card renderer would panic
        // here rather than go unnoticed.
        for target in ShortcutTarget::ALL {
            let mut app = fresh_app();
            app.recording_target = Some(target);
            let _element: Element<'_, SettingsMessage> = shortcuts_card(&app, sample_palette());
        }
    }

    #[test]
    fn shortcuts_card_builds_with_recorder_error_message() {
        let mut app = fresh_app();
        app.shortcut_recorder_message = Some("stale error".to_string());
        let _element: Element<'_, SettingsMessage> = shortcuts_card(&app, sample_palette());
    }

    #[test]
    fn shortcuts_card_builds_across_all_languages() {
        // Every language must paint the card without panicking; a stale L10n
        // key would surface here as a missing localization at paint time.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            let mut app = fresh_app();
            app.language = lang;
            let _element: Element<'_, SettingsMessage> = shortcuts_card(&app, sample_palette());
        }
    }

    #[test]
    fn shortcuts_card_builds_across_all_presets() {
        // Every preset's palette must round-trip through the card renderer.
        // A stale color field on `SettingsPalette` would trip the type
        // checker; a runtime panic would surface here.
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let app = SettingsApp::new(
                TraceTheme::for_preset(preset),
                Arc::new(AppSettings::default()),
            );
            let _element: Element<'_, SettingsMessage> =
                shortcuts_card(&app, TraceTheme::for_preset(preset).settings);
        }
    }

    #[test]
    fn shortcut_display_label_reflects_shortcut_spec_format() {
        // The wrapper must not filter the value — the chip reads the
        // `ShortcutSpec::display_label` output verbatim.
        let spec = ShortcutSpec::new(0x4E, MOD_CONTROL);
        let label = shortcut_display_label(spec);
        assert_eq!(label, spec.display_label());
        assert!(label.contains('N'), "{label}");
        assert!(label.contains("Ctrl"), "{label}");
    }

    #[test]
    fn configurable_row_builds_per_target_at_rest() {
        let app = fresh_app();
        for target in ShortcutTarget::ALL {
            let _element: Element<'_, SettingsMessage> =
                configurable_row(&app, sample_palette(), app.language, target);
        }
    }

    #[test]
    fn configurable_row_builds_per_target_while_recording() {
        let mut app = fresh_app();
        for target in ShortcutTarget::ALL {
            app.recording_target = Some(target);
            let _element: Element<'_, SettingsMessage> =
                configurable_row(&app, sample_palette(), app.language, target);
        }
    }

    #[test]
    fn fixed_row_builds_with_ascii_chip_label() {
        let _element: Element<'_, SettingsMessage> =
            fixed_row(sample_palette(), Language::Zh, "Esc", "close panel");
    }

    #[test]
    fn layout_constants_match_mac_reference() {
        // Freeze the constants against Mac `SettingsView.swift:564-626`.
        assert_eq!(LABEL_COLUMN_WIDTH, 100.0);
        assert_eq!(ROW_VERTICAL_PADDING, 8);
        assert_eq!(CATEGORY_FONT_SIZE, 9.0);
        assert_eq!(TARGET_NAME_FONT_SIZE, 12.0);
        assert_eq!(SHORTCUT_CHIP_FONT_SIZE, 12.0);
        assert_eq!(EDIT_BUTTON_FONT_SIZE, 11.0);
        assert_eq!(FIXED_ROW_LABEL_FONT_SIZE, 12.0);
        assert_eq!(CHIP_HORIZONTAL_PADDING, 8);
        assert_eq!(CHIP_VERTICAL_PADDING, 4);
        assert_eq!(CHIP_CORNER_RADIUS, 6.0);
        assert_eq!(LABEL_COLUMN_SPACING, 2.0);
        assert_eq!(ERROR_MESSAGE_FONT_SIZE, 11.0);
        assert_eq!(ERROR_MESSAGE_TOP_PADDING, 6);
        assert_eq!(DIVIDER_VERTICAL_SPACING, 6);
    }

    #[test]
    fn fixed_row_section_switch_label_uses_en_dash() {
        // Guard the fixed row's section-switch chip label against drift back
        // to the ASCII `~` placeholder — the Mac reference's `⌘1–9` uses
        // U+2013 EN DASH and this is the Windows-side mirror.
        assert_eq!(FIXED_LABEL_SECTION_SWITCH, "Ctrl+1\u{2013}9");
        assert!(!FIXED_LABEL_SECTION_SWITCH.contains('~'));
    }
}

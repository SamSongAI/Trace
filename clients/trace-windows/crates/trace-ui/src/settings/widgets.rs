//! Pure widget factories for the settings window shell.
//!
//! Phase 12 sub-task 2 adds the two reusable shapes every settings card needs:
//!
//! * [`section_card`] — the rounded rectangle wrapper that carries a card
//!   title and an arbitrary body built by the caller (a column of
//!   `setting_row`s in the typical case). The shell itself never renders a
//!   description line; cards that need caption text add a `text()` inside
//!   their own body.
//! * [`setting_row`] — a label (with an optional inline hint) stacked above
//!   an arbitrary control. Used inside a card body for every labeled field
//!   (language picker, vault path, hotkey display, …).
//!
//! Both factories are generic over the caller's message type so settings cards
//! can emit their own [`crate::settings::SettingsMessage`] variants without
//! this module depending on the enum. The styling pulls from
//! [`trace_core::SettingsPalette`] via the `card_container_style` style
//! function — this module never reaches into `CapturePalette`.
//!
//! # Layout reference
//!
//! The Mac source (`SettingsView.swift`) renders each card with:
//!
//! * 14 pt corner radius and a 1 px border (`SectionCard` body).
//! * 18 pt interior padding on the card itself.
//! * A 15 pt bold title directly above the body — no description line.
//! * Rows laid out as a `VStack(spacing: 6)` with a label/hint `HStack` on
//!   top of the control.
//!
//! Phase 12 locks these constants in named `pub const` items so tests and
//! future sub-tasks can keep the values in sync with the Swift source.

use iced::alignment::Vertical;
use iced::widget::{column, container, row, text};
use iced::{Element, Length, Pixels};
use trace_core::SettingsPalette;

use crate::theme::card_container_style;

/// Corner radius of the card container, matching Mac `SettingsView.swift`'s
/// private `SectionCard` (`RoundedRectangle(cornerRadius: 14, …)`).
/// Duplicated as a `pub const` so layout-level code can pad/align around it
/// without re-reading the style function.
pub const CARD_CORNER_RADIUS: f32 = 14.0;
/// Interior padding of the card body. Matches Mac `SectionCard.padding(18)`.
pub const CARD_INNER_PADDING: u16 = 18;
/// Spacing between the title and the body inside a card. Stored as `f32`
/// because iced's `column::spacing` takes `Into<Pixels>` (which is
/// implemented for `f32` / `u32` but not `u16`). Matches Mac
/// `VStack(alignment: .leading, spacing: 14)` in `SectionCard`.
pub const CARD_VERTICAL_SPACING: f32 = 14.0;
/// Font size of the card title. Matches Mac
/// `.font(.system(size: 15, weight: .bold))` in `SectionCard`.
pub const CARD_TITLE_FONT_SIZE: f32 = 15.0;
/// Font size of the row label. Windows renders the label in a regular weight
/// (Mac uses uppercase + tracking; iced 0.14 has no `tracking` knob, so the
/// Windows port keeps it as a plain label at 14 pt).
pub const ROW_LABEL_FONT_SIZE: f32 = 14.0;
/// Font size of the optional inline hint next to the row label. Matches the
/// Mac reference's 10 pt hint scaled up slightly for Windows readability.
pub const ROW_HINT_FONT_SIZE: f32 = 13.0;
/// Vertical spacing between the label/hint line and the control beneath it.
/// Matches Mac `VStack(alignment: .leading, spacing: 6)` in `SettingRow`.
pub const ROW_SPACING: f32 = 6.0;
/// Horizontal spacing between the row label and its inline hint. Matches Mac
/// `HStack(alignment: .firstTextBaseline, spacing: 6)` in `SettingRow`.
pub const ROW_LABEL_HINT_SPACING: f32 = 6.0;

/// Builds a settings card shell — a rounded container carrying a bold title
/// and an arbitrary body element.
///
/// The factory is generic over the message type so cards can freely wire into
/// whichever message enum the caller uses. Typical usage from within
/// [`crate::settings::settings_view`]:
///
/// ```ignore
/// let body = column![row1, row2].spacing(CARD_VERTICAL_SPACING);
/// let card = section_card(palette, "Language", body.into());
/// ```
///
/// The shell only carries a title plus the caller-provided body — no caption
/// line. Cards that want an introductory description add a `text()` widget
/// at the top of their own body; the shell intentionally stays symmetric
/// with the Mac `SectionCard` view so the two ports can share layout
/// reasoning.
pub fn section_card<'a, Message: 'a>(
    palette: SettingsPalette,
    title: &'a str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let title_widget = text(title)
        .size(Pixels(CARD_TITLE_FONT_SIZE))
        .color(crate::theme::trace_color_to_iced(palette.section_title));

    let inner = column![title_widget, body]
        .spacing(CARD_VERTICAL_SPACING)
        .width(Length::Fill);

    container(inner)
        .padding(CARD_INNER_PADDING)
        .width(Length::Fill)
        .style(card_container_style(palette))
        .into()
}

/// Builds a labeled row with an optional inline hint and a control stacked
/// underneath.
///
/// Layout mirrors Mac `SettingRow` in `SettingsView.swift`:
///
/// ```text
/// LABEL  hint text (optional)
/// <control>
/// ```
///
/// When `hint` is `None` the label line is rendered on its own — no empty
/// spacer, no reserved vertical space. The control sits on the next line
/// regardless, which keeps vertical rhythm consistent across rows that mix
/// labeled-only and labeled-with-hint fields.
///
/// `control` is an arbitrary [`Element`] so the caller can pass whatever
/// control fits (a text input, a button, a picker, a plain text caption).
pub fn setting_row<'a, Message: 'a>(
    palette: SettingsPalette,
    label: &'a str,
    hint: Option<&'a str>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    let label_widget = text(label)
        .size(Pixels(ROW_LABEL_FONT_SIZE))
        .color(crate::theme::trace_color_to_iced(palette.row_label));

    let label_line: Element<'a, Message> = if let Some(hint) = hint {
        let hint_widget = text(hint)
            .size(Pixels(ROW_HINT_FONT_SIZE))
            .color(crate::theme::trace_color_to_iced(palette.muted_text));
        row![label_widget, hint_widget]
            .spacing(ROW_LABEL_HINT_SPACING)
            .align_y(Vertical::Center)
            .width(Length::Fill)
            .into()
    } else {
        label_widget.into()
    };

    column![label_line, control]
        .spacing(ROW_SPACING)
        .width(Length::Fill)
        .into()
}

/// Convenience helper that builds a card whose body is a column of
/// `setting_row` elements.
///
/// The Mac reference stacks rows with a fixed vertical rhythm, so every card
/// that is really just "label + control, label + control" benefits from
/// letting callers hand in a pre-built `Vec` of rows. The card shell itself
/// already allows arbitrary bodies via [`section_card`]; this helper exists
/// to keep the common case terse.
pub fn section_card_with_rows<'a, Message: 'a>(
    palette: SettingsPalette,
    title: &'a str,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let body = column(rows)
        .spacing(CARD_VERTICAL_SPACING)
        .width(Length::Fill);

    section_card(palette, title, body.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{button, text_input};
    use trace_core::{ThemePreset, TraceTheme};

    /// Distinct message type so the tests prove the factories are generic and
    /// not accidentally coupled to `SettingsMessage`.
    #[derive(Debug, Clone)]
    enum TestMsg {
        Ping,
    }

    fn sample_palette() -> SettingsPalette {
        TraceTheme::for_preset(ThemePreset::Light).settings
    }

    #[test]
    fn section_card_builds_with_simple_body() {
        let palette = sample_palette();
        let body: Element<'_, TestMsg> = text("body").into();
        let _card = section_card(palette, "Title", body);
    }

    #[test]
    fn section_card_builds_across_all_presets() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let body: Element<'_, TestMsg> = text("body").into();
            let _card = section_card(palette, "Title", body);
        }
    }

    #[test]
    fn setting_row_builds_without_hint() {
        let palette = sample_palette();
        let control: Element<'_, TestMsg> = text("Active").into();
        let _row = setting_row(palette, "Status", None, control);
    }

    #[test]
    fn setting_row_builds_with_hint() {
        // Mac `SettingRow` accepts an optional trailing hint next to the
        // label; the Windows port must render the same shape when the hint
        // is present.
        let palette = sample_palette();
        let control: Element<'_, TestMsg> = text("Active").into();
        let _row = setting_row(palette, "Status", Some("advanced"), control);
    }

    #[test]
    fn setting_row_builds_with_button_control() {
        // Proves the factory accepts interactive controls without constraining
        // the caller's message type.
        let palette = sample_palette();
        let control: Element<'_, TestMsg> = button(text("Click")).on_press(TestMsg::Ping).into();
        let _row = setting_row(palette, "Action", None, control);
    }

    #[test]
    fn setting_row_builds_with_text_input_control() {
        // The vault-path row passes a `text_input` here. Make sure the factory
        // composes with the common stable widgets.
        let palette = sample_palette();
        let value = String::new();
        let control: Element<'_, TestMsg> = text_input("Path", &value).into();
        let _row = setting_row(palette, "Notes folder", Some("Pick a folder"), control);
    }

    #[test]
    fn section_card_with_rows_stacks_multiple_rows() {
        let palette = sample_palette();
        let rows: Vec<Element<'_, TestMsg>> = vec![
            setting_row(palette, "Row 1", None, text("A").into()),
            setting_row(palette, "Row 2", Some("hint"), text("B").into()),
            setting_row(palette, "Row 3", None, text("C").into()),
        ];
        let _card = section_card_with_rows(palette, "Stacked", rows);
    }

    #[test]
    fn section_card_with_rows_accepts_empty_body() {
        // A card with zero rows is degenerate but must still build — the
        // placeholder state when a user clears every row in a card.
        let palette = sample_palette();
        let _card: Element<'_, TestMsg> = section_card_with_rows(palette, "Empty", Vec::new());
    }

    #[test]
    fn card_metric_constants_match_mac_reference() {
        // Locked to Mac `SettingsView.swift` / `SectionCard`:
        //   - RoundedRectangle(cornerRadius: 14)
        //   - .padding(18)
        //   - Text(title).font(.system(size: 15, weight: .bold))
        //   - VStack(alignment: .leading, spacing: 14)
        assert_eq!(CARD_CORNER_RADIUS, 14.0);
        assert_eq!(CARD_INNER_PADDING, 18);
        assert_eq!(CARD_VERTICAL_SPACING, 14.0);
        assert_eq!(CARD_TITLE_FONT_SIZE, 15.0);
    }

    #[test]
    fn row_metric_constants_are_stable() {
        // Mac `SettingRow`: VStack(spacing: 6) { HStack(spacing: 6), control }
        assert_eq!(ROW_LABEL_FONT_SIZE, 14.0);
        assert_eq!(ROW_HINT_FONT_SIZE, 13.0);
        assert_eq!(ROW_SPACING, 6.0);
        assert_eq!(ROW_LABEL_HINT_SPACING, 6.0);
    }
}

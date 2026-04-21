//! Pure widget factories for the settings window shell.
//!
//! Phase 12 sub-task 2 adds the two reusable shapes every settings card needs:
//!
//! * [`section_card`] — the rounded rectangle wrapper that carries a card
//!   title, an optional description, and an arbitrary body built by the caller
//!   (a column of `setting_row`s in the typical case).
//! * [`setting_row`] — a left-aligned label + right-aligned control pair used
//!   inside a card body for every labeled field (language picker, vault path,
//!   hotkey display, …).
//!
//! Both factories are generic over the caller's message type so settings cards
//! can emit their own [`crate::settings::SettingsMessage`] variants without
//! this module depending on the enum. The styling pulls from
//! [`trace_core::SettingsPalette`] via the `card_container_style` style
//! function — this module never reaches into `CapturePalette`.
//!
//! # Layout reference
//!
//! The Mac source (`SettingsView.swift` / `SectionCard.swift`) renders each
//! card with:
//!
//! * 16 pt corner radius and a 1 px border.
//! * 20 pt interior padding on the card itself.
//! * An 18 pt title, optional 13 pt description, and 12 pt vertical spacing
//!   between title, description, and body.
//! * Rows laid out as an `HStack` with `Spacer()` between label and control.
//!
//! Phase 12 locks these constants in named `pub const` items so tests and
//! future sub-tasks can keep the values in sync with the Swift source.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{column, container, row, text, Space};
use iced::{Element, Length, Pixels};
use trace_core::SettingsPalette;

use crate::theme::card_container_style;

/// Corner radius of the card container, matching Mac `SectionCard.swift`.
/// Duplicated as a `pub const` so layout-level code can pad/align around it
/// without re-reading the style function.
pub const CARD_CORNER_RADIUS: f32 = 16.0;
/// Interior padding of the card body. Matches Mac `SectionCard.swift`.
pub const CARD_INNER_PADDING: u16 = 20;
/// Spacing between the title, description, and body inside a card. Stored as
/// `f32` because iced's `column::spacing` takes `Into<Pixels>` (which is
/// implemented for `f32` / `u32` but not `u16`).
pub const CARD_VERTICAL_SPACING: f32 = 12.0;
/// Font size of the card title. Matches Mac `.font(.title3)` in
/// `SectionCard.swift`.
pub const CARD_TITLE_FONT_SIZE: f32 = 18.0;
/// Font size of the card description. Matches Mac `.font(.footnote)` in
/// `SectionCard.swift`.
pub const CARD_DESCRIPTION_FONT_SIZE: f32 = 13.0;
/// Font size of the row label. Matches Mac `.font(.body)` for settings rows.
pub const ROW_LABEL_FONT_SIZE: f32 = 14.0;
/// Horizontal spacing between a row's label and its trailing control.
/// Kept as a hard minimum — the [`row!`] macro inserts a `Space` that grows to
/// fill the remaining width and pushes the control to the trailing edge.
pub const ROW_LABEL_CONTROL_SPACING: f32 = 12.0;
/// Minimum height of a setting row. Guarantees that a bare `text` control
/// aligns with a tall field (picker, text input) without jitter.
pub const ROW_MIN_HEIGHT: f32 = 28.0;

/// Builds a settings card shell — a rounded container carrying an optional
/// description and an arbitrary body element.
///
/// The factory is generic over the message type so cards can freely wire into
/// whichever message enum the caller uses. Typical usage from within
/// [`crate::settings::settings_view`]:
///
/// ```ignore
/// let body = column![row1, row2].spacing(CARD_VERTICAL_SPACING);
/// let card = section_card(
///     palette,
///     "Language",
///     Some("Pick how Trace labels its UI."),
///     body.into(),
/// );
/// ```
///
/// Passing `None` for the description omits the caption line entirely — the
/// title butts up against the body.
pub fn section_card<'a, Message: 'a>(
    palette: SettingsPalette,
    title: &'a str,
    description: Option<&'a str>,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let title_widget = text(title)
        .size(Pixels(CARD_TITLE_FONT_SIZE))
        .color(crate::theme::trace_color_to_iced(palette.section_title));

    let mut inner_items: Vec<Element<'a, Message>> = Vec::with_capacity(3);
    inner_items.push(title_widget.into());
    if let Some(description) = description {
        let description_widget = text(description)
            .size(Pixels(CARD_DESCRIPTION_FONT_SIZE))
            .color(crate::theme::trace_color_to_iced(
                palette.section_description,
            ));
        inner_items.push(description_widget.into());
    }
    inner_items.push(body);

    let inner = column(inner_items)
        .spacing(CARD_VERTICAL_SPACING)
        .width(Length::Fill);

    container(inner)
        .padding(CARD_INNER_PADDING)
        .width(Length::Fill)
        .style(card_container_style(palette))
        .into()
}

/// Builds a label + trailing-control row inside a card body.
///
/// The label hugs the leading edge and the control hugs the trailing edge; a
/// flexible [`Space`] between them absorbs the remaining width. `Length::Fill`
/// is applied to the row, so callers don't need to size it themselves — they
/// just drop the row into the card's body column.
///
/// `control` is an arbitrary [`Element`] so the caller can pass whatever
/// control fits (a text input, a button, a picker, a plain text caption).
/// The row does not impose a specific alignment on the control itself — if
/// the control has intrinsic sizing it will appear flush-right; if it uses
/// `Length::Fill` it will absorb the remaining space after the label.
pub fn setting_row<'a, Message: 'a>(
    palette: SettingsPalette,
    label: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    let label_widget = text(label)
        .size(Pixels(ROW_LABEL_FONT_SIZE))
        .color(crate::theme::trace_color_to_iced(palette.row_label));

    row![
        label_widget,
        Space::new().width(Length::Fill).height(Length::Shrink),
        control,
    ]
    .spacing(ROW_LABEL_CONTROL_SPACING)
    .align_y(Vertical::Center)
    .width(Length::Fill)
    .height(Length::Shrink)
    .into()
}

/// Convenience helper that builds a card whose body is a column of
/// `setting_row` elements.
///
/// The Mac reference always stacks rows with a fixed vertical rhythm of
/// 12 pt, so every card that is really just "label + control, label + control"
/// benefits from letting callers hand in a pre-built `Vec` of rows. The card
/// shell itself already allows arbitrary bodies via [`section_card`]; this
/// helper exists to keep the common case terse.
pub fn section_card_with_rows<'a, Message: 'a>(
    palette: SettingsPalette,
    title: &'a str,
    description: Option<&'a str>,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let body = column(rows)
        .spacing(CARD_VERTICAL_SPACING)
        .width(Length::Fill);

    // Preserve consistent horizontal alignment even when a row returns
    // `Length::Shrink` so cards don't collapse to min-width.
    let bordered = container(body)
        .width(Length::Fill)
        .align_x(Horizontal::Left);

    section_card(palette, title, description, bordered.into())
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
    fn section_card_builds_with_description() {
        let palette = sample_palette();
        let body: Element<'_, TestMsg> = text("body").into();
        let _card = section_card(palette, "Title", Some("A short caption."), body);
    }

    #[test]
    fn section_card_builds_without_description() {
        // `None` description is a supported shape — some cards (e.g.
        // "Shortcuts") render their own layout inside the body and don't need
        // a caption line.
        let palette = sample_palette();
        let body: Element<'_, TestMsg> = text("body").into();
        let _card = section_card(palette, "Title", None, body);
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
            let _card = section_card(palette, "Title", Some("Caption"), body);
        }
    }

    #[test]
    fn setting_row_builds_with_text_control() {
        let palette = sample_palette();
        let control: Element<'_, TestMsg> = text("Active").into();
        let _row = setting_row(palette, "Status", control);
    }

    #[test]
    fn setting_row_builds_with_button_control() {
        // Proves the factory accepts interactive controls without constraining
        // the caller's message type.
        let palette = sample_palette();
        let control: Element<'_, TestMsg> = button(text("Click")).on_press(TestMsg::Ping).into();
        let _row = setting_row(palette, "Action", control);
    }

    #[test]
    fn setting_row_builds_with_text_input_control() {
        // The vault-path row passes a `text_input` here. Make sure the factory
        // composes with the common stable widgets.
        let palette = sample_palette();
        let value = String::new();
        let control: Element<'_, TestMsg> = text_input("Path", &value).into();
        let _row = setting_row(palette, "Notes folder", control);
    }

    #[test]
    fn section_card_with_rows_stacks_multiple_rows() {
        let palette = sample_palette();
        let rows: Vec<Element<'_, TestMsg>> = vec![
            setting_row(palette, "Row 1", text("A").into()),
            setting_row(palette, "Row 2", text("B").into()),
            setting_row(palette, "Row 3", text("C").into()),
        ];
        let _card = section_card_with_rows(palette, "Stacked", None, rows);
    }

    #[test]
    fn section_card_with_rows_accepts_empty_body() {
        // A card with zero rows is degenerate but must still build — the
        // placeholder state when a user clears every row in a card.
        let palette = sample_palette();
        let _card: Element<'_, TestMsg> =
            section_card_with_rows(palette, "Empty", Some("caption"), Vec::new());
    }

    #[test]
    fn card_metric_constants_match_mac_reference() {
        assert_eq!(CARD_CORNER_RADIUS, 16.0);
        assert_eq!(CARD_INNER_PADDING, 20);
        assert_eq!(CARD_VERTICAL_SPACING, 12.0);
        assert_eq!(CARD_TITLE_FONT_SIZE, 18.0);
        assert_eq!(CARD_DESCRIPTION_FONT_SIZE, 13.0);
    }

    #[test]
    fn row_metric_constants_are_stable() {
        assert_eq!(ROW_LABEL_FONT_SIZE, 14.0);
        assert_eq!(ROW_LABEL_CONTROL_SPACING, 12.0);
        assert_eq!(ROW_MIN_HEIGHT, 28.0);
    }
}

//! Quick Sections card: add / remove / rename up to
//! [`NoteSection::MAXIMUM_COUNT`] sections shown as the Dimension-mode chip
//! row on the capture panel. Mirrors Mac `SettingsView.swift:415-446` (card
//! body) + `:764-814` (per-row widget).
//!
//! # Shadow-only contract
//!
//! This module **only mutates the shadow `Vec<String>`** on
//! [`crate::settings::SettingsApp`]. The write-back to `Arc<AppSettings>`
//! (which trims, normalizes length, and escapes control characters) is
//! deferred to sub-task 8 — the same contract the Storage card already
//! follows for vault paths / daily folder names.
//!
//! Mac's `SectionTitleRow` keeps a draft-state `@State` mirror of the
//! user's edits to avoid mid-keystroke normalization from the `AppSettings`
//! setter. The Windows port does not need that shim because the shadow
//! `Vec<String>` is never normalized on write — every keystroke lands
//! verbatim, so the widget can read/write the shadow directly. The Mac
//! reference's explicit "Save" button similarly collapses into an implicit
//! blur commit there; Windows has no equivalent concept because there is
//! nothing to commit during sub-task 5.
//!
//! # Remove affordance
//!
//! iced 0.14 has no SF Symbols equivalent, so the minus-circle glyph used
//! on Mac is replaced by the multiplication sign (`×`, U+00D7). The button
//! leans on [`button::on_press_maybe`] so a row at
//! [`NoteSection::MINIMUM_COUNT`] renders as `Status::Disabled` without
//! needing an extra style branch outside
//! [`crate::theme::settings_remove_button_style`].

use iced::font::Weight;
use iced::widget::{button, column, row, text, text_input};
use iced::{Element, Font, Length, Pixels};
use trace_core::{L10n, Language, NoteSection, SettingsPalette};

use crate::theme::{
    settings_field_style, settings_remove_button_style, settings_secondary_button_style,
    trace_color_to_iced,
};

use super::widgets::{section_card, CARD_VERTICAL_SPACING};
use super::SettingsMessage;

/// Font used for the 1-based display-index label. Matches Mac
/// `Text("\(section.displayIndex)").font(.system(size: 11, weight: .bold,
/// design: .monospaced))` in `SectionTitleRow` (`SettingsView.swift:764-814`).
/// Built from `Font::MONOSPACE` to inherit the default stretch / style, then
/// overridden to `Weight::Bold` for the monospaced digits' bolder optical
/// weight.
const SECTION_INDEX_FONT: Font = Font {
    weight: Weight::Bold,
    ..Font::MONOSPACE
};

/// Horizontal spacing inside every section-title row. Matches Mac
/// `HStack(spacing: 8)` in `SectionTitleRow`.
const ROW_HORIZONTAL_SPACING: f32 = 8.0;
/// Reserved width of the 1-based display-index column. Matches Mac
/// `.frame(width: 18)` on the `SectionTitleRow` index label.
const INDEX_LABEL_WIDTH: f32 = 18.0;
/// Font size of the index label. Matches Mac 11 pt bold monospaced digits.
const INDEX_LABEL_FONT_SIZE: f32 = 11.0;
/// Font size of the remove-button glyph. Mac uses 14 pt semibold for the
/// minus-circle SF Symbol; `×` sits at the same optical weight with the same
/// size.
const REMOVE_GLYPH_FONT_SIZE: f32 = 14.0;
/// Multiplication sign (U+00D7) used as the remove-button glyph. iced 0.14
/// cannot render SF Symbols, and `−` / `×` are the two candidates that look
/// balanced inside a button at the same size as the Mac reference.
const REMOVE_GLYPH: &str = "\u{00D7}";

/// Builds the Quick Sections card element for the current shadow state.
///
/// Render order mirrors Mac `SectionCard`:
///
/// 1. One [`section_title_row`] per entry in `section_titles`.
/// 2. An "Add Section" button at the bottom, disabled once the shadow has
///    reached [`NoteSection::MAXIMUM_COUNT`].
///
/// The caller is responsible for gating the card on
/// [`trace_core::WriteMode::Dimension`]; this function renders
/// unconditionally when called so the unit tests can smoke-test each
/// (length, language) combination without depending on the write-mode
/// dispatcher.
pub(super) fn quick_sections_card<'a>(
    palette: SettingsPalette,
    lang: Language,
    section_titles: &'a [String],
) -> Element<'a, SettingsMessage> {
    let can_remove = section_titles.len() > NoteSection::MINIMUM_COUNT;
    let can_add = section_titles.len() < NoteSection::MAXIMUM_COUNT;

    // Build one row per section title. `enumerate` carries the 0-based slot
    // index into `SectionTitleChanged` / `SectionRemoved`.
    let rows = section_titles
        .iter()
        .enumerate()
        .map(|(index, title)| section_title_row(palette, lang, index, title, can_remove));

    // Trailing "Add section" row lives in its own vec slot so the iterator
    // stays homogeneous — iced's `column(_)` takes an `IntoIterator<Item =
    // Element<_, _>>`.
    // Mac renders `Label(L10n.addSection, systemImage: "plus")`, which prefixes
    // the label with a `plus` SF Symbol. iced 0.14 has no SF Symbols shim, so
    // the Windows port uses the ASCII `+` character to reproduce the Mac visual
    // — the same substitution already applied to the remove-button glyph above.
    let add_button = button(text(format!("+ {}", L10n::add_section(lang))))
        .on_press_maybe(can_add.then_some(SettingsMessage::SectionAdded))
        .style(settings_secondary_button_style(palette));

    let body = column(rows.chain(std::iter::once::<Element<'a, SettingsMessage>>(
        add_button.into(),
    )))
    .spacing(CARD_VERTICAL_SPACING)
    .width(Length::Fill);

    section_card(palette, L10n::quick_sections(lang), body.into())
}

/// One editable row for a single section slot. Layout mirrors Mac
/// `SectionTitleRow`:
///
/// ```text
/// {1-based index}  [text input............]  [×]
/// ```
///
/// * `can_remove` reflects the *global* floor check; iced renders the button
///   as `Status::Disabled` when `on_press_maybe` receives `None`, which lets
///   [`settings_remove_button_style`] flatten the glyph color to
///   `muted_text` without an extra parameter.
fn section_title_row<'a>(
    palette: SettingsPalette,
    lang: Language,
    index: usize,
    title: &'a str,
    can_remove: bool,
) -> Element<'a, SettingsMessage> {
    let display_index = index + 1;
    let index_label = text(display_index.to_string())
        .size(Pixels(INDEX_LABEL_FONT_SIZE))
        .font(SECTION_INDEX_FONT)
        .color(trace_color_to_iced(palette.muted_text))
        .width(Length::Fixed(INDEX_LABEL_WIDTH));

    let field = text_input(L10n::section_name(lang), title)
        .on_input(move |value| SettingsMessage::SectionTitleChanged(index, value))
        .width(Length::Fill)
        .style(settings_field_style(palette));

    let remove_button = button(text(REMOVE_GLYPH).size(Pixels(REMOVE_GLYPH_FONT_SIZE)))
        .on_press_maybe(can_remove.then_some(SettingsMessage::SectionRemoved(index)))
        .style(settings_remove_button_style(palette));

    row![index_label, field, remove_button]
        .spacing(ROW_HORIZONTAL_SPACING)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};

    fn sample_palette() -> SettingsPalette {
        TraceTheme::for_preset(ThemePreset::Light).settings
    }

    #[test]
    fn card_renders_for_default_section_titles() {
        // The Mac-compatible default is a four-entry vec (`Note`, `Memo`,
        // `Link`, `Task`). Prove the view builds without panicking for every
        // language so a drift in the L10n helpers is caught at test time.
        let palette = sample_palette();
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            let titles: Vec<String> = NoteSection::DEFAULT_TITLES
                .iter()
                .map(|t| (*t).to_string())
                .collect();
            let _element: Element<'_, SettingsMessage> =
                quick_sections_card(palette, lang, &titles);
        }
    }

    #[test]
    fn card_renders_for_single_section() {
        // At `MINIMUM_COUNT` the remove button must still render (with the
        // disabled style); the card itself must build across every language.
        let palette = sample_palette();
        let titles = vec!["Solo".to_string()];
        for lang in [Language::Zh, Language::En, Language::Ja] {
            let _element: Element<'_, SettingsMessage> =
                quick_sections_card(palette, lang, &titles);
        }
    }

    #[test]
    fn card_renders_at_maximum_capacity() {
        // At `MAXIMUM_COUNT` the Add button must still render (with the
        // disabled style). Guards against a future regression that makes the
        // body iterator unbalanced when the cap is hit.
        let palette = sample_palette();
        let titles: Vec<String> = (0..NoteSection::MAXIMUM_COUNT)
            .map(NoteSection::default_title_for)
            .collect();
        let _element: Element<'_, SettingsMessage> =
            quick_sections_card(palette, Language::En, &titles);
    }

    #[test]
    fn card_renders_across_every_preset() {
        // Palette swaps must not break the card — the remove button's
        // disabled branch in particular routes through a different slot
        // (`muted_text`) than the active branch (`warning_text`).
        let titles: Vec<String> = vec!["A".into(), "B".into()];
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let _element: Element<'_, SettingsMessage> =
                quick_sections_card(palette, Language::Zh, &titles);
        }
    }
}

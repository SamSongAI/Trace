//! Threads card: per-row CRUD UI for the `thread_configs` shadow on
//! [`crate::settings::SettingsApp`]. Mirrors Mac `SettingsView.swift:448-520`
//! (card shell) plus `ThreadConfigRow.swift` (row widget).
//!
//! # Shadow-only contract
//!
//! Like the Quick Sections card, this module **only mutates the shadow
//! `Vec<ThreadConfig>`** on `SettingsApp`. Write-back to `Arc<AppSettings>`
//! (which trims whitespace, enforces uniqueness, and persists to disk) is
//! deferred to sub-task 8 so the save semantics stay consistent across cards.
//!
//! # Visual diff vs the Mac reference
//!
//! iced 0.14 does not expose a `Divider` widget; the Mac reference separates
//! the folder text input from the "Choose Folder" button with a 1-pt vertical
//! rule inside a shared field background. The Windows port drops the shared
//! background + rule and renders `row![text_input, button]` directly — the
//! Sub-task 6 brief explicitly sanctions this as the minimum-viable visual
//! shape. Pixel-accurate re-do of the divider lives in a polish pass.

use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::{button, column, row, text, text_input};
use iced::{Element, Font, Length, Pixels};
use trace_core::{split_target_file, L10n, Language, SettingsPalette, ThreadConfig};

use crate::theme::{
    settings_field_style, settings_remove_button_style, settings_secondary_button_style,
};

use super::widgets::{section_card, CARD_VERTICAL_SPACING};
use super::SettingsMessage;

/// Horizontal spacing between adjacent columns in a row (name field, folder
/// row, filename field, remove button) and between the folder input and its
/// choose-folder button. Matches Mac `HStack(spacing: 8)` in `ThreadConfigRow`.
const COLUMN_SPACING: f32 = 8.0;
/// Horizontal spacing between the folder text input and its "Choose Folder"
/// trailing button. Mac uses 6 pt here (`HStack(spacing: 6)`); a slightly
/// tighter gap reads as a single control group.
const FOLDER_CONTROL_SPACING: f32 = 6.0;
/// Fixed width of the thread-name column. Matches Mac
/// `.frame(width: 100)` on the name field.
const NAME_WIDTH: f32 = 100.0;
/// Minimum width of the folder-path column. Matches Mac
/// `.frame(minWidth: 380, maxWidth: .infinity)`.
///
/// iced 0.14's [`iced::Length`] enum does not carry a `min_width` variant
/// (only `Fixed` / `Fill` / `Shrink` / `FillPortion`), so the Windows port
/// uses `Length::Fill` on the folder row instead. The constant is kept as
/// documentation of the Mac contract and as a fixture for the layout-
/// constant regression test. The only reference lives under `#[cfg(test)]`
/// so the non-test build's `dead_code` lint fires — the `#[allow]` here
/// anchors the constant in the source-of-truth file regardless of build
/// flavour, which is cheaper than duplicating the value into the test
/// module.
#[allow(dead_code)]
const FOLDER_MIN_WIDTH: f32 = 380.0;
/// Fixed width of the filename column. Matches Mac `.frame(width: 120)` on
/// the filename field.
const FILE_NAME_WIDTH: f32 = 120.0;
/// Fixed width of the per-row remove button. Gives the `×` glyph a consistent
/// hit target across all rows.
const DELETE_BUTTON_WIDTH: f32 = 28.0;
/// Font size of the column-header labels. 11 pt with `Weight::Semibold` lines
/// up with the Mac reference's `.font(.system(size: 11, weight: .semibold))`.
const HEADER_FONT_SIZE: f32 = 11.0;
/// Font size of the name text input. Matches Mac `.font(.system(size: 13,
/// weight: .medium))`.
const NAME_FIELD_FONT_SIZE: f32 = 13.0;
/// Font size of the folder / filename text inputs. Matches Mac `.font(.system(
/// size: 12, weight: .medium))`.
const PATH_FIELD_FONT_SIZE: f32 = 12.0;
/// Font size of the remove-button glyph. Mac `14 pt, semibold`.
const REMOVE_GLYPH_FONT_SIZE: f32 = 14.0;
/// Font size of the trailing "Choose Folder" button label. Mac `11 pt,
/// medium`.
const CHOOSE_FOLDER_FONT_SIZE: f32 = 11.0;
/// Multiplication sign (U+00D7) used as the per-row remove glyph. Matches the
/// substitution already applied by the Quick Sections card; iced 0.14 cannot
/// render SF Symbols and `×` reads at the same optical weight as the Mac
/// `minus.circle.fill` at the same size.
const REMOVE_GLYPH: &str = "\u{00D7}";

/// Font used for the column-header labels: semibold so the 11 pt header
/// differentiates from the slightly larger row inputs.
const HEADER_FONT: Font = Font {
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

/// Builds the Threads card for the current shadow state.
///
/// Render order mirrors Mac `SettingsView.swift:448-520`:
///
/// 1. Column headers (name / folder / filename).
/// 2. One [`thread_config_row`] per entry in `thread_configs`, sorted by
///    `order` ascending. The shadow vec's physical order is not disturbed —
///    this function pulls references into a fresh `Vec<&ThreadConfig>` and
///    sorts the references so a future reorder UX can stay a pure
///    data-layer operation.
/// 3. A trailing "Add Thread" button at the bottom, disabled once the shadow
///    has reached [`ThreadConfig::MAXIMUM_COUNT`].
///
/// `vault_path` is threaded through unused at the render layer today — the
/// Mac reference consumes it inside `chooseFolder` to compute relative
/// paths. That logic lives in the `BrowseThreadFolderRequested` /
/// `ThreadFolderBrowseChose` update branches on the Windows port, so the
/// card itself does not need to read the vault path during a view pass.
/// The parameter stays in the signature so a future sub-task that wants to
/// surface (for example) a "relative to vault" hint next to the folder
/// column can do so without a breaking API change.
pub(super) fn threads_card<'a>(
    palette: SettingsPalette,
    lang: Language,
    thread_configs: &'a [ThreadConfig],
    _vault_path: &'a str,
) -> Element<'a, SettingsMessage> {
    let can_add = thread_configs.len() < ThreadConfig::MAXIMUM_COUNT;
    let can_remove = thread_configs.len() > ThreadConfig::MINIMUM_COUNT;

    // Sort references by `order` without re-ordering the backing shadow. This
    // keeps the view pure and lets the update layer stay free of "re-sort
    // after every edit" plumbing.
    let mut sorted: Vec<&ThreadConfig> = thread_configs.iter().collect();
    sorted.sort_by_key(|t| t.order);

    let header = column_header(palette, lang);

    let rows = sorted
        .into_iter()
        .map(|thread| thread_config_row(palette, lang, thread, can_remove));

    // Mac prefixes the add button with a `plus` SF Symbol (`Label(
    // L10n.addThread, systemImage: "plus")`); the Windows port uses the
    // ASCII `+` character for the same visual, mirroring the substitution
    // applied to the Quick Sections "Add Section" button.
    let add_button = button(text(format!("+ {}", L10n::add_thread(lang))))
        .on_press_maybe(can_add.then_some(SettingsMessage::ThreadAdded))
        .style(settings_secondary_button_style(palette));

    let body = column(
        std::iter::once::<Element<'a, SettingsMessage>>(header)
            .chain(rows)
            .chain(std::iter::once::<Element<'a, SettingsMessage>>(add_button.into())),
    )
    .spacing(CARD_VERTICAL_SPACING)
    .width(Length::Fill);

    section_card(palette, L10n::thread_management(lang), body.into())
}

/// Builds the three-column header labeling the name, folder, and filename
/// columns. Uses `muted_text` / `Weight::Semibold` at 11 pt to sit
/// visually below the row inputs without disappearing.
fn column_header<'a>(palette: SettingsPalette, lang: Language) -> Element<'a, SettingsMessage> {
    let make_label = move |label: &'a str, width: Length| {
        text(label)
            .size(Pixels(HEADER_FONT_SIZE))
            .font(HEADER_FONT)
            .color(crate::theme::trace_color_to_iced(palette.muted_text))
            .width(width)
    };

    // The remove-button column uses an empty spacer so the header line's
    // column widths line up with the row's `[name, folder, filename, delete]`
    // layout below.
    row![
        make_label(L10n::thread_name(lang), Length::Fixed(NAME_WIDTH)),
        make_label(L10n::folder_path(lang), Length::Fill),
        make_label(L10n::file_name(lang), Length::Fixed(FILE_NAME_WIDTH)),
        text("").width(Length::Fixed(DELETE_BUTTON_WIDTH)),
    ]
    .spacing(COLUMN_SPACING)
    .align_y(Vertical::Center)
    .width(Length::Fill)
    .into()
}

/// One editable row for a single thread. Layout mirrors Mac
/// `ThreadConfigRow.body`:
///
/// ```text
/// [ name input ]  [ folder input | Choose Folder ]  [ filename input ]  [ × ]
/// ```
///
/// The three edits emit the per-field [`SettingsMessage`] variants so the
/// shadow update layer can split/join `target_file` on every keystroke.
/// The "Choose Folder" button kicks off the async picker via
/// [`SettingsMessage::BrowseThreadFolderRequested`].
fn thread_config_row<'a>(
    palette: SettingsPalette,
    lang: Language,
    thread: &'a ThreadConfig,
    can_remove: bool,
) -> Element<'a, SettingsMessage> {
    let thread_id = thread.id;
    let (folder, filename) = split_target_file(&thread.target_file);

    let name_field = text_input(L10n::thread_name(lang), &thread.name)
        .on_input(move |value| SettingsMessage::ThreadNameChanged(thread_id, value))
        .size(Pixels(NAME_FIELD_FONT_SIZE))
        .width(Length::Fixed(NAME_WIDTH))
        .style(settings_field_style(palette));

    let folder_input = text_input(L10n::folder_path(lang), &folder)
        .on_input(move |value| SettingsMessage::ThreadFolderChanged(thread_id, value))
        .size(Pixels(PATH_FIELD_FONT_SIZE))
        .width(Length::Fill)
        .style(settings_field_style(palette));

    let choose_button = button(
        text(L10n::choose_folder(lang)).size(Pixels(CHOOSE_FOLDER_FONT_SIZE)),
    )
    .on_press(SettingsMessage::BrowseThreadFolderRequested(thread_id))
    .style(settings_secondary_button_style(palette));

    // Folder + Choose-Folder are a single logical group. The outer `row!`
    // width is `Length::Fill` so the group absorbs the residual horizontal
    // space; the inner spacing differentiates "field + trailing action" from
    // the 8-pt separation between columns.
    let folder_row = row![folder_input, choose_button]
        .spacing(FOLDER_CONTROL_SPACING)
        .align_y(Vertical::Center)
        .width(Length::FillPortion(1))
        .height(Length::Shrink);

    let filename_field = text_input(L10n::file_name(lang), &filename)
        .on_input(move |value| SettingsMessage::ThreadFilenameChanged(thread_id, value))
        .size(Pixels(PATH_FIELD_FONT_SIZE))
        .width(Length::Fixed(FILE_NAME_WIDTH))
        .style(settings_field_style(palette));

    let remove_button = button(text(REMOVE_GLYPH).size(Pixels(REMOVE_GLYPH_FONT_SIZE)))
        .on_press_maybe(can_remove.then_some(SettingsMessage::ThreadRemoved(thread_id)))
        .width(Length::Fixed(DELETE_BUTTON_WIDTH))
        .style(settings_remove_button_style(palette));

    row![name_field, folder_row, filename_field, remove_button]
        .spacing(COLUMN_SPACING)
        .align_y(Vertical::Center)
        .width(Length::Fill)
        .into()
}

// --- Layout contracts ------------------------------------------------------
// Keep the width constants together so a drift against the Mac reference is
// caught in one place.

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};

    fn sample_palette() -> SettingsPalette {
        TraceTheme::for_preset(ThemePreset::Light).settings
    }

    fn fixture(name: &str, target_file: &str, order: i32) -> ThreadConfig {
        ThreadConfig::new(name, target_file, None, order)
    }

    #[test]
    fn card_renders_with_single_thread() {
        // At `MINIMUM_COUNT` the remove button must still render (with the
        // disabled style); the card itself must build across every language
        // and every palette so the disabled branch is exercised.
        let threads = vec![fixture("Solo", "Solo.md", 0)];
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for preset in [
                ThemePreset::Light,
                ThemePreset::Dark,
                ThemePreset::Paper,
                ThemePreset::Dune,
            ] {
                let palette = TraceTheme::for_preset(preset).settings;
                let _element: Element<'_, SettingsMessage> =
                    threads_card(palette, lang, &threads, "");
            }
        }
    }

    #[test]
    fn card_renders_at_maximum_capacity() {
        // At `MAXIMUM_COUNT` the Add button must still render (with the
        // disabled style). Guards against a future regression that makes the
        // body iterator unbalanced when the cap is hit.
        let palette = sample_palette();
        let threads: Vec<ThreadConfig> = (0..ThreadConfig::MAXIMUM_COUNT)
            .map(|i| fixture(&format!("T{i}"), &format!("T{i}.md"), i as i32))
            .collect();
        let _element: Element<'_, SettingsMessage> =
            threads_card(palette, Language::En, &threads, "");
    }

    #[test]
    fn card_renders_with_mixed_absolute_and_relative_target_files() {
        // The view layer splits `target_file` into folder + filename on
        // every render; mix absolute and relative paths so the parse branch
        // is exercised against both shapes.
        let palette = sample_palette();
        let threads = vec![
            fixture("A", "notes.md", 0),
            fixture("B", "Projects/notes.md", 1),
            fixture("C", "/Users/x/Vault/Logs/daily.md", 2),
        ];
        let _element: Element<'_, SettingsMessage> =
            threads_card(palette, Language::Zh, &threads, "/Users/x/Vault");
    }

    #[test]
    fn card_preserves_shadow_order_across_view_passes() {
        // The sort step collects references into a fresh Vec — the physical
        // order of the caller's slice must be untouched so the view pass is
        // pure.
        let palette = sample_palette();
        let threads = vec![
            fixture("B", "B.md", 2),
            fixture("A", "A.md", 0),
            fixture("C", "C.md", 1),
        ];
        let before_order: Vec<i32> = threads.iter().map(|t| t.order).collect();
        let _element: Element<'_, SettingsMessage> =
            threads_card(palette, Language::En, &threads, "");
        let after_order: Vec<i32> = threads.iter().map(|t| t.order).collect();
        assert_eq!(before_order, after_order);
    }

    #[test]
    fn layout_constants_match_mac_reference() {
        // Mac `ThreadConfigRow`: HStack(spacing: 8), .frame(width: 100) on
        // name, .frame(minWidth: 380) on folder, .frame(width: 120) on
        // filename. Lock the constants so a drift is caught at test time.
        assert_eq!(COLUMN_SPACING, 8.0);
        assert_eq!(NAME_WIDTH, 100.0);
        assert_eq!(FOLDER_MIN_WIDTH, 380.0);
        assert_eq!(FILE_NAME_WIDTH, 120.0);
    }
}

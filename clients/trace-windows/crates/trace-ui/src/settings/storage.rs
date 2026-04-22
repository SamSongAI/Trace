//! Row factories for the Storage card.
//!
//! Phase 12 sub-task 4 layers the per-write-mode rows (vault path, daily
//! folder, filename format, entry format, inbox vault path) on top of the
//! sub-task 3 write-mode row. Each factory is a thin builder that wires a
//! single iced widget onto a [`crate::settings::widgets::setting_row`] shell,
//! so the `storage_card` dispatcher can pick the right subset based on
//! [`trace_core::WriteMode`] without the main `settings/mod.rs` file
//! ballooning.
//!
//! The factories stay message-generic so they can be unit-tested against a
//! lightweight placeholder message type (`TestMsg`) without dragging in the
//! full `SettingsMessage` enum. Production callers pass closures that wrap
//! each change in the matching `SettingsMessage` variant.
//!
//! # Pick-list wrappers
//!
//! iced's [`iced::widget::pick_list`] takes a `T: ToString + PartialEq +
//! Clone` option type. We can't impl `Display` on the `trace-core` enums
//! directly because the filename preview depends on the current locale (and
//! the entry-format label depends on the active UI language), both of which
//! are UI-layer context. The module defines two small wrapper types
//! ([`DailyFileDateFormatOption`], [`EntryThemeOption`]) that carry the
//! variant plus any context needed to render its label, and implement
//! `Display` via the core helpers.

use std::fmt;

use iced::widget::{button, column, pick_list, row, text, text_input};
use iced::{Element, Length, Pixels};
use trace_core::{
    DailyFileDateFormat, EntryTheme, L10n, Language, SettingsPalette, VaultPathValidationIssue,
};

use crate::theme::{settings_field_style, settings_primary_button_style, trace_color_to_iced};

use super::widgets::{setting_row, ROW_HINT_FONT_SIZE, ROW_SPACING};

/// Horizontal spacing between the vault-path text input and its Browse
/// button. Matches Mac `HStack(spacing: 8)` in `SettingsView.swift`'s vault
/// row.
pub const VAULT_ROW_CONTROL_SPACING: f32 = 8.0;
/// Font size of the inline vault-validation warning. Matches Mac
/// `.font(.system(size: 11, weight: .medium))`.
pub const VAULT_WARNING_FONT_SIZE: f32 = 11.0;

/// Wrapper around [`DailyFileDateFormat`] that delegates its `Display` impl
/// to [`DailyFileDateFormat::title`], so iced's `pick_list` can render the
/// full `"{raw}  →  {example}"` label without the UI layer having to format
/// the option strings manually on every view pass.
///
/// Scoped to `pub(super)` because this is a purely presentational adapter
/// consumed only by the Storage card's row factories inside this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DailyFileDateFormatOption(pub DailyFileDateFormat);

impl fmt::Display for DailyFileDateFormatOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `title` already includes the current-date preview.
        f.write_str(&self.0.title())
    }
}

/// Wrapper around ([`EntryTheme`], [`Language`]) that delegates its `Display`
/// impl to [`EntryTheme::title`]. The language is captured at construction
/// time so switching the active language re-renders the pick-list labels.
///
/// Scoped to `pub(super)` for the same reason as
/// [`DailyFileDateFormatOption`] — the newtype is an internal helper of the
/// Storage card, not a `trace-ui` public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EntryThemeOption(pub EntryTheme, pub Language);

impl fmt::Display for EntryThemeOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.title(self.1))
    }
}

/// Builds the vault-path row used by Dimension mode: a labeled text input
/// with a matching Browse button and an inline validation warning rendered
/// beneath the input when the stored path fails classification.
///
/// `on_change` fires on every keystroke; `on_browse` fires when the Browse
/// button is pressed. The caller is responsible for kicking off the async
/// folder picker (typically via `iced::Task::perform` against
/// `rfd::AsyncFileDialog`) in response to `on_browse`.
pub fn vault_path_row<'a, Message: Clone + 'a>(
    palette: SettingsPalette,
    lang: Language,
    value: &'a str,
    issue: Option<VaultPathValidationIssue>,
    on_change: impl Fn(String) -> Message + 'a,
    on_browse: Message,
) -> Element<'a, Message> {
    vault_like_row(
        palette,
        L10n::vault(lang),
        Some(L10n::vault_hint_dimension(lang)),
        lang,
        "/Users/you/MyVault",
        value,
        issue,
        on_change,
        on_browse,
    )
}

/// Builds the inbox-vault-path row used by File mode. Identical shape to
/// [`vault_path_row`] but keyed against a different L10n hint and a
/// Windows-friendly `C:\\Users\\you\\Documents`-style placeholder that
/// mirrors the Mac reference's `/Users/you/Documents` sample.
pub fn inbox_vault_path_row<'a, Message: Clone + 'a>(
    palette: SettingsPalette,
    lang: Language,
    value: &'a str,
    issue: Option<VaultPathValidationIssue>,
    on_change: impl Fn(String) -> Message + 'a,
    on_browse: Message,
) -> Element<'a, Message> {
    vault_like_row(
        palette,
        L10n::vault(lang),
        Some(L10n::vault_hint_file(lang)),
        lang,
        "/Users/you/Documents",
        value,
        issue,
        on_change,
        on_browse,
    )
}

/// Shared implementation of the two vault-path rows. Keeps the layout code in
/// one place so Dimension and File mode pick up the same visual rhythm — the
/// Mac reference also shares its `SettingRow` body between the two modes.
#[allow(clippy::too_many_arguments)]
fn vault_like_row<'a, Message: Clone + 'a>(
    palette: SettingsPalette,
    label: &'a str,
    hint: Option<&'a str>,
    lang: Language,
    placeholder: &'a str,
    value: &'a str,
    issue: Option<VaultPathValidationIssue>,
    on_change: impl Fn(String) -> Message + 'a,
    on_browse: Message,
) -> Element<'a, Message> {
    let field = text_input(placeholder, value)
        .on_input(on_change)
        .width(Length::Fill)
        .style(settings_field_style(palette));

    let browse = button(text(L10n::browse(lang)))
        .on_press(on_browse)
        .style(settings_primary_button_style(palette));

    let input_row = row![field, browse]
        .spacing(VAULT_ROW_CONTROL_SPACING)
        .width(Length::Fill);

    let control: Element<'a, Message> = if let Some(issue) = issue {
        let warning = text(vault_issue_message(issue, lang))
            .size(Pixels(VAULT_WARNING_FONT_SIZE))
            .color(trace_color_to_iced(palette.warning_text));
        column![input_row, warning]
            .spacing(ROW_SPACING)
            .width(Length::Fill)
            .into()
    } else {
        input_row.into()
    };

    setting_row(palette, label, hint, control)
}

/// Builds the Daily-folder-name row: a single text input used only in
/// Dimension mode. Accepts a free-form folder name; the writer layer already
/// falls back to `"Daily"` when the stored value is blank.
pub fn daily_folder_row<'a, Message: Clone + 'a>(
    palette: SettingsPalette,
    lang: Language,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let field = text_input("Daily", value)
        .on_input(on_change)
        .width(Length::Fill)
        .style(settings_field_style(palette));

    setting_row(
        palette,
        L10n::daily_folder(lang),
        Some(L10n::daily_folder_hint(lang)),
        field.into(),
    )
}

/// Builds the filename-format row: a picker listing every
/// [`DailyFileDateFormat`] variant with its ICU raw value and an example
/// preview. Used only in Dimension mode.
pub fn file_name_format_row<'a, Message: Clone + 'a>(
    palette: SettingsPalette,
    lang: Language,
    selected: DailyFileDateFormat,
    on_change: impl Fn(DailyFileDateFormat) -> Message + 'a,
) -> Element<'a, Message> {
    let options: Vec<DailyFileDateFormatOption> = DailyFileDateFormat::ALL
        .iter()
        .copied()
        .map(DailyFileDateFormatOption)
        .collect();
    let picker = pick_list(
        options,
        Some(DailyFileDateFormatOption(selected)),
        move |option| on_change(option.0),
    )
    .width(Length::Fill)
    .text_size(Pixels(ROW_HINT_FONT_SIZE));

    setting_row(palette, L10n::file_name_format(lang), None, picker.into())
}

/// Builds the entry-format row: a picker listing every [`EntryTheme`]
/// variant. Used only in Dimension mode. `lang` is threaded through so the
/// picker labels localize to the same language as the surrounding card.
pub fn entry_format_row<'a, Message: Clone + 'a>(
    palette: SettingsPalette,
    lang: Language,
    selected: EntryTheme,
    on_change: impl Fn(EntryTheme) -> Message + 'a,
) -> Element<'a, Message> {
    let options: Vec<EntryThemeOption> = EntryTheme::ALL
        .iter()
        .copied()
        .map(|theme| EntryThemeOption(theme, lang))
        .collect();
    let picker = pick_list(
        options,
        Some(EntryThemeOption(selected, lang)),
        move |option| on_change(option.0),
    )
    .width(Length::Fill)
    .text_size(Pixels(ROW_HINT_FONT_SIZE));

    setting_row(palette, L10n::entry_format(lang), None, picker.into())
}

/// Dispatches a [`VaultPathValidationIssue`] to the matching L10n string.
///
/// Kept in the UI layer rather than `trace-core` because the message copy is
/// purely presentational — the core enum carries only the classification.
/// Matches the Mac reference's `VaultPathValidationIssue.message` computed
/// property.
pub fn vault_issue_message(issue: VaultPathValidationIssue, lang: Language) -> &'static str {
    match issue {
        VaultPathValidationIssue::Empty => L10n::vault_empty(lang),
        VaultPathValidationIssue::DoesNotExist => L10n::vault_not_exist(lang),
        VaultPathValidationIssue::NotDirectory => L10n::vault_not_directory(lang),
        VaultPathValidationIssue::NotWritable => L10n::vault_not_writable(lang),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};

    /// Distinct, trivial message type so the factory tests don't couple to
    /// `SettingsMessage`. The variants carry their respective payloads so
    /// the `impl Fn(T) -> Message` callbacks type-check against the real
    /// factory signatures; none of the payloads are inspected in-test.
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum TestMsg {
        VaultChanged(String),
        BrowseVault,
        InboxChanged(String),
        BrowseInbox,
        FolderChanged(String),
        FormatChanged(DailyFileDateFormat),
        ThemeChanged(EntryTheme),
    }

    fn sample_palette() -> SettingsPalette {
        TraceTheme::for_preset(ThemePreset::Light).settings
    }

    #[test]
    fn daily_file_date_format_option_display_matches_core_title() {
        for preset in DailyFileDateFormat::ALL {
            assert_eq!(
                DailyFileDateFormatOption(preset).to_string(),
                preset.title()
            );
        }
    }

    #[test]
    fn entry_theme_option_display_matches_core_title_per_language() {
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for theme in EntryTheme::ALL {
                assert_eq!(EntryThemeOption(theme, lang).to_string(), theme.title(lang));
            }
        }
    }

    #[test]
    fn vault_issue_message_covers_every_variant_for_every_language() {
        // A missing arm would make the helper return an empty string or fail
        // to compile; the assertion pins the non-empty contract for every
        // (variant, lang) pair.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for issue in [
                VaultPathValidationIssue::Empty,
                VaultPathValidationIssue::DoesNotExist,
                VaultPathValidationIssue::NotDirectory,
                VaultPathValidationIssue::NotWritable,
            ] {
                let message = vault_issue_message(issue, lang);
                assert!(!message.is_empty(), "{issue:?}@{lang:?} empty");
            }
        }
    }

    #[test]
    fn vault_path_row_builds_without_issue() {
        let palette = sample_palette();
        let _row: Element<'_, TestMsg> = vault_path_row(
            palette,
            Language::Zh,
            "C:/vault",
            None,
            TestMsg::VaultChanged,
            TestMsg::BrowseVault,
        );
    }

    #[test]
    fn vault_path_row_builds_with_every_issue_variant() {
        // Each classification must produce a renderable element — guards
        // against an iced widget graph that only compiles for the happy path.
        let palette = sample_palette();
        for issue in [
            VaultPathValidationIssue::Empty,
            VaultPathValidationIssue::DoesNotExist,
            VaultPathValidationIssue::NotDirectory,
            VaultPathValidationIssue::NotWritable,
        ] {
            let _row: Element<'_, TestMsg> = vault_path_row(
                palette,
                Language::En,
                "",
                Some(issue),
                TestMsg::VaultChanged,
                TestMsg::BrowseVault,
            );
        }
    }

    #[test]
    fn inbox_vault_path_row_builds_for_every_language() {
        let palette = sample_palette();
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            let _row: Element<'_, TestMsg> = inbox_vault_path_row(
                palette,
                lang,
                "",
                Some(VaultPathValidationIssue::Empty),
                TestMsg::InboxChanged,
                TestMsg::BrowseInbox,
            );
        }
    }

    #[test]
    fn daily_folder_row_builds_with_and_without_value() {
        let palette = sample_palette();
        let _empty: Element<'_, TestMsg> =
            daily_folder_row(palette, Language::Zh, "", TestMsg::FolderChanged);
        let _filled: Element<'_, TestMsg> =
            daily_folder_row(palette, Language::Zh, "DayNotes", TestMsg::FolderChanged);
    }

    #[test]
    fn file_name_format_row_builds_for_every_preset() {
        let palette = sample_palette();
        for preset in DailyFileDateFormat::ALL {
            let _row: Element<'_, TestMsg> =
                file_name_format_row(palette, Language::Zh, preset, TestMsg::FormatChanged);
        }
    }

    #[test]
    fn entry_format_row_builds_for_every_theme_and_language() {
        let palette = sample_palette();
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for theme in EntryTheme::ALL {
                let _row: Element<'_, TestMsg> =
                    entry_format_row(palette, lang, theme, TestMsg::ThemeChanged);
            }
        }
    }

    #[test]
    fn vault_row_spacing_matches_mac_reference() {
        // Mac `HStack(spacing: 8)` in the vault row — lock the constant so
        // a drift is caught at test time.
        assert_eq!(VAULT_ROW_CONTROL_SPACING, 8.0);
        // Mac `.font(.system(size: 11, weight: .medium))` on the warning.
        assert_eq!(VAULT_WARNING_FONT_SIZE, 11.0);
    }
}

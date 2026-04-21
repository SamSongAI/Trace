//! `SettingsApp` state, `SettingsMessage` enum, and `update/view/theme/subscription`
//! functions driving the settings window of the Trace client.
//!
//! Phase 12 sub-task 1 scaffolds the skeleton only: a second iced window
//! routed via [`iced::daemon`] that renders a single scrollable column with
//! the localized "Settings" title and no content. Subsequent sub-tasks layer
//! the 7 Cards (Language, Theme, Storage, Quick Sections, Shortcuts, System,
//! …) on top of this foundation.
//!
//! The module intentionally mirrors the `CaptureApp` pattern:
//!
//! * `SettingsApp` holds the mutable state plus a cached `iced::Theme` so the
//!   per-frame `theme()` call stays cheap.
//! * `settings_update` / `settings_view` / `settings_theme` / `settings_subscription`
//!   are free functions that match iced's builder API and are trivial to
//!   unit-test without spinning up the iced runtime.
//! * `AppSettings` is shared with [`crate::app::CaptureApp`] via `Arc` so the
//!   two windows see a consistent view of the user's preferences. Mutation
//!   through the settings window will land in later sub-tasks.
//!
//! The window itself is opened by [`crate::app::CaptureApp`] in response to
//! [`crate::app::Message::SettingsRequested`]; see `crates/trace-app` for the
//! top-level `iced::daemon` wiring that routes messages between the two
//! windows.

pub mod storage;
pub mod tiles;
pub mod widgets;

use std::sync::Arc;

use iced::widget::{column, container, row, scrollable, text};
use iced::{window, Element, Length, Pixels, Size, Subscription, Task, Theme};
use trace_core::{
    AppSettings, DailyFileDateFormat, EntryTheme, L10n, Language, ThemePreset, TraceTheme,
    WriteMode,
};

use crate::theme::to_iced_theme;

/// Default logical width of the settings window. Matches Mac
/// `SettingsView.swift` default frame width.
pub const DEFAULT_SETTINGS_WINDOW_WIDTH: f32 = 720.0;
/// Default logical height of the settings window.
pub const DEFAULT_SETTINGS_WINDOW_HEIGHT: f32 = 640.0;
/// Minimum width the user can resize the settings window down to. Matches
/// Mac `SettingsView.swift` `minWidth` of 640 pt.
pub const MIN_SETTINGS_WINDOW_WIDTH: f32 = 640.0;
/// Minimum height the user can resize the settings window down to. Matches
/// Mac `SettingsView.swift` `minHeight` of 480 pt.
pub const MIN_SETTINGS_WINDOW_HEIGHT: f32 = 480.0;
/// Vertical padding around the scrollable card stack. Sub-task 2 onwards adds
/// the card shell inside this padding.
pub const SETTINGS_OUTER_PADDING: u16 = 24;
/// Font size of the settings window title. Matches Mac `SettingsView.swift`
/// 22 pt title.
pub const SETTINGS_TITLE_FONT_SIZE: f32 = 22.0;

/// Returns the iced [`window::Settings`] for the settings window.
///
/// Split out as a free function so the `trace-app` crate can pass it
/// directly to `iced::window::open` and tests can compare the resulting
/// dimensions against the Mac `SettingsView.swift` defaults without running
/// the iced runtime. Unlike the capture panel, the settings window uses the
/// native decorations and stays on the normal window layer — it is a
/// conventional preferences sheet, not a floating capture panel.
pub fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(
            DEFAULT_SETTINGS_WINDOW_WIDTH,
            DEFAULT_SETTINGS_WINDOW_HEIGHT,
        ),
        min_size: Some(Size::new(
            MIN_SETTINGS_WINDOW_WIDTH,
            MIN_SETTINGS_WINDOW_HEIGHT,
        )),
        resizable: true,
        decorations: true,
        transparent: false,
        ..window::Settings::default()
    }
}

/// Messages consumed by [`settings_update`].
///
/// Sub-task 3 wires the first three cards (Language, Theme, Storage → Write
/// Mode). Sub-task 4 layers the per-write-mode Storage rows on top: vault /
/// inbox paths (with Browse buttons whose folder-picker result flows back
/// through [`VaultBrowseChose`] / [`InboxVaultBrowseChose`]), the Daily
/// folder name, the Daily file-name format, and the Daily entry format.
///
/// All new variants still only touch shadow fields on [`SettingsApp`] —
/// persistence to [`AppSettings`] lands in sub-task 8.
///
/// [`LanguageChanged`]: SettingsMessage::LanguageChanged
/// [`ThemePresetChanged`]: SettingsMessage::ThemePresetChanged
/// [`WriteModeChanged`]: SettingsMessage::WriteModeChanged
/// [`VaultBrowseChose`]: SettingsMessage::VaultBrowseChose
/// [`InboxVaultBrowseChose`]: SettingsMessage::InboxVaultBrowseChose
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    /// Fired when the user changes the active language via the Language card.
    /// Updates the shadow [`SettingsApp::language`] field so the rest of the
    /// settings view localizes immediately without a round-trip through
    /// [`AppSettings`].
    LanguageChanged(Language),
    /// Fired when the user picks a different theme preset via the Theme card.
    /// Updates the shadow [`SettingsApp::theme_preset`] and rebuilds the
    /// cached iced theme so the settings window itself previews the selection
    /// in-place.
    ThemePresetChanged(ThemePreset),
    /// Fired when the user picks a different write mode via the Storage card.
    /// Updates the shadow [`SettingsApp::write_mode`] field without touching
    /// the shared [`AppSettings`] snapshot.
    WriteModeChanged(WriteMode),
    /// Fired on every keystroke in the Dimension-mode vault path text
    /// input. Updates [`SettingsApp::vault_path`].
    VaultPathChanged(String),
    /// Fired when the user clicks the Dimension-mode vault Browse button.
    /// `settings_update` reacts by kicking off an `rfd::AsyncFileDialog`
    /// folder picker via `iced::Task::perform` and emits a follow-up
    /// [`VaultBrowseChose`](SettingsMessage::VaultBrowseChose) with the
    /// selected path (or `None` if the user cancelled).
    BrowseVaultRequested,
    /// Follow-up to [`BrowseVaultRequested`](SettingsMessage::BrowseVaultRequested)
    /// carrying the picker's result. A `Some(path)` updates
    /// [`SettingsApp::vault_path`]; a `None` (cancelled) is a no-op.
    VaultBrowseChose(Option<String>),
    /// Fired on every keystroke in the File-mode inbox-vault path text
    /// input. Updates [`SettingsApp::inbox_vault_path`].
    InboxVaultPathChanged(String),
    /// Fired when the user clicks the File-mode inbox-vault Browse button.
    /// Mirrors [`BrowseVaultRequested`](SettingsMessage::BrowseVaultRequested)
    /// for the inbox vault.
    BrowseInboxVaultRequested,
    /// Follow-up to [`BrowseInboxVaultRequested`](SettingsMessage::BrowseInboxVaultRequested)
    /// carrying the picker's result. A `Some(path)` updates
    /// [`SettingsApp::inbox_vault_path`]; a `None` is a no-op.
    InboxVaultBrowseChose(Option<String>),
    /// Fired on every keystroke in the Daily folder name text input.
    /// Updates [`SettingsApp::daily_folder_name`].
    DailyFolderNameChanged(String),
    /// Fired when the user picks a different [`DailyFileDateFormat`] variant
    /// via the File Name Format picker. Updates
    /// [`SettingsApp::daily_file_date_format`].
    DailyFileDateFormatChanged(DailyFileDateFormat),
    /// Fired when the user picks a different [`EntryTheme`] variant via the
    /// Entry Format picker. Updates
    /// [`SettingsApp::daily_entry_theme_preset`].
    DailyEntryThemePresetChanged(EntryTheme),
}

/// Mutable application state for the settings window.
///
/// The struct is intentionally data-oriented so `settings_update` can be
/// tested without running iced's event loop. Field parity with
/// [`crate::app::CaptureApp`]:
///
/// * `theme` — full [`TraceTheme`] snapshot for the current preset.
/// * `iced_theme` — cached [`iced::Theme`] derived from `theme`; kept in
///   sync via the [`SettingsApp::set_theme`] helper.
/// * `settings` — shared `AppSettings` snapshot. Wrapped in [`Arc`] so the
///   capture and settings windows can hand the same pointer to writers
///   without deep-cloning the `Vec<ThreadConfig>` / `Vec<NoteSection>`
///   allocations.
/// * `language` — starts as `settings.language` and drives localized
///   strings in `settings_view`. Split out from the `Arc<AppSettings>` so
///   the settings window can preview a language change before the
///   `AppSettings` write lands.
pub struct SettingsApp {
    /// Full theme palette bundle for the currently selected preset.
    pub theme: TraceTheme,
    /// Cached iced [`Theme`] derived from [`SettingsApp::theme`]. Refreshed
    /// atomically via [`SettingsApp::set_theme`].
    ///
    /// **Invariant**: `iced_theme == to_iced_theme(&theme)`.
    pub iced_theme: Theme,
    /// Shared settings snapshot. Mutated through the settings window is a
    /// later sub-task; sub-task 1 only reads from this handle.
    pub settings: Arc<AppSettings>,
    /// Active language for localized view strings. Seeded from
    /// `settings.language`; later sub-tasks wire a language picker to this
    /// field.
    pub language: Language,
    /// Active theme preset shown in the Theme card. Seeded from
    /// `settings.app_theme_preset`; mutated in-place by
    /// [`SettingsMessage::ThemePresetChanged`] so the settings window can
    /// preview a new preset before the write back to [`AppSettings`] lands
    /// (sub-task 8).
    pub theme_preset: ThemePreset,
    /// Active write mode shown in the Storage → Write Mode row. Seeded from
    /// `settings.note_write_mode`; mutated in-place by
    /// [`SettingsMessage::WriteModeChanged`] so the Storage card can toggle
    /// between `Dimension` / `Thread` / `File` independently of the shared
    /// [`AppSettings`] snapshot.
    pub write_mode: WriteMode,
    /// Active Dimension-mode vault path shown in the Storage card. Seeded
    /// from `settings.vault_path`; mutated in-place by
    /// [`SettingsMessage::VaultPathChanged`] / [`SettingsMessage::VaultBrowseChose`]
    /// so the inline validation hint tracks edits in real time.
    pub vault_path: String,
    /// Active File-mode inbox vault path shown in the Storage card. Seeded
    /// from `settings.inbox_vault_path`.
    pub inbox_vault_path: String,
    /// Active Daily folder name. Seeded from `settings.daily_folder_name`.
    pub daily_folder_name: String,
    /// Active Daily file-name format. Seeded from
    /// `settings.daily_file_date_format` via
    /// [`DailyFileDateFormat::resolved_from_raw`] so an unknown on-disk
    /// value falls back to the [`DailyFileDateFormat::ChineseFull`] default,
    /// matching Mac's `DailyFileDateFormat.resolved(fromStored:)`.
    pub daily_file_date_format: DailyFileDateFormat,
    /// Active Daily entry theme preset. Seeded from
    /// `settings.daily_entry_theme_preset`.
    pub daily_entry_theme_preset: EntryTheme,
}

impl SettingsApp {
    /// Builds a fresh [`SettingsApp`] from a resolved [`TraceTheme`] and a
    /// shared [`AppSettings`] handle.
    ///
    /// `settings.language` is copied into the local `language` field so the
    /// settings window can render immediately without waiting for the first
    /// message dispatch. Callers must keep the `Arc<AppSettings>` in sync if
    /// other windows mutate the underlying settings.
    pub fn new(theme: TraceTheme, settings: Arc<AppSettings>) -> Self {
        let iced_theme = to_iced_theme(&theme);
        let language = settings.language;
        let theme_preset = settings.app_theme_preset;
        let write_mode = settings.note_write_mode;
        let vault_path = settings.vault_path.clone();
        let inbox_vault_path = settings.inbox_vault_path.clone();
        let daily_folder_name = settings.daily_folder_name.clone();
        // On-disk value is a raw ICU string; resolve to a preset variant via
        // the Mac-compatible fallback rule so an unknown string decodes to
        // `ChineseFull` rather than silently mismatching the picker.
        let daily_file_date_format =
            DailyFileDateFormat::resolved_from_raw(&settings.daily_file_date_format);
        let daily_entry_theme_preset = settings.daily_entry_theme_preset;
        Self {
            theme,
            iced_theme,
            settings,
            language,
            theme_preset,
            write_mode,
            vault_path,
            inbox_vault_path,
            daily_folder_name,
            daily_file_date_format,
            daily_entry_theme_preset,
        }
    }

    /// Replaces the active [`TraceTheme`] and refreshes the cached
    /// [`Self::iced_theme`] atomically. Mirrors
    /// [`crate::app::CaptureApp::set_theme`].
    pub fn set_theme(&mut self, theme: TraceTheme) {
        self.iced_theme = to_iced_theme(&theme);
        self.theme = theme;
    }
}

/// Mutates the supplied [`SettingsApp`] in response to a [`SettingsMessage`]
/// and returns an [`iced::Task`] describing any follow-up effect.
///
/// Sub-task 1 handled only [`SettingsMessage::LanguageChanged`]; sub-task 3
/// added theme-preset and write-mode branches; sub-task 4 wires the full
/// per-write-mode Storage card: free-text edits land directly on shadow
/// fields, and the two Browse buttons fire `Task::perform` against
/// [`rfd::AsyncFileDialog::pick_folder`] so the picker runs outside the iced
/// view thread. The picker's `Option<String>` flows back through
/// [`SettingsMessage::VaultBrowseChose`] / [`SettingsMessage::InboxVaultBrowseChose`].
pub fn settings_update(state: &mut SettingsApp, message: SettingsMessage) -> Task<SettingsMessage> {
    match message {
        SettingsMessage::LanguageChanged(lang) => {
            state.language = lang;
            Task::none()
        }
        SettingsMessage::ThemePresetChanged(preset) => {
            // Swap the shadow field first, then rebuild the full
            // [`TraceTheme`] so the cached iced theme stays in lock-step.
            // `set_theme` handles the iced-theme recompute so this branch
            // only needs to remember the preset and hand off to the helper.
            state.theme_preset = preset;
            state.set_theme(TraceTheme::for_preset(preset));
            Task::none()
        }
        SettingsMessage::WriteModeChanged(mode) => {
            // The write-mode shadow field feeds the view layer. Sub-task 8
            // will wire the actual persistence.
            state.write_mode = mode;
            Task::none()
        }
        SettingsMessage::VaultPathChanged(path) => {
            state.vault_path = path;
            Task::none()
        }
        SettingsMessage::BrowseVaultRequested => pick_folder_task(SettingsMessage::VaultBrowseChose),
        SettingsMessage::VaultBrowseChose(Some(path)) => {
            state.vault_path = path;
            Task::none()
        }
        // User cancelled the picker — leave the current path untouched.
        SettingsMessage::VaultBrowseChose(None) => Task::none(),
        SettingsMessage::InboxVaultPathChanged(path) => {
            state.inbox_vault_path = path;
            Task::none()
        }
        SettingsMessage::BrowseInboxVaultRequested => {
            pick_folder_task(SettingsMessage::InboxVaultBrowseChose)
        }
        SettingsMessage::InboxVaultBrowseChose(Some(path)) => {
            state.inbox_vault_path = path;
            Task::none()
        }
        SettingsMessage::InboxVaultBrowseChose(None) => Task::none(),
        SettingsMessage::DailyFolderNameChanged(name) => {
            state.daily_folder_name = name;
            Task::none()
        }
        SettingsMessage::DailyFileDateFormatChanged(format) => {
            state.daily_file_date_format = format;
            Task::none()
        }
        SettingsMessage::DailyEntryThemePresetChanged(theme) => {
            state.daily_entry_theme_preset = theme;
            Task::none()
        }
    }
}

/// Kicks off an `rfd::AsyncFileDialog::pick_folder` picker off the iced view
/// thread and wraps the selection in `wrap`. A cancellation surfaces as
/// `wrap(None)`. Extracted so the Dimension-vault and File-inbox browse
/// requests share a single code path.
fn pick_folder_task(
    wrap: fn(Option<String>) -> SettingsMessage,
) -> Task<SettingsMessage> {
    // `AsyncFileDialog::pick_folder` returns a `Future<Output = Option<FileHandle>>`.
    // We flatten to `Option<String>` so the follow-up `SettingsMessage` carries
    // plain, owned UTF-8 and the rest of the app never has to know about
    // `rfd`'s `FileHandle` wrapper.
    let future = async {
        rfd::AsyncFileDialog::new()
            .pick_folder()
            .await
            .map(|handle| handle.path().to_string_lossy().into_owned())
    };
    Task::perform(future, wrap)
}

// 下面四个布局常量仅在 settings 模块内部消费(构建 card 列、chip 行、tile
// 列/行间距),以及同模块的回归测试。改为模块私有 `const`,避免 `pub` 暗
// 示它们是 crate 级公共 API —— 若未来别的 card 也用到,就地提级到
// `pub(crate)` 即可。
/// Spacing (in pixels) between stacked settings cards inside the scrollable
/// column. Matches Mac `SettingsView.swift`'s `VStack(spacing: 18)`.
const SETTINGS_CARD_STACK_SPACING: f32 = 18.0;
/// Spacing between adjacent language chips inside the Language card row.
const LANGUAGE_CHIP_SPACING: f32 = 8.0;
/// Vertical spacing between theme-preset tiles inside the Theme card column.
const THEME_TILE_SPACING: f32 = 8.0;
/// Spacing between adjacent write-mode tiles inside the Storage card row.
const WRITE_MODE_TILE_SPACING: f32 = 8.0;

/// Renders the settings window. Returns an [`Element`] ready for use in an
/// `iced::daemon(...).view(...)` router.
///
/// Sub-task 3 layers the first three cards (Language, Theme, Storage) on top
/// of the sub-task 1 scrollable shell. Every subsequent sub-task appends more
/// cards to [`build_cards`] without reshaping the outer layout.
pub fn settings_view(state: &SettingsApp) -> Element<'_, SettingsMessage> {
    let title = text(L10n::settings(state.language)).size(Pixels(SETTINGS_TITLE_FONT_SIZE));

    let cards = build_cards(state);

    let body = column![title, cards]
        .spacing(SETTINGS_CARD_STACK_SPACING)
        .width(Length::Fill);

    let scrollable_body = scrollable(
        container(body)
            .padding(SETTINGS_OUTER_PADDING)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    container(scrollable_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the stacked card column for [`settings_view`]. Split out as a
/// private helper so the card-per-sub-task wiring stays local without making
/// the outer `settings_view` balloon. Returns an [`Element`] rather than a
/// concrete column so future sub-tasks can swap in a more complex container
/// without touching the caller.
fn build_cards(state: &SettingsApp) -> Element<'_, SettingsMessage> {
    let palette = state.theme.settings;

    column![
        language_card(state, palette),
        theme_card(state, palette),
        storage_card(state, palette),
    ]
    .spacing(SETTINGS_CARD_STACK_SPACING)
    .width(Length::Fill)
    .into()
}

/// Builds the Language card: a single row of chips covering
/// `SystemDefault` + the three native locales.
fn language_card<'a>(
    state: &'a SettingsApp,
    palette: trace_core::SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;
    let chips: [(Language, &'a str); 4] = [
        (Language::SystemDefault, L10n::language_system_default(lang)),
        // Every locale renders with its endonym so the chip reads in its own
        // script. `native_display_name` returns `None` only for
        // `Language::SystemDefault`, which is handled above — the unwrap on
        // these three arms is total.
        (
            Language::Zh,
            Language::Zh
                .native_display_name()
                .expect("Zh endonym must exist"),
        ),
        (
            Language::En,
            Language::En
                .native_display_name()
                .expect("En endonym must exist"),
        ),
        (
            Language::Ja,
            Language::Ja
                .native_display_name()
                .expect("Ja endonym must exist"),
        ),
    ];

    let chip_row = row(chips.iter().map(|(variant, label)| {
        tiles::language_chip(
            palette,
            label,
            state.language == *variant,
            SettingsMessage::LanguageChanged(*variant),
        )
    }))
    .spacing(LANGUAGE_CHIP_SPACING)
    .width(Length::Fill);

    widgets::section_card(palette, L10n::language(lang), chip_row.into())
}

/// Builds the Theme card: a vertical list of tiles, one per preset.
fn theme_card<'a>(
    state: &'a SettingsApp,
    palette: trace_core::SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;
    let presets = [
        ThemePreset::Light,
        ThemePreset::Dark,
        ThemePreset::Paper,
        ThemePreset::Dune,
    ];

    let tiles = presets.iter().map(|preset| {
        // 直接使用 `ThemePreset::preview_swatches` helper,避免为了拿色板
        // 而构造整个 `TraceTheme` palette bundle。语义上这是 preset 自身
        // 的属性,与 `title()` / `icon_glyph()` 的访问方式对齐。
        tiles::theme_preset_tile(
            palette,
            preset.title(),
            preset.icon_glyph(),
            preset.preview_swatches(),
            state.theme_preset == *preset,
            SettingsMessage::ThemePresetChanged(*preset),
        )
    });

    let stack = column(tiles)
        .spacing(THEME_TILE_SPACING)
        .width(Length::Fill);

    widgets::section_card(palette, L10n::theme(lang), stack.into())
}

/// Builds the Storage card. The top row always shows the three Write Mode
/// tiles; the rows underneath are swapped in based on the active
/// [`WriteMode`]:
///
/// * `Dimension` — vault path (with inline validation), daily folder name,
///   file-name format, and entry format.
/// * `File` — inbox vault path (with inline validation).
/// * `Thread` — no additional rows (thread configs are their own card).
///
/// Mirrors Mac `SettingsView.swift`'s per-mode conditional branches so the
/// two ports show the same fields in the same order.
fn storage_card<'a>(
    state: &'a SettingsApp,
    palette: trace_core::SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;

    let modes = [WriteMode::Dimension, WriteMode::Thread, WriteMode::File];
    let tiles_row = row(modes.iter().map(|mode| {
        tiles::write_mode_tile(
            palette,
            mode.compact_title(lang),
            mode.destination_title(lang),
            mode.icon_glyph(),
            state.write_mode == *mode,
            SettingsMessage::WriteModeChanged(*mode),
        )
    }))
    .spacing(WRITE_MODE_TILE_SPACING)
    .width(Length::Fill);

    let write_mode_row =
        widgets::setting_row(palette, L10n::write_mode(lang), None, tiles_row.into());

    // Start the card body with the write-mode row, then append whatever
    // extra rows the selected write mode calls for.
    let mut body_rows: Vec<Element<'a, SettingsMessage>> = vec![write_mode_row];

    match state.write_mode {
        WriteMode::Dimension => {
            let vault_issue = trace_platform::validate_vault_path(&state.vault_path);
            body_rows.push(storage::vault_path_row(
                palette,
                lang,
                &state.vault_path,
                vault_issue,
                SettingsMessage::VaultPathChanged,
                SettingsMessage::BrowseVaultRequested,
            ));
            body_rows.push(storage::daily_folder_row(
                palette,
                lang,
                &state.daily_folder_name,
                SettingsMessage::DailyFolderNameChanged,
            ));
            body_rows.push(storage::file_name_format_row(
                palette,
                lang,
                state.daily_file_date_format,
                SettingsMessage::DailyFileDateFormatChanged,
            ));
            body_rows.push(storage::entry_format_row(
                palette,
                lang,
                state.daily_entry_theme_preset,
                SettingsMessage::DailyEntryThemePresetChanged,
            ));
        }
        WriteMode::File => {
            let inbox_issue = trace_platform::validate_vault_path(&state.inbox_vault_path);
            body_rows.push(storage::inbox_vault_path_row(
                palette,
                lang,
                &state.inbox_vault_path,
                inbox_issue,
                SettingsMessage::InboxVaultPathChanged,
                SettingsMessage::BrowseInboxVaultRequested,
            ));
        }
        WriteMode::Thread => {
            // No extra rows — thread configs live in their own card in later
            // sub-tasks, mirroring the Mac reference's `ThreadConfigCard`.
        }
    }

    widgets::section_card_with_rows(palette, L10n::storage(lang), body_rows)
}

/// Returns the iced [`Theme`] derived from [`SettingsApp::theme`]. Plumbed
/// through `iced::daemon(...).theme(...)`.
pub fn settings_theme(state: &SettingsApp) -> Theme {
    state.iced_theme.clone()
}

/// Aggregate subscription for the settings window. Sub-task 1 has no live
/// event sources, so the subscription is
/// [`Subscription::none`]. Later sub-tasks (e.g. shortcut recording) will
/// expand this.
pub fn settings_subscription(_state: &SettingsApp) -> Subscription<SettingsMessage> {
    Subscription::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::ThemePreset;

    fn fresh_app() -> SettingsApp {
        SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
        )
    }

    #[test]
    fn settings_app_default_constructs_from_settings_and_theme() {
        let settings = AppSettings {
            language: Language::Zh,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        // Language is mirrored from `AppSettings` so the first render reads
        // the user's preferred locale without waiting for a dispatched
        // message.
        assert_eq!(app.language, Language::Zh);
        assert_eq!(app.theme.preset, ThemePreset::Dark);
        assert_eq!(app.settings.language, Language::Zh);
    }

    #[test]
    fn settings_app_new_seeds_iced_theme_from_trace_theme() {
        let app = fresh_app();
        // The cached iced theme's palette must match a fresh conversion.
        let expected = to_iced_theme(&app.theme);
        assert_eq!(
            settings_theme(&app).palette().background,
            expected.palette().background
        );
    }

    #[test]
    fn set_theme_refreshes_cached_iced_theme() {
        let mut app = fresh_app();
        let dark_bg = settings_theme(&app).palette().background;
        app.set_theme(TraceTheme::for_preset(ThemePreset::Light));
        let light_bg = settings_theme(&app).palette().background;
        assert_ne!(
            dark_bg, light_bg,
            "switching preset must rebuild the cached iced theme"
        );
    }

    #[test]
    fn settings_view_constructs_without_panic() {
        let app = fresh_app();
        // Construct the view tree — proves the widget graph type-checks for
        // each language without running the iced event loop.
        let _element: Element<'_, SettingsMessage> = settings_view(&app);
    }

    #[test]
    fn settings_view_builds_across_all_languages() {
        let mut app = fresh_app();
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            app.language = lang;
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    #[test]
    fn language_changed_updates_state() {
        let mut app = fresh_app();
        assert_eq!(app.language, Language::SystemDefault);
        let _ = settings_update(&mut app, SettingsMessage::LanguageChanged(Language::Zh));
        assert_eq!(app.language, Language::Zh);
        let _ = settings_update(&mut app, SettingsMessage::LanguageChanged(Language::En));
        assert_eq!(app.language, Language::En);
    }

    #[test]
    fn settings_subscription_starts_empty() {
        // Sub-task 1 has no live event sources. We can't inspect the
        // subscription tree directly, but constructing it proves the
        // branch compiles against the current iced 0.14 surface.
        let app = fresh_app();
        let _sub: Subscription<SettingsMessage> = settings_subscription(&app);
    }

    #[test]
    fn default_settings_window_dimensions_match_mac_reference() {
        // Mac `SettingsView.swift` picks 720 x 640 for the default frame.
        // Lock the constants so a drift is caught at test time.
        assert_eq!(DEFAULT_SETTINGS_WINDOW_WIDTH, 720.0);
        assert_eq!(DEFAULT_SETTINGS_WINDOW_HEIGHT, 640.0);
    }

    // --- Sub-task 3 ----------------------------------------------------

    #[test]
    fn settings_app_new_seeds_theme_preset_and_write_mode_from_settings() {
        // Shadow fields must pick up the persisted settings snapshot so the
        // first render reflects the user's last choice without waiting for a
        // message dispatch.
        let persisted = AppSettings {
            app_theme_preset: ThemePreset::Paper,
            note_write_mode: WriteMode::File,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Paper),
            Arc::new(persisted),
        );
        assert_eq!(app.theme_preset, ThemePreset::Paper);
        assert_eq!(app.write_mode, WriteMode::File);
    }

    #[test]
    fn settings_update_theme_preset_changed_refreshes_cached_iced_theme() {
        let mut app = fresh_app();
        let dark_bg = settings_theme(&app).palette().background;
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThemePresetChanged(ThemePreset::Light),
        );
        let light_bg = settings_theme(&app).palette().background;
        assert_eq!(app.theme_preset, ThemePreset::Light);
        assert_eq!(app.theme.preset, ThemePreset::Light);
        assert_ne!(
            dark_bg, light_bg,
            "theme preset change must rebuild the cached iced theme"
        );
    }

    #[test]
    fn settings_update_theme_preset_changed_covers_every_preset() {
        // Cycle through every preset to prove the match arms never miss one.
        let mut app = fresh_app();
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let _ = settings_update(&mut app, SettingsMessage::ThemePresetChanged(preset));
            assert_eq!(app.theme_preset, preset);
            assert_eq!(app.theme.preset, preset);
        }
    }

    #[test]
    fn settings_update_write_mode_changed_updates_shadow_field() {
        let mut app = fresh_app();
        // `AppSettings::default()` uses `WriteMode::Dimension`.
        assert_eq!(app.write_mode, WriteMode::Dimension);
        for mode in [WriteMode::Thread, WriteMode::File, WriteMode::Dimension] {
            let _ = settings_update(&mut app, SettingsMessage::WriteModeChanged(mode));
            assert_eq!(app.write_mode, mode);
        }
    }

    #[test]
    fn settings_update_write_mode_changed_does_not_mutate_shared_settings() {
        // Sub-task 3 only touches the shadow field — persistence to
        // `AppSettings` is wired in sub-task 8. Guard the invariant so a
        // future edit that leaks a write back to the Arc is caught by tests.
        let mut app = fresh_app();
        let original = app.settings.note_write_mode;
        let _ = settings_update(
            &mut app,
            SettingsMessage::WriteModeChanged(WriteMode::Thread),
        );
        assert_eq!(app.settings.note_write_mode, original);
    }

    #[test]
    fn settings_update_language_changed_still_works_after_expansion() {
        // Regression guard: adding new `SettingsMessage` variants must not
        // break the existing LanguageChanged branch.
        let mut app = fresh_app();
        let _ = settings_update(&mut app, SettingsMessage::LanguageChanged(Language::Ja));
        assert_eq!(app.language, Language::Ja);
    }

    #[test]
    fn settings_view_renders_language_theme_storage_cards_without_panic() {
        // The three cards must build across every language × every preset ×
        // every write-mode combination so a drift in the matching logic is
        // caught at test time.
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
                for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
                    let settings = AppSettings {
                        language: lang,
                        app_theme_preset: preset,
                        note_write_mode: mode,
                        ..AppSettings::default()
                    };
                    let app = SettingsApp::new(
                        TraceTheme::for_preset(preset),
                        Arc::new(settings),
                    );
                    let _element: Element<'_, SettingsMessage> = settings_view(&app);
                }
            }
        }
    }

    #[test]
    fn card_stack_spacing_matches_mac_reference() {
        // Mac `SettingsView.swift` uses `VStack(spacing: 18)` for the card
        // stack. Lock the constant so a drift is caught at test time.
        assert_eq!(SETTINGS_CARD_STACK_SPACING, 18.0);
    }

    // --- Sub-task 4 ----------------------------------------------------

    #[test]
    fn settings_app_new_seeds_storage_shadow_fields_from_settings() {
        // All five new storage shadow fields must be hydrated from the
        // persisted `AppSettings` snapshot so the Storage card shows the
        // user's last choices on first paint.
        let persisted = AppSettings {
            vault_path: "C:/vault".into(),
            inbox_vault_path: "C:/inbox".into(),
            daily_folder_name: "DayNotes".into(),
            daily_file_date_format: DailyFileDateFormat::IsoDate.raw_value().to_string(),
            daily_entry_theme_preset: EntryTheme::MarkdownQuote,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.vault_path, "C:/vault");
        assert_eq!(app.inbox_vault_path, "C:/inbox");
        assert_eq!(app.daily_folder_name, "DayNotes");
        // Raw ICU string rehydrates through `resolved_from_raw` so a match
        // with a known preset resolves to that variant exactly.
        assert_eq!(app.daily_file_date_format, DailyFileDateFormat::IsoDate);
        assert_eq!(app.daily_entry_theme_preset, EntryTheme::MarkdownQuote);
    }

    #[test]
    fn settings_app_new_falls_back_to_chinese_full_for_unknown_format() {
        // Mac's `DailyFileDateFormat.resolved(fromStored:)` falls back to
        // `ChineseFull` when the stored ICU string is not a known preset. The
        // Windows port must match so a hand-edited JSON does not land in an
        // unrepresentable `UnknownFormat` state.
        let persisted = AppSettings {
            daily_file_date_format: "not a real format".into(),
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.daily_file_date_format, DailyFileDateFormat::ChineseFull);
    }

    #[test]
    fn settings_update_vault_path_changed_updates_shadow_field() {
        let mut app = fresh_app();
        assert!(app.vault_path.is_empty());
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("C:/new-vault".into()),
        );
        assert_eq!(app.vault_path, "C:/new-vault");
    }

    #[test]
    fn settings_update_vault_browse_chose_some_overwrites_vault_path() {
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultBrowseChose(Some("D:/picked".into())),
        );
        assert_eq!(app.vault_path, "D:/picked");
    }

    #[test]
    fn settings_update_vault_browse_chose_none_is_noop() {
        // User cancelled the picker — the current path must be preserved.
        let mut app = fresh_app();
        app.vault_path = "C:/keep-me".into();
        let _ = settings_update(&mut app, SettingsMessage::VaultBrowseChose(None));
        assert_eq!(app.vault_path, "C:/keep-me");
    }

    #[test]
    fn settings_update_inbox_vault_path_changed_updates_shadow_field() {
        let mut app = fresh_app();
        assert!(app.inbox_vault_path.is_empty());
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultPathChanged("C:/inbox".into()),
        );
        assert_eq!(app.inbox_vault_path, "C:/inbox");
    }

    #[test]
    fn settings_update_inbox_vault_browse_chose_some_overwrites_inbox_path() {
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultBrowseChose(Some("D:/picked-inbox".into())),
        );
        assert_eq!(app.inbox_vault_path, "D:/picked-inbox");
    }

    #[test]
    fn settings_update_inbox_vault_browse_chose_none_is_noop() {
        let mut app = fresh_app();
        app.inbox_vault_path = "C:/keep-inbox".into();
        let _ = settings_update(&mut app, SettingsMessage::InboxVaultBrowseChose(None));
        assert_eq!(app.inbox_vault_path, "C:/keep-inbox");
    }

    #[test]
    fn settings_update_daily_folder_name_changed_updates_shadow_field() {
        let mut app = fresh_app();
        assert_eq!(app.daily_folder_name, "Daily"); // AppSettings default
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFolderNameChanged("Weekly".into()),
        );
        assert_eq!(app.daily_folder_name, "Weekly");
    }

    #[test]
    fn settings_update_daily_file_date_format_changed_cycles_every_preset() {
        // Cycle through every preset so the match arm in `settings_update`
        // is exercised for each variant, not just the first one.
        let mut app = fresh_app();
        for preset in DailyFileDateFormat::ALL {
            let _ = settings_update(
                &mut app,
                SettingsMessage::DailyFileDateFormatChanged(preset),
            );
            assert_eq!(app.daily_file_date_format, preset);
        }
    }

    #[test]
    fn settings_update_daily_entry_theme_preset_changed_cycles_every_variant() {
        let mut app = fresh_app();
        for theme in EntryTheme::ALL {
            let _ = settings_update(
                &mut app,
                SettingsMessage::DailyEntryThemePresetChanged(theme),
            );
            assert_eq!(app.daily_entry_theme_preset, theme);
        }
    }

    #[test]
    fn settings_update_storage_messages_do_not_mutate_shared_settings() {
        // Sub-task 4 still only touches shadow fields. Persistence lands in
        // sub-task 8. Guard every new variant against leaking a write back
        // into the shared `Arc<AppSettings>`.
        let mut app = fresh_app();
        let original_vault = app.settings.vault_path.clone();
        let original_inbox = app.settings.inbox_vault_path.clone();
        let original_daily_folder = app.settings.daily_folder_name.clone();
        let original_date_format = app.settings.daily_file_date_format.clone();
        let original_entry_theme = app.settings.daily_entry_theme_preset;

        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("C:/new".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultPathChanged("C:/new-inbox".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFolderNameChanged("NewDaily".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFileDateFormatChanged(DailyFileDateFormat::SlashDate),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyEntryThemePresetChanged(EntryTheme::PlainTextTimestamp),
        );

        assert_eq!(app.settings.vault_path, original_vault);
        assert_eq!(app.settings.inbox_vault_path, original_inbox);
        assert_eq!(app.settings.daily_folder_name, original_daily_folder);
        assert_eq!(app.settings.daily_file_date_format, original_date_format);
        assert_eq!(app.settings.daily_entry_theme_preset, original_entry_theme);
    }

    #[test]
    fn storage_card_builds_for_every_write_mode_across_all_languages() {
        // Smoke test: rendering the Storage card must succeed for each
        // (language, write_mode) pair so a missing arm in the dispatch logic
        // is caught at test time rather than at first paint.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
                let settings = AppSettings {
                    language: lang,
                    note_write_mode: mode,
                    vault_path: "".into(),       // exercises the Empty issue
                    inbox_vault_path: "".into(), // same for File mode
                    ..AppSettings::default()
                };
                let app = SettingsApp::new(
                    TraceTheme::for_preset(ThemePreset::Dark),
                    Arc::new(settings),
                );
                let _element: Element<'_, SettingsMessage> = settings_view(&app);
            }
        }
    }

    #[test]
    fn storage_card_in_dimension_mode_renders_every_filename_format() {
        // Cycle through the five filename-format presets. A broken picker
        // serialization would have one preset fail to build while the others
        // succeed; the loop guards against that.
        for preset in DailyFileDateFormat::ALL {
            let settings = AppSettings {
                note_write_mode: WriteMode::Dimension,
                daily_file_date_format: preset.raw_value().to_string(),
                ..Default::default()
            };
            let app = SettingsApp::new(
                TraceTheme::for_preset(ThemePreset::Dark),
                Arc::new(settings),
            );
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    #[test]
    fn storage_card_in_dimension_mode_renders_every_entry_theme() {
        // Same regression shape as the filename-format test, for the entry
        // theme picker.
        for theme in EntryTheme::ALL {
            let settings = AppSettings {
                note_write_mode: WriteMode::Dimension,
                daily_entry_theme_preset: theme,
                ..Default::default()
            };
            let app = SettingsApp::new(
                TraceTheme::for_preset(ThemePreset::Dark),
                Arc::new(settings),
            );
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }
}

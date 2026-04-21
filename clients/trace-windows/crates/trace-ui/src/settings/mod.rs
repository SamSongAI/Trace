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

pub mod tiles;
pub mod widgets;

use std::sync::Arc;

use iced::widget::{column, container, row, scrollable, text};
use iced::{window, Element, Length, Pixels, Size, Subscription, Task, Theme};
use trace_core::{AppSettings, L10n, Language, ThemePreset, TraceTheme, WriteMode};

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
/// Mode). The three [`LanguageChanged`]/[`ThemePresetChanged`]/
/// [`WriteModeChanged`] variants only touch shadow fields on
/// [`SettingsApp`] — persistence to [`AppSettings`] lands in sub-task 8,
/// after every card has its full row complement wired up.
///
/// [`LanguageChanged`]: SettingsMessage::LanguageChanged
/// [`ThemePresetChanged`]: SettingsMessage::ThemePresetChanged
/// [`WriteModeChanged`]: SettingsMessage::WriteModeChanged
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
        Self {
            theme,
            iced_theme,
            settings,
            language,
            theme_preset,
            write_mode,
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
/// Sub-task 1 handles only [`SettingsMessage::LanguageChanged`] so tests can
/// confirm state round-trips through the dispatch layer. Later sub-tasks
/// extend the match with card-specific variants.
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
            // The write-mode shadow field only feeds the view layer. Sub-task 4
            // will thread this into the Storage card's vault-path / filename
            // rows so toggling the mode retargets the fields; sub-task 8 wires
            // the actual persistence.
            state.write_mode = mode;
            Task::none()
        }
    }
}

/// Spacing (in pixels) between stacked settings cards inside the scrollable
/// column. Matches Mac `SettingsView.swift`'s `VStack(spacing: 18)`.
pub const SETTINGS_CARD_STACK_SPACING: f32 = 18.0;
/// Spacing between adjacent language chips inside the Language card row.
pub const LANGUAGE_CHIP_SPACING: f32 = 8.0;
/// Vertical spacing between theme-preset tiles inside the Theme card column.
pub const THEME_TILE_SPACING: f32 = 8.0;
/// Spacing between adjacent write-mode tiles inside the Storage card row.
pub const WRITE_MODE_TILE_SPACING: f32 = 8.0;

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

/// Builds the Storage card. Sub-task 3 wires only the Write Mode row — the
/// vault path and filename template rows will be appended in sub-task 4.
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

    let write_mode_row = widgets::setting_row(palette, L10n::write_mode(lang), None, tiles_row.into());

    widgets::section_card(palette, L10n::storage(lang), write_mode_row)
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
}

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

pub mod widgets;

use std::sync::Arc;

use iced::widget::{column, container, scrollable, text, Space};
use iced::{window, Element, Length, Pixels, Size, Subscription, Task, Theme};
use trace_core::{AppSettings, L10n, Language, TraceTheme};

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
/// Sub-task 1 keeps the surface minimal — only the "close" pathway is wired
/// so that tests can verify the message dispatch round-trips. Every later
/// sub-task extends this enum with card-specific variants (e.g. a Language
/// picker change, a Theme preset change, etc.).
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    /// Fired when the user changes the active language. Sub-task 1 plumbs
    /// this through `settings_update` so later sub-tasks can hook in the
    /// real `AppSettings` write path without touching the dispatch layer.
    LanguageChanged(Language),
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
        Self {
            theme,
            iced_theme,
            settings,
            language,
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
    }
}

/// Renders the settings window. Returns an [`Element`] ready for use in an
/// `iced::daemon(...).view(...)` router.
///
/// Sub-task 1 renders a single scrollable column with a localized "Settings"
/// title. Card-level content lands in sub-tasks 3+.
pub fn settings_view(state: &SettingsApp) -> Element<'_, SettingsMessage> {
    let title = text(L10n::settings(state.language)).size(Pixels(SETTINGS_TITLE_FONT_SIZE));

    // The scrollable column is intentionally left with a single child so
    // sub-task 2 can slot in the card stack here without reshaping the
    // outer layout.
    let body = column![title, Space::new().height(Length::Shrink)]
        .spacing(16)
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
}

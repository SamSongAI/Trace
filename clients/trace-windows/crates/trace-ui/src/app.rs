//! `CaptureApp` state and `Message` enum for the iced capture panel.
//!
//! Phase 10 scope is deliberately narrow: [`update`] only mutates
//! `CaptureApp` state for the subset of messages that let us exercise the
//! footer branching and chip selection visuals. Cross-cutting interactions
//! such as submit, global hotkeys, image paste, toast rendering, and window
//! pinning side-effects are Phase 11 concerns.
//!
//! # View layout
//!
//! The panel is a vertical stack:
//!
//! 1. [`crate::widgets::header`] — 36 px brand header with Pin and Settings
//!    buttons
//! 2. 1 px separator line colored with [`trace_core::CapturePalette::border`]
//! 3. [`crate::widgets::editor`] — multi-line text editor filling available
//!    vertical space
//! 4. [`crate::widgets::footer`] — switched on [`trace_core::WriteMode`]
//!
//! # Window settings
//!
//! The capture panel is frameless, transparent-to-false, and resizable from
//! the initial `440 x 520` down to a minimum `360 x 220`, matching the
//! SwiftUI `.frame(minWidth:360, minHeight:220)` and the Mac window
//! controller's default frame. See [`window_settings`].

use iced::widget::{column, container, text_editor, Space};
use iced::window;
use iced::{Element, Length, Size, Theme};
use trace_core::{NoteSection, ThreadConfig, TraceTheme, WriteMode};
use uuid::Uuid;

use crate::theme::{panel_container_style, separator_container_style, to_iced_theme};
use crate::widgets;

/// Initial logical width of the capture panel.
pub const DEFAULT_PANEL_WIDTH: f32 = 440.0;
/// Initial logical height of the capture panel.
pub const DEFAULT_PANEL_HEIGHT: f32 = 520.0;
/// Minimum width the user can resize the panel down to.
pub const MIN_PANEL_WIDTH: f32 = 360.0;
/// Minimum height the user can resize the panel down to.
pub const MIN_PANEL_HEIGHT: f32 = 220.0;
/// Height of the brand header, matching Mac `CaptureView.swift:121`.
pub const HEADER_HEIGHT: f32 = 36.0;
/// Thickness of the divider between header/editor and editor/footer.
pub const SEPARATOR_HEIGHT: f32 = 1.0;

/// Messages consumed by [`update`].
///
/// The Phase 10 surface is intentionally small — every variant either mutates
/// in-memory state or is a no-op placeholder to be wired up in Phase 11.
#[derive(Debug, Clone)]
pub enum Message {
    /// Forwarded from [`iced::widget::text_editor`] — mutates
    /// [`CaptureApp::editor_content`].
    EditorAction(text_editor::Action),
    /// Switches the active footer mode. In Phase 10 this is only ever raised
    /// by tests; Phase 11 will also emit it from the mode-toggle hotkey.
    WriteModeChanged(WriteMode),
    /// Dimension footer chip tap. Phase 10 only persists the selection in
    /// memory; Phase 11 wires the write path.
    SectionSelected(SectionId),
    /// Thread footer chip tap. Same Phase-10-is-state-only rule as
    /// [`Self::SectionSelected`].
    ThreadSelected(ThreadId),
    /// Text change in the document title input.
    DocumentTitleChanged(String),
    /// Pin button tap — toggles [`CaptureApp::pinned`]. The panel-level
    /// "stay on top" effect lives in Phase 11.
    PinToggled,
    /// Settings button tap. Recorded for assertions; opening the settings
    /// window is a Phase 12 task.
    SettingsRequested,
}

/// Stable identifier for a section chip, mirroring the index-based identity
/// of [`trace_core::NoteSection`].
pub type SectionId = usize;

/// Stable identifier for a thread chip, mirroring [`trace_core::ThreadConfig::id`].
pub type ThreadId = Uuid;

/// Mutable application state for the capture panel.
///
/// The struct is intentionally data-oriented: every field is owned by the
/// struct, there are no references to iced internals, and it can be round-
/// tripped through `update()` in unit tests without running iced's event loop.
pub struct CaptureApp {
    /// Full theme palette bundle for the currently selected preset. Cloned
    /// wholesale on preset change so read paths can take a snapshot without
    /// locking.
    pub theme: TraceTheme,
    /// Whether the document-title input is visible, plus all the footer
    /// branching.
    pub write_mode: WriteMode,
    /// iced `text_editor` content handle — mutably borrowed during `update`.
    pub editor_content: text_editor::Content,
    /// Currently highlighted section chip, or [`None`] when no chip has been
    /// tapped this session.
    pub selected_section: Option<SectionId>,
    /// Currently highlighted thread chip.
    pub selected_thread: Option<ThreadId>,
    /// Text shown in the document-title input when `write_mode ==
    /// WriteMode::File`.
    pub document_title: String,
    /// Whether the panel wants to stay on top. The actual z-order effect is
    /// handled by the platform layer in Phase 11.
    pub pinned: bool,
    /// Whether a settings-open request was observed. Exposed mainly for
    /// testability — Phase 12 will upgrade this to an outbound action.
    pub settings_requested: bool,
    /// Sections as configured by the user. Phase 10 carries an owned copy so
    /// the view layer doesn't need to reach into `AppSettings`.
    pub sections: Vec<NoteSection>,
    /// Threads as configured by the user. Already sorted by `order` so
    /// rendering is deterministic.
    pub threads: Vec<ThreadConfig>,
}

impl CaptureApp {
    /// Builds a fresh [`CaptureApp`] seeded from a resolved [`TraceTheme`],
    /// the configured sections, and the configured threads.
    ///
    /// The editor starts empty, the write mode defaults to
    /// [`WriteMode::default`] (dimension), and no chip is highlighted.
    pub fn new(theme: TraceTheme, sections: Vec<NoteSection>, threads: Vec<ThreadConfig>) -> Self {
        // Sort threads by `order` eagerly so the footer grid renders in a
        // stable order without re-sorting per frame.
        let mut threads = threads;
        threads.sort_by_key(|t| t.order);
        Self {
            theme,
            write_mode: WriteMode::default(),
            editor_content: text_editor::Content::new(),
            selected_section: None,
            selected_thread: None,
            document_title: String::new(),
            pinned: false,
            settings_requested: false,
            sections,
            threads,
        }
    }

    /// Returns the current editor text. Convenience wrapper around
    /// [`text_editor::Content::text`].
    pub fn editor_text(&self) -> String {
        self.editor_content.text()
    }
}

/// Mutates the supplied [`CaptureApp`] in response to a [`Message`].
///
/// Split out as a free function (rather than an inherent method on
/// [`CaptureApp`]) to match the iced builder API style and to make unit
/// testing trivial: callers can construct an app, push messages one by one,
/// and assert on the final state.
pub fn update(state: &mut CaptureApp, message: Message) {
    match message {
        Message::EditorAction(action) => {
            state.editor_content.perform(action);
        }
        Message::WriteModeChanged(mode) => {
            state.write_mode = mode;
        }
        Message::SectionSelected(id) => {
            state.selected_section = Some(id);
        }
        Message::ThreadSelected(id) => {
            state.selected_thread = Some(id);
        }
        Message::DocumentTitleChanged(title) => {
            state.document_title = title;
        }
        Message::PinToggled => {
            state.pinned = !state.pinned;
        }
        Message::SettingsRequested => {
            // Phase 10: only record the request. Phase 11 will translate this
            // into an iced Task that opens the settings window.
            state.settings_requested = true;
        }
    }
}

/// Renders the capture panel. Returns an [`Element`] ready for use in
/// `iced::application(...).view(...)`.
pub fn view(state: &CaptureApp) -> Element<'_, Message> {
    let palette = state.theme.capture;
    let header = widgets::header::header(palette, state.pinned);
    let separator = container(
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(SEPARATOR_HEIGHT)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(SEPARATOR_HEIGHT))
    .style(separator_container_style(palette));
    let editor = widgets::editor::editor(palette, &state.editor_content);
    let footer = widgets::footer::footer(
        palette,
        state.write_mode,
        &state.sections,
        &state.threads,
        state.selected_section,
        state.selected_thread,
        &state.document_title,
    );

    let stack = column![header, separator, editor, footer]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_container_style(palette))
        .into()
}

/// Returns the iced [`Theme`] derived from [`CaptureApp::theme`].
///
/// Plumbed through `iced::application(...).theme(theme)` so changing the
/// preset in Phase 12 will live-update the shell.
pub fn theme(state: &CaptureApp) -> Theme {
    to_iced_theme(&state.theme)
}

/// Returns the panel's [`window::Settings`].
///
/// Kept as a free function so the `trace-app` crate can merge these defaults
/// with a persisted [`trace_core::PanelFrame`] before handing them to iced.
/// `trace-ui` intentionally does not depend on `trace-platform`.
pub fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(DEFAULT_PANEL_WIDTH, DEFAULT_PANEL_HEIGHT),
        min_size: Some(Size::new(MIN_PANEL_WIDTH, MIN_PANEL_HEIGHT)),
        resizable: true,
        decorations: false,
        transparent: false,
        ..window::Settings::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::ThemePreset;

    fn sample_sections() -> Vec<NoteSection> {
        (0..NoteSection::DEFAULT_TITLES.len())
            .map(|i| NoteSection::new(i, NoteSection::DEFAULT_TITLES[i]))
            .collect()
    }

    fn sample_threads() -> Vec<ThreadConfig> {
        vec![
            ThreadConfig::new("想法", "想法.md", None, 0),
            ThreadConfig::new("读书笔记", "读书笔记.md", None, 1),
        ]
    }

    fn fresh_app() -> CaptureApp {
        CaptureApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            sample_sections(),
            sample_threads(),
        )
    }

    #[test]
    fn new_starts_in_default_write_mode() {
        let app = fresh_app();
        assert_eq!(app.write_mode, WriteMode::default());
        assert_eq!(app.write_mode, WriteMode::Dimension);
    }

    #[test]
    fn new_has_empty_editor_and_title() {
        let app = fresh_app();
        assert_eq!(app.editor_text(), "");
        assert!(app.document_title.is_empty());
    }

    #[test]
    fn new_has_no_chip_selection() {
        let app = fresh_app();
        assert!(app.selected_section.is_none());
        assert!(app.selected_thread.is_none());
    }

    #[test]
    fn new_is_not_pinned() {
        let app = fresh_app();
        assert!(!app.pinned);
        assert!(!app.settings_requested);
    }

    #[test]
    fn new_sorts_threads_by_order() {
        let threads = vec![
            ThreadConfig::new("b", "b.md", None, 2),
            ThreadConfig::new("a", "a.md", None, 0),
            ThreadConfig::new("c", "c.md", None, 1),
        ];
        let app = CaptureApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            sample_sections(),
            threads,
        );
        let names: Vec<_> = app.threads.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["a", "c", "b"]);
    }

    #[test]
    fn write_mode_changed_switches_mode() {
        let mut app = fresh_app();

        update(&mut app, Message::WriteModeChanged(WriteMode::Thread));
        assert_eq!(app.write_mode, WriteMode::Thread);

        update(&mut app, Message::WriteModeChanged(WriteMode::File));
        assert_eq!(app.write_mode, WriteMode::File);

        update(&mut app, Message::WriteModeChanged(WriteMode::Dimension));
        assert_eq!(app.write_mode, WriteMode::Dimension);
    }

    #[test]
    fn section_selected_updates_state_only() {
        let mut app = fresh_app();
        assert!(app.selected_section.is_none());
        update(&mut app, Message::SectionSelected(2));
        assert_eq!(app.selected_section, Some(2));
        // Other state unchanged.
        assert!(app.selected_thread.is_none());
        assert!(!app.pinned);
    }

    #[test]
    fn thread_selected_updates_state_only() {
        let mut app = fresh_app();
        let id = app.threads[0].id;
        update(&mut app, Message::ThreadSelected(id));
        assert_eq!(app.selected_thread, Some(id));
        assert!(app.selected_section.is_none());
    }

    #[test]
    fn document_title_changed_is_replaced_wholesale() {
        let mut app = fresh_app();
        update(
            &mut app,
            Message::DocumentTitleChanged("draft".to_string()),
        );
        assert_eq!(app.document_title, "draft");
        update(
            &mut app,
            Message::DocumentTitleChanged("final".to_string()),
        );
        assert_eq!(app.document_title, "final");
    }

    #[test]
    fn pin_toggled_flips_flag() {
        let mut app = fresh_app();
        assert!(!app.pinned);
        update(&mut app, Message::PinToggled);
        assert!(app.pinned);
        update(&mut app, Message::PinToggled);
        assert!(!app.pinned);
    }

    #[test]
    fn settings_requested_is_recorded_without_side_effects() {
        let mut app = fresh_app();
        assert!(!app.settings_requested);
        update(&mut app, Message::SettingsRequested);
        assert!(app.settings_requested);
        // Phase 10 guarantees no other state changes.
        assert_eq!(app.write_mode, WriteMode::Dimension);
        assert!(app.selected_section.is_none());
    }

    #[test]
    fn theme_function_returns_iced_custom_theme() {
        let app = fresh_app();
        let iced_theme = theme(&app);
        // The conversion plants the panel_background in the iced palette —
        // re-verify the round-trip here, not because it's theme.rs's job, but
        // because it catches wire-up drift between `theme()` and
        // `to_iced_theme`.
        let expected_bg = app.theme.capture.panel_background;
        let palette = iced_theme.palette();
        assert!((palette.background.r - expected_bg.r as f32 / 255.0).abs() < 0.01);
    }

    #[test]
    fn window_settings_match_mac_panel_defaults() {
        let settings = window_settings();
        assert_eq!(settings.size.width, DEFAULT_PANEL_WIDTH);
        assert_eq!(settings.size.height, DEFAULT_PANEL_HEIGHT);
        let min = settings.min_size.expect("min_size must be set");
        assert_eq!(min.width, MIN_PANEL_WIDTH);
        assert_eq!(min.height, MIN_PANEL_HEIGHT);
        assert!(settings.resizable);
        assert!(!settings.decorations);
        assert!(!settings.transparent);
    }

    /// Smoke test: `view()` must build without panicking for each write mode.
    /// We can't spin up the iced runtime in a unit test, but constructing the
    /// `Element` tree proves the widget graph type-checks under real data.
    #[test]
    fn view_builds_for_every_write_mode() {
        let mut app = fresh_app();
        for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
            update(&mut app, Message::WriteModeChanged(mode));
            let _element: Element<'_, Message> = view(&app);
        }
    }

    #[test]
    fn view_builds_with_chip_selection() {
        let mut app = fresh_app();
        let thread_id = app.threads[0].id;
        update(&mut app, Message::SectionSelected(0));
        update(&mut app, Message::ThreadSelected(thread_id));
        let _element: Element<'_, Message> = view(&app);
    }
}

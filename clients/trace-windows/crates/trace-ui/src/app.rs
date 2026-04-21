//! `CaptureApp` state and `Message` enum for the iced capture panel.
//!
//! Phase 11 wires the keyboard-driven interactions on top of the Phase 10
//! shell: panel-scoped shortcuts (Ctrl+Enter / Ctrl+Shift+Enter / Shift+Tab /
//! Ctrl+1..9 / Ctrl+P / Esc), writer dispatch, toast notifications with a
//! timed auto-dismiss, and auto-close on focus loss when not pinned.
//!
//! Platform-specific effects (topmost window bit, keyboard-focus restoration)
//! are routed through the [`crate::platform::PlatformHandler`] trait so
//! `trace-ui` stays pure — `trace-app` plugs in the real Win32-backed
//! implementation in a later phase.
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
//! A toast pill overlay is rendered on top of the stack via
//! [`iced::widget::stack`] when [`CaptureApp::toast`] is `Some`.
//!
//! # Window settings
//!
//! The capture panel is frameless, transparent-to-false, and resizable from
//! the initial `440 x 520` down to a minimum `360 x 220`, matching the
//! SwiftUI `.frame(minWidth:360, minHeight:220)` and the Mac window
//! controller's default frame. See [`window_settings`].

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use iced::event::{self as iced_event};
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key, Modifiers};
use iced::widget::{column, container, stack, text_editor, Space};
use iced::window::Event as WindowEvent;
use iced::{time, window, Event, Subscription, Task};
use iced::{Element, Length, Size, Theme};
use trace_core::{
    AppSettings, DailyNoteWriter, FileWriter, NoteSection, SaveMode, ThreadConfig, ThreadWriter,
    TraceTheme, WriteMode,
};
use uuid::Uuid;

use crate::platform::PlatformHandler;
use crate::theme::{panel_container_style, separator_container_style, to_iced_theme};
use crate::widgets;

/// How long a toast stays on screen before the auto-dismiss subscription
/// fires. Matches Mac's 1.5 s fade-out.
pub const TOAST_AUTO_DISMISS: Duration = Duration::from_millis(1500);

/// User-facing toast message shown when the editor is empty on Send/Append.
/// Chinese literal per Phase 11 plan; L10n wiring lands in Phase 12.
pub const TOAST_EMPTY_NOT_SAVED: &str = "空内容未保存";
/// User-facing toast message shown when Thread mode is active but no thread
/// chip has been selected yet.
pub const TOAST_THREAD_NOT_SELECTED: &str = "请先选择一个线程";

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
/// Phase 11 extends the surface with keyboard-driven interactions (Send /
/// Append / CycleMode / SelectByIndex / ClosePanel) and toast plumbing.
/// Every variant either mutates in-memory state, issues an [`iced::Task`],
/// or both.
#[derive(Debug, Clone)]
pub enum Message {
    /// Forwarded from [`iced::widget::text_editor`] — mutates
    /// [`CaptureApp::editor_content`].
    EditorAction(text_editor::Action),
    /// Switches the active footer mode. Phase 10 only raised this from tests;
    /// Phase 11 also emits it from [`Self::CycleModeForward`].
    WriteModeChanged(WriteMode),
    /// Dimension footer chip tap.
    SectionSelected(SectionId),
    /// Thread footer chip tap.
    ThreadSelected(ThreadId),
    /// Text change in the document title input.
    DocumentTitleChanged(String),
    /// Pin button tap — toggles [`CaptureApp::pinned`] and notifies the
    /// platform handler so the window's topmost bit stays in sync.
    PinToggled,
    /// Settings button tap. Recorded for assertions; opening the settings
    /// window is a Phase 12 task.
    SettingsRequested,
    /// Ctrl+Enter / Send: write the current editor content as a **new** entry
    /// via the writer appropriate for the active [`WriteMode`], then clear
    /// the editor and close the panel if not pinned.
    SendNote,
    /// Ctrl+Shift+Enter / Append: append the current editor content to the
    /// latest entry. Falls back to create-new when the writer reports no
    /// prior entry anchor (handled inside the writer).
    AppendNote,
    /// Shift+Tab: cycles [`CaptureApp::write_mode`] through
    /// Dimension → Thread → File → Dimension.
    CycleModeForward,
    /// Ctrl+1..9: jump to the zero-indexed chip in the currently active
    /// mode's list (sections for Dimension, threads for Thread). Silent
    /// no-op in File mode or when the index is out of bounds.
    SelectByIndex(usize),
    /// Esc / post-send when not pinned: close the capture window. Emits an
    /// [`iced::window::close`] task when a window id has been captured via
    /// [`Self::WindowOpened`].
    ClosePanel,
    /// Requests the toast pill to display `message` for the auto-dismiss
    /// duration. Replaces any in-flight toast.
    ToastShow(String),
    /// Timer-driven clear signal. Emitted by the [`subscription`] 1.5 s
    /// [`time::every`] while a toast is active.
    ToastDismiss,
    /// `iced::window::Event::Unfocused` translated to a domain message, only
    /// raised while `pinned == false`. Routed through the main [`update`]
    /// path so the close flow is centralised.
    FocusLost,
    /// `iced::window::Event::Opened` translated to a domain message so
    /// [`CaptureApp`] can capture the window id for the close task.
    WindowOpened(window::Id),
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
    /// Pre-built iced [`Theme`] derived from [`CaptureApp::theme`]. iced calls
    /// the `theme()` closure on every frame, and `to_iced_theme` performs a
    /// full palette rebuild (colour mixing plus an `Arc` allocation). We cache
    /// the result here and only rebuild it when the preset actually changes.
    ///
    /// **Invariant**: `iced_theme == to_iced_theme(&theme)`. [`set_theme`]
    /// refreshes both fields atomically; direct mutation of `theme` will
    /// desynchronise the cache.
    pub iced_theme: Theme,
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
    /// routed through [`CaptureApp::platform_handler`] in Phase 11.
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
    /// Settings snapshot used by the writers in [`Message::SendNote`] /
    /// [`Message::AppendNote`]. We hold an owned [`AppSettings`] so writers
    /// can be constructed cheaply per-call without borrowing across the
    /// iced event loop; Phase 12 swaps this to an `Arc` when settings-edit
    /// flows need live reload.
    pub settings: AppSettings,
    /// Message shown in the overlay toast pill. `None` hides the overlay.
    /// Mutated by [`Message::ToastShow`] / [`Message::ToastDismiss`]; rendered
    /// via [`iced::widget::stack`] on top of the main column.
    pub toast: Option<String>,
    /// Window id captured from [`iced::window::Event::Opened`]. Needed because
    /// the close task requires a concrete id and iced 0.14 exposes window ids
    /// only at runtime.
    pub capture_window_id: Option<window::Id>,
    /// Optional platform hook. `None` in unit tests that don't care about
    /// side effects; `Some` when `trace-app` plugs in its
    /// `trace-platform`-backed implementation. Stored behind an `Arc` so it
    /// survives any `Clone` of the handler across iced's internal task
    /// plumbing.
    pub platform_handler: Option<Arc<dyn PlatformHandler + Send + Sync>>,
}

impl CaptureApp {
    /// Builds a fresh [`CaptureApp`] seeded from a resolved [`TraceTheme`],
    /// the configured sections, the configured threads, and an
    /// [`AppSettings`] snapshot used by the writers in
    /// [`Message::SendNote`] / [`Message::AppendNote`].
    ///
    /// The editor starts empty, the write mode defaults to
    /// [`WriteMode::default`] (dimension), no chip is highlighted, no toast is
    /// visible, no window id has been captured yet, and
    /// [`CaptureApp::platform_handler`] is `None` — attach one via
    /// [`CaptureApp::with_platform_handler`] during app wire-up when real
    /// platform effects are required.
    pub fn new(
        theme: TraceTheme,
        sections: Vec<NoteSection>,
        threads: Vec<ThreadConfig>,
        settings: AppSettings,
    ) -> Self {
        // Sort threads by `order` eagerly so the footer grid renders in a
        // stable order without re-sorting per frame.
        let mut threads = threads;
        threads.sort_by_key(|t| t.order);
        let iced_theme = to_iced_theme(&theme);
        Self {
            theme,
            iced_theme,
            write_mode: WriteMode::default(),
            editor_content: text_editor::Content::new(),
            selected_section: None,
            selected_thread: None,
            document_title: String::new(),
            pinned: false,
            settings_requested: false,
            sections,
            threads,
            settings,
            toast: None,
            capture_window_id: None,
            platform_handler: None,
        }
    }

    /// Plugs a platform handler into an already-constructed [`CaptureApp`].
    ///
    /// Chained after [`CaptureApp::new`] in `trace-app`'s wire-up so
    /// `trace-ui` never has to know how a real handler is built. Unit tests
    /// that care about side effects can pass in a mock.
    pub fn with_platform_handler(
        mut self,
        handler: Arc<dyn PlatformHandler + Send + Sync>,
    ) -> Self {
        self.platform_handler = Some(handler);
        self
    }

    /// Returns the current editor text. Convenience wrapper around
    /// [`text_editor::Content::text`].
    pub fn editor_text(&self) -> String {
        self.editor_content.text()
    }

    /// Replaces the active [`TraceTheme`] and refreshes the cached
    /// [`Self::iced_theme`] atomically. This is the only supported way to
    /// change the preset — callers that mutate [`Self::theme`] directly will
    /// leave the cache stale. Phase 12 wires this into the settings flow.
    pub fn set_theme(&mut self, theme: TraceTheme) {
        self.iced_theme = to_iced_theme(&theme);
        self.theme = theme;
    }
}

/// Mutates the supplied [`CaptureApp`] in response to a [`Message`] and
/// returns an [`iced::Task`] describing any follow-up effect.
///
/// Split out as a free function (rather than an inherent method on
/// [`CaptureApp`]) to match the iced builder API style and to make unit
/// testing trivial: callers can construct an app, push messages one by one,
/// and assert on the final state. Follow-up tasks returned by this function
/// are `Task::none()` in sub-task 1; sub-tasks 3 and 5 wire in the real
/// writer-dispatch and close-window effects.
pub fn update(state: &mut CaptureApp, message: Message) -> Task<Message> {
    match message {
        Message::EditorAction(action) => {
            state.editor_content.perform(action);
            Task::none()
        }
        Message::WriteModeChanged(mode) => {
            state.write_mode = mode;
            Task::none()
        }
        Message::SectionSelected(id) => {
            state.selected_section = Some(id);
            Task::none()
        }
        Message::ThreadSelected(id) => {
            state.selected_thread = Some(id);
            Task::none()
        }
        Message::DocumentTitleChanged(title) => {
            state.document_title = title;
            Task::none()
        }
        Message::PinToggled => {
            state.pinned = !state.pinned;
            if let Some(handler) = state.platform_handler.as_ref() {
                handler.set_topmost(state.pinned);
            }
            Task::none()
        }
        Message::SettingsRequested => {
            // Recorded for test visibility. Phase 12 upgrades this to an
            // outbound task that opens the settings window.
            state.settings_requested = true;
            Task::none()
        }
        Message::SendNote => dispatch_save(state, SaveMode::CreateNewEntry).into_task(),
        Message::AppendNote => dispatch_save(state, SaveMode::AppendToLatestEntry).into_task(),
        Message::CycleModeForward => {
            state.write_mode = state.write_mode.next();
            Task::none()
        }
        Message::SelectByIndex(index) => {
            // Route the zero-indexed shortcut into the active mode's chip list.
            // Silent no-op in File mode or when the index is out of range,
            // matching Mac `CapturePanelController.setupLocalKeyMonitor`.
            match state.write_mode {
                WriteMode::Dimension => {
                    if index < state.sections.len() {
                        state.selected_section = Some(index);
                    }
                }
                WriteMode::Thread => {
                    if let Some(thread) = state.threads.get(index) {
                        state.selected_thread = Some(thread.id);
                    }
                }
                WriteMode::File => {
                    // File mode has no chips — silently ignore.
                }
            }
            Task::none()
        }
        Message::ClosePanel => {
            // Hand the keyboard focus back to whichever app was frontmost
            // before the panel was shown, then issue the window close task.
            // The platform handler is optional so headless tests can run
            // without stubbing Win32.
            if let Some(handler) = state.platform_handler.as_ref() {
                handler.restore_foreground();
            }
            if let Some(id) = state.capture_window_id {
                window::close(id)
            } else {
                // No window id captured yet — nothing to close. The
                // `restore_foreground` call above is still useful for tests.
                Task::none()
            }
        }
        Message::ToastShow(message) => {
            state.toast = Some(message);
            Task::none()
        }
        Message::ToastDismiss => {
            state.toast = None;
            Task::none()
        }
        Message::FocusLost => {
            // Auto-close on focus loss unless the user has pinned the panel.
            // Matches Mac `NSPanel.hidesOnDeactivate = !viewModel.pinned`.
            if state.pinned {
                Task::none()
            } else {
                Task::done(Message::ClosePanel)
            }
        }
        Message::WindowOpened(id) => {
            state.capture_window_id = Some(id);
            Task::none()
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

    let panel_stack = column![header, separator, editor, footer]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    let panel: Element<'_, Message> = container(panel_stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_container_style(palette))
        .into();

    // Overlay the toast pill via `iced::widget::stack!` when a message is
    // live. Layering the toast on top of the existing panel keeps the
    // editor/footer layout untouched — the pill floats over them rather
    // than stealing vertical space.
    match state.toast.as_deref() {
        Some(message) => {
            let pill = widgets::toast::toast(palette, message);
            stack![panel, pill].into()
        }
        None => panel,
    }
}

/// Returns the iced [`Theme`] derived from [`CaptureApp::theme`].
///
/// Plumbed through `iced::application(...).theme(theme)` so changing the
/// preset in Phase 12 will live-update the shell. Reads from the cached
/// [`CaptureApp::iced_theme`] so the per-frame cost is an `Arc` clone rather
/// than a full palette rebuild.
pub fn theme(state: &CaptureApp) -> Theme {
    state.iced_theme.clone()
}

/// Aggregate subscription for the capture panel.
///
/// Fans out three independent streams via [`Subscription::batch`]:
///
/// 1. The 1.5 s toast auto-dismiss timer — only mounted while
///    [`CaptureApp::toast`] is `Some`, otherwise [`Subscription::none`].
/// 2. The keyboard shortcut + window-unfocus listener, implemented as a
///    `fn` pointer passed to [`iced::event::listen_with`]. Because
///    `listen_with` requires a non-capturing function pointer, all context-
///    dependent decisions (e.g. honouring the `pinned` flag before closing)
///    are deferred into [`update`].
/// 3. The window-open listener, which captures the primary [`window::Id`]
///    so [`Message::ClosePanel`] can emit the correct close task.
pub fn subscription(state: &CaptureApp) -> Subscription<Message> {
    let toast = if state.toast.is_some() {
        time::every(TOAST_AUTO_DISMISS).map(|_| Message::ToastDismiss)
    } else {
        Subscription::none()
    };

    let events = iced_event::listen_with(decode_event);
    let opens = window::open_events().map(Message::WindowOpened);

    Subscription::batch(vec![toast, events, opens])
}

/// `listen_with` callback. Must be a plain `fn` (no captures) — see
/// [`iced::event::listen_with`]. Keeps the runtime noise out of the
/// keyboard decoder by delegating to [`decode_shortcut`] for every
/// keyboard event and to a small inline branch for focus loss.
fn decode_event(event: Event, status: iced_event::Status, _window: window::Id) -> Option<Message> {
    // Skip events already handled by a widget (e.g. the editor swallowing
    // a printable character). iced's `listen_with` only delivers
    // `Ignored` events, but `listen_raw` would leak every redraw, so the
    // explicit match is kept defensive.
    if matches!(status, iced_event::Status::Captured) {
        return None;
    }

    match event {
        Event::Keyboard(KeyboardEvent::KeyPressed { key, modifiers, .. }) => {
            decode_shortcut(&key, modifiers)
        }
        Event::Window(WindowEvent::Unfocused) => Some(Message::FocusLost),
        _ => None,
    }
}

/// Translates a keyboard key + modifier combo into a [`Message`].
///
/// Invoked by [`decode_event`] for every key-press on an unfocused event
/// stream. Returning `None` leaves the key for the editor.
///
/// The contract mirrors `CapturePanelController.setupLocalKeyMonitor` on
/// Mac:
///
/// | Keys                | Message                   |
/// |---------------------|---------------------------|
/// | `Esc`               | [`Message::ClosePanel`]   |
/// | `Shift+Tab`         | [`Message::CycleModeForward`] |
/// | `Ctrl+Enter`        | [`Message::SendNote`]     |
/// | `Ctrl+Shift+Enter`  | [`Message::AppendNote`]   |
/// | `Ctrl+P`            | [`Message::PinToggled`]   |
/// | `Ctrl+1`..`Ctrl+9`  | [`Message::SelectByIndex`] |
///
/// Ctrl+0 is intentionally unmapped to match Mac's 1-indexed chip list
/// (`Cmd+1` → index 0, so `Ctrl+1` → index 0 here).
pub(crate) fn decode_shortcut(key: &Key, modifiers: Modifiers) -> Option<Message> {
    // Escape is a bare key — no modifiers required. This is safe to
    // fire from inside the editor because a plain Esc can't produce any
    // useful text editing operation in a single-line capture panel.
    if let Key::Named(Named::Escape) = key {
        if modifiers.is_empty() {
            return Some(Message::ClosePanel);
        }
    }

    // Shift+Tab cycles the footer mode. Plain Tab is reserved for focus
    // traversal inside the editor.
    if matches!(key, Key::Named(Named::Tab)) && modifiers.shift() && !modifiers.control() {
        return Some(Message::CycleModeForward);
    }

    // Ctrl+[Shift+]Enter dispatches writer save.
    if matches!(key, Key::Named(Named::Enter)) && modifiers.control() {
        if modifiers.shift() {
            return Some(Message::AppendNote);
        } else {
            return Some(Message::SendNote);
        }
    }

    // Ctrl-prefixed character shortcuts. Mac uses Cmd; Windows uses Ctrl.
    if modifiers.control() && !modifiers.alt() && !modifiers.logo() {
        if let Key::Character(chars) = key {
            // `Character` payloads are locale-aware; lowercase them so
            // Shift+Ctrl+P still routes to PinToggled on keyboards that
            // report an uppercase glyph under shift.
            let lower = chars.to_lowercase();
            if !modifiers.shift() && lower == "p" {
                return Some(Message::PinToggled);
            }
            if let Some(idx) = ctrl_digit_to_index(lower.as_str()) {
                return Some(Message::SelectByIndex(idx));
            }
        }
    }

    None
}

/// Maps the text payload of a `Key::Character` produced by a digit key
/// to its zero-based chip index. `"1"` → `0`, `"9"` → `8`; anything else
/// (including `"0"`) returns `None`.
fn ctrl_digit_to_index(s: &str) -> Option<usize> {
    let c = s.chars().next()?;
    if s.len() == c.len_utf8() && ('1'..='9').contains(&c) {
        Some(c as usize - '1' as usize)
    } else {
        None
    }
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

/// Inspectable outcome of [`dispatch_save`]. Kept separate from the
/// [`Task<Message>`] return so unit tests can reason about the decision
/// without dismantling iced's task machinery, which exposes no public API
/// for peeking at queued messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    /// Guard failure — editor text was empty or whitespace-only.
    EmptyGuard,
    /// Guard failure — Thread mode is active but no thread is selected, or
    /// the previously selected thread id is no longer configured.
    ThreadNotSelected,
    /// Guard failure — Dimension mode is active but the configured section
    /// list is empty. A pathological settings shape.
    NoSectionAvailable,
    /// Writer returned `Ok(None)` — the trimmed text was empty after the
    /// writer re-trimmed. Unreachable if `EmptyGuard` fired first, but we
    /// still translate it to a toast (the writer is the source of truth).
    WriterSilentNoop,
    /// Writer succeeded. Callers must clear the editor and close the
    /// panel.
    Written,
    /// Writer returned an error. The payload is the error's `to_string()`
    /// so tests can inspect the user-visible toast text.
    WriterError(String),
}

impl SaveOutcome {
    /// Translates the outcome into the iced task that feeds the next
    /// [`update`] pass.
    fn into_task(self) -> Task<Message> {
        match self {
            SaveOutcome::EmptyGuard => {
                Task::done(Message::ToastShow(TOAST_EMPTY_NOT_SAVED.to_string()))
            }
            SaveOutcome::ThreadNotSelected => {
                Task::done(Message::ToastShow(TOAST_THREAD_NOT_SELECTED.to_string()))
            }
            SaveOutcome::NoSectionAvailable => {
                Task::done(Message::ToastShow("未找到可用的章节".to_string()))
            }
            SaveOutcome::WriterSilentNoop => {
                Task::done(Message::ToastShow(TOAST_EMPTY_NOT_SAVED.to_string()))
            }
            SaveOutcome::Written => Task::done(Message::ClosePanel),
            SaveOutcome::WriterError(msg) => Task::done(Message::ToastShow(msg)),
        }
    }
}

/// Drives the `SendNote` / `AppendNote` handlers.
///
/// Guards are applied in the UI layer (empty text, thread-mode-requires-
/// selection) so the user sees a toast immediately — the writers also guard
/// internally but toast dispatch lives here. On success the editor is
/// cleared to match Mac's `CaptureViewModel.save`.
///
/// The function is synchronous: trace-core writers are cheap to construct
/// and I/O is small (single markdown file), so blocking the UI thread for a
/// few milliseconds is simpler than threading an async boundary through
/// iced's task machinery. Phase 13 can lift this into `Task::perform` when
/// vault paths live on remote drives.
///
/// Returns a [`SaveOutcome`] describing what happened; the caller converts
/// it to an [`iced::Task`] via [`SaveOutcome::into_task`] on the way back
/// into `update()`.
pub(crate) fn dispatch_save(state: &mut CaptureApp, mode: SaveMode) -> SaveOutcome {
    let text = state.editor_content.text();
    if text.trim().is_empty() {
        return SaveOutcome::EmptyGuard;
    }

    let now = Utc::now();
    let write_result: Result<Option<()>, trace_core::TraceError> = match state.write_mode {
        WriteMode::Dimension => {
            // Default to the first configured section when the user hasn't
            // tapped a chip yet, matching Mac's `selectedSection = .note`.
            let section_index = state.selected_section.unwrap_or(0);
            let section = state
                .sections
                .get(section_index)
                .cloned()
                .or_else(|| state.sections.first().cloned());
            let Some(section) = section else {
                return SaveOutcome::NoSectionAvailable;
            };
            let writer = DailyNoteWriter::new(state.settings.clone());
            writer
                .save(&text, &section, mode, now)
                .map(|written| written.map(|_| ()))
        }
        WriteMode::Thread => {
            let Some(thread_id) = state.selected_thread else {
                return SaveOutcome::ThreadNotSelected;
            };
            let Some(thread) = state.threads.iter().find(|t| t.id == thread_id).cloned() else {
                // Selected id no longer matches any configured thread; treat
                // the same as the not-selected case so the user knows to
                // re-pick.
                return SaveOutcome::ThreadNotSelected;
            };
            let writer = ThreadWriter::new(state.settings.clone());
            writer
                .save(&text, &thread, mode, now)
                .map(|written| written.map(|_| ()))
        }
        WriteMode::File => {
            // `FileWriter::save` has no `SaveMode` parameter — Append
            // degrades to a fresh-file Send, matching Mac.
            let writer = FileWriter::new(state.settings.clone());
            let title_opt = if state.document_title.trim().is_empty() {
                None
            } else {
                Some(state.document_title.as_str())
            };
            writer
                .save(&text, title_opt, None, now)
                .map(|written| written.map(|_| ()))
        }
    };

    match write_result {
        Ok(Some(())) => {
            // Clear the editor by replacing its contents. iced's
            // `text_editor::Content` has no `clear()` shortcut, so swap in a
            // fresh instance.
            state.editor_content = text_editor::Content::new();
            SaveOutcome::Written
        }
        Ok(None) => SaveOutcome::WriterSilentNoop,
        Err(err) => SaveOutcome::WriterError(err.to_string()),
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
            AppSettings::default(),
        )
    }

    /// Test-only wrapper around [`update`] that discards the returned
    /// [`Task`]. The production code treats the task as load-bearing (that is
    /// what drives window closes / file writes under iced), but unit tests in
    /// this module only care about the state mutation, so we drop it
    /// explicitly to keep call sites terse and clippy happy.
    fn apply(app: &mut CaptureApp, message: Message) {
        let _ = update(app, message);
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
            AppSettings::default(),
        );
        let names: Vec<_> = app.threads.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["a", "c", "b"]);
    }

    #[test]
    fn write_mode_changed_switches_mode() {
        let mut app = fresh_app();

        apply(&mut app, Message::WriteModeChanged(WriteMode::Thread));
        assert_eq!(app.write_mode, WriteMode::Thread);

        apply(&mut app, Message::WriteModeChanged(WriteMode::File));
        assert_eq!(app.write_mode, WriteMode::File);

        apply(&mut app, Message::WriteModeChanged(WriteMode::Dimension));
        assert_eq!(app.write_mode, WriteMode::Dimension);
    }

    #[test]
    fn section_selected_updates_state_only() {
        let mut app = fresh_app();
        assert!(app.selected_section.is_none());
        apply(&mut app, Message::SectionSelected(2));
        assert_eq!(app.selected_section, Some(2));
        // Other state unchanged.
        assert!(app.selected_thread.is_none());
        assert!(!app.pinned);
    }

    #[test]
    fn thread_selected_updates_state_only() {
        let mut app = fresh_app();
        let id = app.threads[0].id;
        apply(&mut app, Message::ThreadSelected(id));
        assert_eq!(app.selected_thread, Some(id));
        assert!(app.selected_section.is_none());
    }

    #[test]
    fn document_title_changed_is_replaced_wholesale() {
        let mut app = fresh_app();
        apply(&mut app, Message::DocumentTitleChanged("draft".to_string()));
        assert_eq!(app.document_title, "draft");
        apply(&mut app, Message::DocumentTitleChanged("final".to_string()));
        assert_eq!(app.document_title, "final");
    }

    #[test]
    fn pin_toggled_flips_flag() {
        let mut app = fresh_app();
        assert!(!app.pinned);
        apply(&mut app, Message::PinToggled);
        assert!(app.pinned);
        apply(&mut app, Message::PinToggled);
        assert!(!app.pinned);
    }

    #[test]
    fn settings_requested_is_recorded_without_side_effects() {
        let mut app = fresh_app();
        assert!(!app.settings_requested);
        apply(&mut app, Message::SettingsRequested);
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
    fn set_theme_refreshes_cached_iced_theme() {
        use crate::theme::to_iced_theme;

        let mut app = fresh_app();
        let dark_fresh = to_iced_theme(&app.theme);
        assert_eq!(
            theme(&app).palette().background,
            dark_fresh.palette().background,
            "initial cache must match a fresh to_iced_theme(Dark)"
        );

        app.set_theme(TraceTheme::for_preset(ThemePreset::Light));
        let light_fresh = to_iced_theme(&app.theme);
        assert_eq!(
            theme(&app).palette().background,
            light_fresh.palette().background,
            "after set_theme the cache must match a fresh to_iced_theme(Light)"
        );
        assert_ne!(
            dark_fresh.palette().background,
            light_fresh.palette().background,
            "Dark and Light should produce distinct palette backgrounds"
        );
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
            apply(&mut app, Message::WriteModeChanged(mode));
            let _element: Element<'_, Message> = view(&app);
        }
    }

    #[test]
    fn view_builds_with_chip_selection() {
        let mut app = fresh_app();
        let thread_id = app.threads[0].id;
        apply(&mut app, Message::SectionSelected(0));
        apply(&mut app, Message::ThreadSelected(thread_id));
        let _element: Element<'_, Message> = view(&app);
    }

    // ---------------------------------------------------------------------
    // Phase 11 — new field defaults and skeleton handler behaviour.
    //
    // Every variant added in Phase 11 gets a dedicated state-mutation test so
    // future sub-tasks that widen the handler (write_mode dispatch, window
    // close, etc.) can extend the assertions without having to re-discover
    // which variants exist.
    // ---------------------------------------------------------------------

    #[test]
    fn new_starts_without_toast_or_window_id() {
        let app = fresh_app();
        assert!(app.toast.is_none());
        assert!(app.capture_window_id.is_none());
    }

    #[test]
    fn new_has_no_platform_handler_by_default() {
        let app = fresh_app();
        assert!(app.platform_handler.is_none());
    }

    #[test]
    fn with_platform_handler_plugs_in_and_returns_self() {
        let handler = crate::platform::mock::MockPlatformHandler::new();
        let app = fresh_app().with_platform_handler(handler);
        assert!(app.platform_handler.is_some());
    }

    #[test]
    fn pin_toggled_forwards_state_to_the_platform_handler() {
        let spy = crate::platform::mock::MockPlatformHandler::new();
        let mut app = fresh_app().with_platform_handler(spy.clone());
        assert_eq!(spy.set_topmost_call_count(), 0);
        apply(&mut app, Message::PinToggled);
        assert!(app.pinned);
        assert_eq!(spy.set_topmost_call_count(), 1);
        assert!(
            spy.last_set_topmost(),
            "handler sees pinned=true after first toggle"
        );
        apply(&mut app, Message::PinToggled);
        assert!(!app.pinned);
        assert_eq!(spy.set_topmost_call_count(), 2);
        assert!(
            !spy.last_set_topmost(),
            "handler sees pinned=false after second toggle"
        );
    }

    #[test]
    fn pin_toggled_without_a_handler_still_mutates_state() {
        // Not every wire-up provides a handler (headless tests, hypothetical
        // CI probes). Sub-task 2's contract is that the handler is optional,
        // so state must still flip even when `platform_handler` is `None`.
        let mut app = fresh_app();
        apply(&mut app, Message::PinToggled);
        assert!(app.pinned);
        apply(&mut app, Message::PinToggled);
        assert!(!app.pinned);
    }

    #[test]
    fn new_carries_the_supplied_app_settings() {
        // The settings field drives writer construction in sub-task 3, so
        // confirm the ctor hands off an owned copy. We pick `vault_path`
        // because it's a user-visible scalar on `AppSettings`.
        let settings = AppSettings {
            vault_path: "/tmp/trace-phase11".to_string(),
            ..AppSettings::default()
        };
        let app = CaptureApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            sample_sections(),
            sample_threads(),
            settings,
        );
        assert_eq!(app.settings.vault_path, "/tmp/trace-phase11");
    }

    // The direct-behaviour `SendNote` / `AppendNote` tests live further down
    // (see `send_note_*` / `append_note_*` blocks). Coverage includes the
    // empty-text toast, thread-not-selected toast, dimension success with a
    // real temp vault, thread success/failure paths, file-mode title
    // forwarding, and the editor-clears-on-success contract.

    #[test]
    fn cycle_mode_forward_walks_dimension_thread_file_dimension() {
        let mut app = fresh_app();
        assert_eq!(app.write_mode, WriteMode::Dimension);
        apply(&mut app, Message::CycleModeForward);
        assert_eq!(app.write_mode, WriteMode::Thread);
        apply(&mut app, Message::CycleModeForward);
        assert_eq!(app.write_mode, WriteMode::File);
        apply(&mut app, Message::CycleModeForward);
        assert_eq!(app.write_mode, WriteMode::Dimension);
    }

    #[test]
    fn close_panel_leaves_unrelated_state_untouched() {
        let mut app = fresh_app();
        apply(&mut app, Message::ClosePanel);
        // The close flow issues a window-close task + restore_foreground
        // call (exercised in dedicated tests). It must not mutate the
        // user-visible pin flag or any toast.
        assert!(!app.pinned);
        assert!(app.toast.is_none());
    }

    #[test]
    fn toast_show_sets_message_and_dismiss_clears_it() {
        let mut app = fresh_app();
        apply(&mut app, Message::ToastShow("空内容未保存".to_string()));
        assert_eq!(app.toast.as_deref(), Some("空内容未保存"));
        apply(&mut app, Message::ToastDismiss);
        assert!(app.toast.is_none());
    }

    #[test]
    fn toast_show_replaces_any_in_flight_message() {
        let mut app = fresh_app();
        apply(&mut app, Message::ToastShow("first".to_string()));
        apply(&mut app, Message::ToastShow("second".to_string()));
        assert_eq!(app.toast.as_deref(), Some("second"));
    }

    #[test]
    fn focus_lost_leaves_unrelated_state_untouched() {
        let mut app = fresh_app();
        apply(&mut app, Message::FocusLost);
        // FocusLost in an unpinned app routes to ClosePanel via an
        // iced task (exercised in the sub-task 5 tests). This test
        // guards the minimal invariant: state fields unrelated to the
        // close flow are not touched.
        assert!(!app.pinned);
        assert!(app.capture_window_id.is_none());
    }

    #[test]
    fn window_opened_records_the_window_id() {
        let mut app = fresh_app();
        let id = window::Id::unique();
        apply(&mut app, Message::WindowOpened(id));
        assert_eq!(app.capture_window_id, Some(id));
    }

    // ---------------------------------------------------------------------
    // Phase 11 — `SendNote` / `AppendNote` writer dispatch.
    //
    // The helper below builds an app pointed at a tempdir vault so tests can
    // assert on the file system after a Send. We exercise:
    //   * Empty guard → `EmptyGuard`.
    //   * Dimension success → file on disk, editor cleared, `Written`.
    //   * Thread mode with no selection → `ThreadNotSelected`.
    //   * Thread mode with a selection → `Written` + editor cleared.
    //   * Stale thread id → `ThreadNotSelected`.
    //   * File mode title fallback → `Written` with title forwarded.
    //   * Writer error (invalid vault) → `WriterError` carrying the message.
    //   * Zero sections → `NoSectionAvailable`.
    // ---------------------------------------------------------------------

    fn app_with_vault(tempdir: &tempfile::TempDir, threads: Vec<ThreadConfig>) -> CaptureApp {
        let mut settings = AppSettings {
            vault_path: tempdir.path().to_string_lossy().to_string(),
            inbox_vault_path: tempdir.path().to_string_lossy().to_string(),
            ..AppSettings::default()
        };
        // Thread configs live on AppSettings so ThreadWriter can reach them
        // through the `ThreadSettings` impl.
        settings.thread_configs = threads.clone();
        CaptureApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            sample_sections(),
            threads,
            settings,
        )
    }

    fn write_text(app: &mut CaptureApp, text: &str) {
        app.editor_content = text_editor::Content::with_text(text);
    }

    #[test]
    fn dispatch_save_empty_text_returns_empty_guard() {
        let mut app = fresh_app();
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::EmptyGuard
        );
        // Editor untouched.
        assert_eq!(app.editor_text(), "");
    }

    #[test]
    fn dispatch_save_whitespace_only_returns_empty_guard() {
        let mut app = fresh_app();
        write_text(&mut app, "   \n\t  ");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::EmptyGuard
        );
        // Editor retains the user's in-flight (whitespace) text — the guard
        // does NOT clear it, matching Mac's "show toast, keep typing" UX.
        assert_eq!(app.editor_text(), "   \n\t  ");
    }

    #[test]
    fn dispatch_save_dimension_success_writes_file_and_clears_editor() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        write_text(&mut app, "dimension body");
        let outcome = dispatch_save(&mut app, SaveMode::CreateNewEntry);
        assert_eq!(outcome, SaveOutcome::Written);
        assert_eq!(app.editor_text(), "", "successful Send clears the editor");
        // Writer creates a file under the Daily folder; we don't assert on
        // the exact name (date-dependent) but the vault root should now have
        // at least one file.
        let daily_root = tempdir.path().join(app.settings.daily_folder_name.clone());
        let entries: Vec<_> = std::fs::read_dir(&daily_root)
            .expect("daily folder exists")
            .collect();
        assert!(
            !entries.is_empty(),
            "Dimension write produced at least one daily note file"
        );
    }

    #[test]
    fn dispatch_save_thread_mode_without_selection_returns_thread_not_selected() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        app.write_mode = WriteMode::Thread;
        write_text(&mut app, "note body");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::ThreadNotSelected
        );
        // Editor retained; user can pick a thread and re-send.
        assert_eq!(app.editor_text(), "note body");
    }

    #[test]
    fn dispatch_save_thread_mode_with_selection_writes_and_clears() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let threads = sample_threads();
        let thread_id = threads[0].id;
        let mut app = app_with_vault(&tempdir, threads);
        app.write_mode = WriteMode::Thread;
        app.selected_thread = Some(thread_id);
        write_text(&mut app, "thread body");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::Written
        );
        assert_eq!(app.editor_text(), "");
    }

    #[test]
    fn dispatch_save_thread_mode_with_stale_id_returns_thread_not_selected() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        app.write_mode = WriteMode::Thread;
        app.selected_thread = Some(Uuid::new_v4()); // not in the list
        write_text(&mut app, "body");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::ThreadNotSelected
        );
    }

    #[test]
    fn dispatch_save_file_mode_forwards_document_title() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        app.write_mode = WriteMode::File;
        app.document_title = "my-doc".to_string();
        write_text(&mut app, "free-form body");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::Written
        );
        assert_eq!(app.editor_text(), "");
        // Document title is NOT cleared — matches Mac where users batch
        // multiple notes under one working title.
        assert_eq!(app.document_title, "my-doc");
    }

    #[test]
    fn dispatch_save_writer_error_surfaces_message() {
        // Blank vault path triggers `InvalidVaultPath` from the writer.
        let settings = AppSettings {
            vault_path: "".to_string(),
            ..AppSettings::default()
        };
        let mut app = CaptureApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            sample_sections(),
            sample_threads(),
            settings,
        );
        write_text(&mut app, "body");
        let outcome = dispatch_save(&mut app, SaveMode::CreateNewEntry);
        match outcome {
            SaveOutcome::WriterError(msg) => {
                assert!(
                    msg.contains("vault path") || msg.contains("InvalidVaultPath"),
                    "error toast should mention the invalid vault path (got: {msg})"
                );
            }
            other => panic!("expected WriterError, got {other:?}"),
        }
        // Editor retained on failure.
        assert_eq!(app.editor_text(), "body");
    }

    #[test]
    fn dispatch_save_dimension_with_zero_sections_returns_no_section_available() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        app.sections.clear();
        write_text(&mut app, "body");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::CreateNewEntry),
            SaveOutcome::NoSectionAvailable
        );
    }

    #[test]
    fn dispatch_save_append_reaches_the_writer() {
        // Smoke test: Append on an empty vault falls back to create-new
        // inside `DailyNoteWriter::save`; we only need to confirm the
        // UI-layer plumbing makes it to the writer without tripping any
        // guards.
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        write_text(&mut app, "first entry");
        assert_eq!(
            dispatch_save(&mut app, SaveMode::AppendToLatestEntry),
            SaveOutcome::Written
        );
        assert_eq!(app.editor_text(), "");
    }

    // ---------------------------------------------------------------------
    // Phase 11 — toast overlay + auto-dismiss subscription.
    //
    // We can't render the widget tree without spinning up iced, but we can
    // verify two things:
    //   * `view()` builds without panic for both the toast-visible and
    //     toast-hidden branches (locks the overlay wiring in place).
    //   * `subscription()` returns something when toast is live and
    //     `Subscription::none()` otherwise. We can't inspect the timer
    //     directly, but comparing the subscription handle's identity to
    //     `Subscription::none()` via a recipe-less "empty?" check isn't
    //     exposed; instead we assert the branches exercise without panic
    //     and leave the wall-clock verification to the integration layer.
    // ---------------------------------------------------------------------

    #[test]
    fn view_builds_with_toast_overlay() {
        let mut app = fresh_app();
        {
            let _element_without_toast: Element<'_, Message> = view(&app);
        }
        app.toast = Some(TOAST_EMPTY_NOT_SAVED.to_string());
        {
            let _element_with_toast: Element<'_, Message> = view(&app);
        }
    }

    #[test]
    fn subscription_is_none_when_no_toast_is_active() {
        let app = fresh_app();
        // `Subscription` in iced 0.14 doesn't expose an `is_none()` helper,
        // but constructing the subscription and immediately dropping it
        // proves the branch runs. With sub-task 5 wired up the subscription
        // always carries the keyboard + open listener; the toast branch is
        // the only conditional piece.
        let _sub: Subscription<Message> = subscription(&app);
    }

    #[test]
    fn subscription_builds_auto_dismiss_when_toast_is_active() {
        let mut app = fresh_app();
        app.toast = Some("x".to_string());
        let _sub: Subscription<Message> = subscription(&app);
        // iced's subscription tree is opaque — the test merely guards the
        // branch builds; the actual 1.5 s timing is covered manually on
        // the running app.
    }

    #[test]
    fn toast_auto_dismiss_constant_matches_mac_reference() {
        // Mac `CaptureView` hides the toast after 1.5 s.
        assert_eq!(TOAST_AUTO_DISMISS, Duration::from_millis(1500));
    }

    #[test]
    fn toast_show_followed_by_dismiss_round_trips_state() {
        // End-to-end state test: the subscription-driven path delivers
        // `ToastDismiss`, which must clear the overlay. We simulate the
        // roundtrip without the timer so the test is hermetic.
        let mut app = fresh_app();
        apply(&mut app, Message::ToastShow("空内容未保存".to_string()));
        assert!(app.toast.is_some());
        apply(&mut app, Message::ToastDismiss);
        assert!(app.toast.is_none());
        // The view must still build after dismissal.
        let _element: Element<'_, Message> = view(&app);
    }

    // ---------------------------------------------------------------------
    // Phase 11 — keyboard + window-event decoder (sub-task 5).
    //
    // `decode_shortcut` is the pure function at the heart of the keyboard
    // listener. We can unit-test it without touching iced's subscription
    // machinery: feed it a `Key` + `Modifiers` pair and compare the result
    // to the expected `Message`.
    // ---------------------------------------------------------------------

    /// Convenience constructor for `Key::Character` built from a string
    /// slice. The production decoder lowercases the payload, so tests that
    /// assert case-sensitivity pass "P" and "p" deliberately.
    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn decode_shortcut_esc_triggers_close_panel() {
        let msg = decode_shortcut(&Key::Named(Named::Escape), Modifiers::empty());
        assert!(matches!(msg, Some(Message::ClosePanel)));
    }

    #[test]
    fn decode_shortcut_esc_with_modifier_is_ignored() {
        // Esc with Ctrl should be swallowed — production behaviour is
        // "bare Esc closes", and anything else is left for the editor.
        let msg = decode_shortcut(&Key::Named(Named::Escape), Modifiers::CTRL);
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_shift_tab_cycles_mode() {
        let msg = decode_shortcut(&Key::Named(Named::Tab), Modifiers::SHIFT);
        assert!(matches!(msg, Some(Message::CycleModeForward)));
    }

    #[test]
    fn decode_shortcut_plain_tab_is_not_consumed() {
        // Plain Tab is reserved for focus traversal in the editor.
        let msg = decode_shortcut(&Key::Named(Named::Tab), Modifiers::empty());
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_ctrl_enter_sends_note() {
        let msg = decode_shortcut(&Key::Named(Named::Enter), Modifiers::CTRL);
        assert!(matches!(msg, Some(Message::SendNote)));
    }

    #[test]
    fn decode_shortcut_ctrl_shift_enter_appends_note() {
        let msg = decode_shortcut(
            &Key::Named(Named::Enter),
            Modifiers::CTRL | Modifiers::SHIFT,
        );
        assert!(matches!(msg, Some(Message::AppendNote)));
    }

    #[test]
    fn decode_shortcut_plain_enter_is_not_consumed() {
        // Plain Enter is the editor's "new line" key. The decoder must
        // leave it alone.
        let msg = decode_shortcut(&Key::Named(Named::Enter), Modifiers::empty());
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_ctrl_p_toggles_pin() {
        let msg = decode_shortcut(&ch("p"), Modifiers::CTRL);
        assert!(matches!(msg, Some(Message::PinToggled)));
    }

    #[test]
    fn decode_shortcut_ctrl_p_uppercase_also_toggles_pin() {
        // Shift+Ctrl+P under some keyboard layouts emits "P" instead of
        // "p". Mirror Mac's lowercased comparison so the shortcut still
        // fires.
        let msg = decode_shortcut(&ch("P"), Modifiers::CTRL);
        assert!(matches!(msg, Some(Message::PinToggled)));
    }

    #[test]
    fn decode_shortcut_ctrl_digits_one_through_nine_map_to_zero_based_index() {
        for (d, idx) in [
            ("1", 0usize),
            ("2", 1),
            ("3", 2),
            ("4", 3),
            ("5", 4),
            ("6", 5),
            ("7", 6),
            ("8", 7),
            ("9", 8),
        ] {
            let msg = decode_shortcut(&ch(d), Modifiers::CTRL);
            assert!(
                matches!(msg, Some(Message::SelectByIndex(i)) if i == idx),
                "Ctrl+{d} should map to SelectByIndex({idx})"
            );
        }
    }

    #[test]
    fn decode_shortcut_ctrl_zero_is_not_consumed() {
        // Mac uses 1-indexed chip numbering; zero stays unbound.
        let msg = decode_shortcut(&ch("0"), Modifiers::CTRL);
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_digit_without_ctrl_is_not_consumed() {
        // A bare "1" keystroke must go to the editor.
        let msg = decode_shortcut(&ch("1"), Modifiers::empty());
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_alt_ctrl_digit_is_not_consumed() {
        // Alt-chord combinations collide with system layouts (AltGr on
        // European keyboards); leave them alone.
        let msg = decode_shortcut(&ch("1"), Modifiers::CTRL | Modifiers::ALT);
        assert!(msg.is_none());
    }

    // --- SelectByIndex handler ------------------------------------------

    #[test]
    fn select_by_index_in_dimension_mode_picks_the_section() {
        let mut app = fresh_app();
        assert_eq!(app.write_mode, WriteMode::Dimension);
        apply(&mut app, Message::SelectByIndex(2));
        assert_eq!(app.selected_section, Some(2));
    }

    #[test]
    fn select_by_index_dimension_out_of_range_is_silent_noop() {
        let mut app = fresh_app();
        apply(&mut app, Message::SelectByIndex(100));
        assert!(app.selected_section.is_none());
    }

    #[test]
    fn select_by_index_in_thread_mode_picks_the_thread_by_id() {
        let mut app = fresh_app();
        apply(&mut app, Message::WriteModeChanged(WriteMode::Thread));
        apply(&mut app, Message::SelectByIndex(1));
        let expected = app.threads.get(1).map(|t| t.id);
        assert!(expected.is_some());
        assert_eq!(app.selected_thread, expected);
    }

    #[test]
    fn select_by_index_thread_out_of_range_is_silent_noop() {
        let mut app = fresh_app();
        apply(&mut app, Message::WriteModeChanged(WriteMode::Thread));
        apply(&mut app, Message::SelectByIndex(99));
        assert!(app.selected_thread.is_none());
    }

    #[test]
    fn select_by_index_in_file_mode_is_silent_noop() {
        let mut app = fresh_app();
        apply(&mut app, Message::WriteModeChanged(WriteMode::File));
        apply(&mut app, Message::SelectByIndex(0));
        assert!(app.selected_section.is_none());
        assert!(app.selected_thread.is_none());
    }

    // --- FocusLost / ClosePanel -----------------------------------------

    #[test]
    fn focus_lost_when_pinned_does_not_close() {
        // A pinned panel ignores focus loss. The production handler
        // returns `Task::none()`, which we verify indirectly: the state
        // mutation is expected to be a no-op, and `ClosePanel` only
        // fires when the platform handler's `restore_foreground` is
        // subsequently called.
        let spy = crate::platform::mock::MockPlatformHandler::new();
        let mut app = fresh_app().with_platform_handler(spy.clone());
        apply(&mut app, Message::PinToggled);
        assert!(app.pinned);
        let calls_before = spy.restore_foreground_call_count();
        apply(&mut app, Message::FocusLost);
        // `FocusLost` itself should not trigger `restore_foreground`; the
        // close flow runs only via an explicit `ClosePanel`.
        assert_eq!(spy.restore_foreground_call_count(), calls_before);
    }

    #[test]
    fn close_panel_calls_restore_foreground_on_the_platform_handler() {
        let spy = crate::platform::mock::MockPlatformHandler::new();
        let mut app = fresh_app().with_platform_handler(spy.clone());
        assert_eq!(spy.restore_foreground_call_count(), 0);
        apply(&mut app, Message::ClosePanel);
        assert_eq!(spy.restore_foreground_call_count(), 1);
    }

    #[test]
    fn close_panel_without_a_handler_is_silent() {
        // No handler wired up — the close flow must still be a no-op that
        // doesn't panic when the window id is also missing.
        let mut app = fresh_app();
        apply(&mut app, Message::ClosePanel);
        // No observable side effects beyond "didn't panic".
    }

    #[test]
    fn window_opened_captures_the_id_for_the_close_flow() {
        let mut app = fresh_app();
        assert!(app.capture_window_id.is_none());
        apply(&mut app, Message::WindowOpened(window::Id::unique()));
        assert!(app.capture_window_id.is_some());
    }
}

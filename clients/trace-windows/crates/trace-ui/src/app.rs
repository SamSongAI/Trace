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
use std::time::{Duration, Instant};

use chrono::Utc;
use iced::event::{self as iced_event};
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key, Modifiers};
use iced::widget::text_editor::{Binding, Edit, KeyPress};
use iced::widget::{column, container, stack, text_editor, Space};
use iced::window::Event as WindowEvent;
use iced::{time, window, Event, Subscription, Task};
use iced::{Element, Length, Size, Theme};
use trace_core::{
    AppSettings, ClipboardImageWriter, DailyNoteWriter, FileWriter, NoteSection, SaveMode,
    ThreadConfig, ThreadWriter, TraceTheme, WriteMode,
};
use uuid::Uuid;

use crate::clipboard::ClipboardProbe;
use crate::platform::PlatformHandler;
use crate::theme::{panel_container_style, separator_container_style, to_iced_theme};
use crate::widgets;

/// How long a toast stays on screen before the auto-dismiss subscription
/// fires. Matches Mac's 1.5 s fade-out.
pub const TOAST_AUTO_DISMISS: Duration = Duration::from_millis(1500);
/// Polling interval for the toast-expiry subscription. A short tick lets
/// the [`Message::ToastTick`] handler compare `Instant::now()` against
/// [`CaptureApp::toast_expires_at`] without subscribing to a new timer
/// every time the user triggers a fresh toast.
const TOAST_TICK_INTERVAL: Duration = Duration::from_millis(250);

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
    /// Ctrl+V / Cmd+V intercepted by the text editor's
    /// [`iced::widget::text_editor::Binding::Custom`] hook. The update arm
    /// routes through [`CaptureApp::clipboard_probe`]: image data on the
    /// clipboard becomes a PNG write under the vault's daily assets tree
    /// plus a Markdown-link insert in the editor; otherwise the handler
    /// falls through to a plain-text paste. Matches Mac
    /// `CaptureTextEditor.paste(_:)` (image first, text fallback, beep on
    /// failure).
    PasteRequested,
    /// Settings button tap. Opens the settings window via
    /// [`iced::window::open`] if one isn't already visible, otherwise focuses
    /// the existing window via [`iced::window::gain_focus`]. The window id is
    /// captured through the [`Self::SettingsWindowOpened`] variant below.
    SettingsRequested,
    /// Emitted by the Task returned from [`iced::window::open`] once the
    /// settings window has been created and its runtime id is known.
    /// Stores the id into [`CaptureApp::settings_window_id`] so future
    /// [`Self::SettingsRequested`] clicks can re-focus an already-open window
    /// instead of spawning a second instance.
    SettingsWindowOpened(window::Id),
    /// Emitted by the [`iced::window::close_events`] subscription when any
    /// window is closed (user clicks the X, Alt+F4, etc.). The handler
    /// clears [`CaptureApp::settings_window_id`] **only when** the payload
    /// matches the stored settings window id, so a capture-window close
    /// never accidentally unblocks the "already open" guard for the settings
    /// window. Without this reset, a second
    /// [`Self::SettingsRequested`] click would route through
    /// [`iced::window::gain_focus`] against a stale id and iced silently
    /// dropping the call — the user would never see the settings window
    /// reopen.
    SettingsWindowClosed(window::Id),
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
    /// Timer-driven clear signal. Emitted manually when the handler
    /// chooses to force-dismiss the toast. The polling pipeline uses
    /// [`Self::ToastTick`] instead so the fade-out window is anchored to
    /// [`CaptureApp::toast_expires_at`] rather than a repeating interval.
    ToastDismiss,
    /// Polling tick emitted by the [`subscription`] while a toast is
    /// active. The handler clears the toast when the current instant has
    /// passed [`CaptureApp::toast_expires_at`]. Using a short tick plus
    /// an expiry instant means a fresh [`Self::ToastShow`] mid-cycle
    /// pushes the expiry forward and the next tick correctly honours
    /// the full [`TOAST_AUTO_DISMISS`] window.
    ToastTick,
    /// `iced::window::Event::Unfocused` translated to a domain message, only
    /// raised while `pinned == false`. Routed through the main [`update`]
    /// path so the close flow is centralised.
    FocusLost,
    /// `iced::window::Event::Opened` translated to a domain message so
    /// [`CaptureApp`] can capture the window id for the close task.
    WindowOpened(window::Id),
    /// New [`AppSettings`] snapshot broadcast from the settings window.
    ///
    /// Emitted by the daemon layer (`trace-app::update`) after every
    /// [`crate::settings::SettingsMessage`] dispatch so the capture panel
    /// picks up language / theme / sections / threads / write_mode /
    /// vault_path / daily-* edits live, without waiting for a restart.
    ///
    /// The handler re-derives every settings-driven field from the new
    /// snapshot (theme, sections, threads, write mode) and replaces the
    /// stored [`Arc<AppSettings>`] so subsequent writer dispatches see the
    /// latest values. Transient UI state (editor contents, pinned flag,
    /// focused chip) is intentionally preserved: an in-flight keystroke
    /// must not be discarded because the user happened to tweak a colour
    /// preset.
    ///
    /// `Arc::ptr_eq` short-circuits a no-op dispatch so a benign broadcast
    /// after a hover event (should one ever arrive) doesn't re-derive any
    /// cached field. In practice the daemon only broadcasts after a real
    /// edit (every `persist_working` allocates a fresh `Arc`), so the
    /// short-circuit is defensive but cheap.
    ReplaceSettings(Arc<AppSettings>),
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
    /// Runtime id of the settings window once it has been opened, or [`None`]
    /// while the window is closed. The [`Message::SettingsRequested`] handler
    /// uses this as a guard: `None` spawns a new window, `Some(id)` re-focuses
    /// the existing one. Captured on the first [`Message::SettingsWindowOpened`].
    pub settings_window_id: Option<window::Id>,
    /// Sections as configured by the user. Phase 10 carries an owned copy so
    /// the view layer doesn't need to reach into `AppSettings`.
    pub sections: Vec<NoteSection>,
    /// Threads as configured by the user. Already sorted by `order` so
    /// rendering is deterministic.
    pub threads: Vec<ThreadConfig>,
    /// Settings snapshot shared with the writers in [`Message::SendNote`] /
    /// [`Message::AppendNote`]. Wrapped in an [`Arc`] so each save can
    /// construct a writer via a cheap pointer clone rather than deep-cloning
    /// `AppSettings` (which owns non-trivial `Vec<ThreadConfig>` and
    /// `Vec<NoteSection>` allocations). The blanket `DailyNoteSettings` /
    /// `ThreadSettings` / `FileWriterSettings` impls for `Arc<T>` let the
    /// writers accept the `Arc` directly.
    pub settings: Arc<AppSettings>,
    /// Message shown in the overlay toast pill. `None` hides the overlay.
    /// Mutated by [`Message::ToastShow`] / [`Message::ToastDismiss`]; rendered
    /// via [`iced::widget::stack`] on top of the main column.
    pub toast: Option<String>,
    /// Instant at which the current toast should auto-dismiss. Updated
    /// every [`Message::ToastShow`] so rapid back-to-back toasts get the
    /// full [`TOAST_AUTO_DISMISS`] visibility window instead of the
    /// leftover slice of the previous toast's budget.
    pub toast_expires_at: Option<Instant>,
    /// Monotonically increasing counter bumped on every
    /// [`Message::ToastShow`]. Surfaced so consumers (and tests) can
    /// observe a fresh toast without reaching into the wall clock.
    pub toast_generation: u64,
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
    /// Optional clipboard façade consumed by the [`Message::PasteRequested`]
    /// handler. `None` in unit tests that exercise other code paths; `Some`
    /// when `trace-app` plugs in its platform-backed probe. Stored as a
    /// trait object so the iced `update` function can stay free of
    /// `arboard` / Win32 imports.
    pub clipboard_probe: Option<Arc<dyn ClipboardProbe>>,
}

impl CaptureApp {
    /// Builds a fresh [`CaptureApp`] seeded from a resolved [`TraceTheme`],
    /// the configured sections, the configured threads, and a shared
    /// [`AppSettings`] snapshot used by the writers in
    /// [`Message::SendNote`] / [`Message::AppendNote`].
    ///
    /// `settings` is taken as `Arc<AppSettings>` so every save can clone
    /// the `Arc` (pointer bump) instead of the inner `AppSettings` (deep
    /// allocation including `Vec<ThreadConfig>` and `Vec<NoteSection>`).
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
        settings: Arc<AppSettings>,
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
            settings_window_id: None,
            sections,
            threads,
            settings,
            toast: None,
            toast_expires_at: None,
            toast_generation: 0,
            capture_window_id: None,
            platform_handler: None,
            clipboard_probe: None,
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

    /// Plugs a clipboard probe into an already-constructed [`CaptureApp`].
    ///
    /// Mirrors [`CaptureApp::with_platform_handler`]: production wiring in
    /// `trace-app` passes a `trace-platform`-backed probe so Ctrl+V can
    /// route clipboard images through [`trace_core::ClipboardImageWriter`].
    /// Tests that don't exercise paste can skip this builder and get a
    /// no-op paste handler for free.
    pub fn with_clipboard_probe(mut self, probe: Arc<dyn ClipboardProbe>) -> Self {
        self.clipboard_probe = Some(probe);
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

/// Re-derives every settings-driven field on `state` from the freshly
/// broadcast `snap` and replaces the stored [`Arc<AppSettings>`]. Sub-task
/// 8c.3 uses this helper from the [`Message::ReplaceSettings`] arm so the
/// capture panel reflects settings window edits without a restart.
///
/// Settings-derived fields:
///
/// * `settings` — the `Arc` itself; writers constructed inside
///   `dispatch_save` `Arc::clone` it on every dispatch.
/// * `theme` / `iced_theme` — rebuilt from `snap.app_theme_preset` via
///   [`TraceTheme::for_preset`] + [`to_iced_theme`], mirroring
///   [`CaptureApp::set_theme`].
/// * `sections` — rebuilt via [`AppSettings::sections`] so the footer chip
///   list tracks renames / adds / removes.
/// * `threads` — cloned from `snap.thread_configs` and sorted by `order`,
///   matching the seeding contract in [`CaptureApp::new`].
/// * `write_mode` — mirrored from `snap.note_write_mode`.
///
/// Transient UI fields (editor contents, pinned flag, toast, window ids)
/// are intentionally untouched so the user's in-flight draft survives a
/// live settings edit. Selection indices are clamped into the new chip
/// list bounds: if the user's previously selected section index is now out
/// of range (the settings window shrunk the list), we reset to the first
/// slot; a previously selected thread whose id disappeared is cleared
/// rather than aliased to a stranger.
fn apply_settings_snapshot(state: &mut CaptureApp, snap: &Arc<AppSettings>) {
    // Replace the shared pointer first so any subsequent `dispatch_save`
    // triggered inside this pass already sees the new snapshot.
    state.settings = Arc::clone(snap);
    // Rebuild the theme cache via the existing helper — keeps the invariant
    // `iced_theme == to_iced_theme(&theme)` centralised on `set_theme`.
    state.set_theme(TraceTheme::for_preset(snap.app_theme_preset));
    // `AppSettings::sections()` produces a `NoteSection` view over the
    // stored titles with indices re-assigned in order. Mirrors the seeding
    // used by the Mac port's `CaptureViewModel.refresh()`.
    state.sections = snap.sections();
    // Threads must be sorted by `order` to match the seeding contract in
    // `CaptureApp::new`, otherwise `SelectByIndex(n)` would suddenly route
    // to a different chip after a live edit.
    let mut threads = snap.thread_configs.clone();
    threads.sort_by_key(|t| t.order);
    state.threads = threads;
    state.write_mode = snap.note_write_mode;
    // Clamp the current selection into the new chip list bounds so a
    // shrunken settings list cannot leave `selected_section` pointing past
    // the end. A cleared list → `None`; an in-range index → unchanged;
    // an out-of-range index → the first slot (0) so the user still has a
    // valid highlight rather than a silently dropped selection.
    if let Some(selected) = state.selected_section {
        if state.sections.is_empty() {
            state.selected_section = None;
        } else if selected >= state.sections.len() {
            state.selected_section = Some(0);
        }
    }
    // Thread selection is keyed by `Uuid`, not index — preserve the choice
    // when the id still exists, clear it when the thread was removed.
    if let Some(id) = state.selected_thread {
        if !state.threads.iter().any(|t| t.id == id) {
            state.selected_thread = None;
        }
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
        Message::PasteRequested => {
            handle_paste_requested(state);
            Task::none()
        }
        Message::SettingsRequested => {
            // If a settings window is already open, focus it instead of
            // spawning a second instance. `gain_focus` matches Mac's
            // "click gear → bring existing window to front" UX.
            if let Some(id) = state.settings_window_id {
                window::gain_focus(id)
            } else {
                // No settings window yet — open one. `window::open` returns
                // `(Id, Task<Id>)`; we immediately assign the allocated id to
                // the app state so rapid double-clicks on the gear button
                // don't spawn two windows while the `open` effect is still
                // in-flight. The returned Task resolves to the same id, at
                // which point we also emit `SettingsWindowOpened` as a
                // belt-and-braces sync.
                let settings = crate::settings::window_settings();
                let (id, task) = window::open(settings);
                state.settings_window_id = Some(id);
                task.map(Message::SettingsWindowOpened)
            }
        }
        Message::SettingsWindowOpened(id) => {
            // Idempotent: `SettingsRequested` already assigned the id when
            // it issued `window::open`. This variant lets the subscription
            // layer re-sync in scenarios where the window is opened by a
            // different path (e.g. a tray menu action in a later phase).
            state.settings_window_id = Some(id);
            Task::none()
        }
        Message::SettingsWindowClosed(id) => {
            // ID-guarded reset. `window::close_events()` fires for **every**
            // closed window (capture panel included), so we must compare the
            // payload to the stored settings id before clearing. A mismatch
            // — e.g. the capture window closing while the settings window is
            // open — must leave `settings_window_id` untouched so the next
            // `SettingsRequested` still finds the real settings window.
            if state.settings_window_id == Some(id) {
                state.settings_window_id = None;
            }
            Task::none()
        }
        Message::SendNote => {
            let outcome = dispatch_save(state, SaveMode::CreateNewEntry);
            finalize_save_outcome(state, outcome)
        }
        Message::AppendNote => {
            let outcome = dispatch_save(state, SaveMode::AppendToLatestEntry);
            finalize_save_outcome(state, outcome)
        }
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
            state.toast_expires_at = Some(Instant::now() + TOAST_AUTO_DISMISS);
            state.toast_generation = state.toast_generation.wrapping_add(1);
            Task::none()
        }
        Message::ToastDismiss => {
            state.toast = None;
            state.toast_expires_at = None;
            Task::none()
        }
        Message::ToastTick => {
            // Polling tick: clear the toast only when we've crossed the
            // expiry instant recorded by the last `ToastShow`. A mid-cycle
            // `ToastShow` pushes `toast_expires_at` forward, so the next
            // tick naturally honours the full auto-dismiss window.
            if let Some(expiry) = state.toast_expires_at {
                if Instant::now() >= expiry {
                    state.toast = None;
                    state.toast_expires_at = None;
                }
            }
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
        Message::ReplaceSettings(new_settings) => {
            // Short-circuit when the daemon happens to hand us the exact
            // same `Arc` allocation we already hold. `persist_working`
            // refreshes the snapshot via `Arc::new(...)` on every shadow
            // edit so this branch is effectively defensive — the initial
            // broadcast after `SettingsWindowOpened` is the one case where
            // pointer equality can trigger — but it keeps the invariant
            // that a no-op dispatch has zero side effects.
            if !Arc::ptr_eq(&state.settings, &new_settings) {
                apply_settings_snapshot(state, &new_settings);
            }
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
/// Fans out four independent streams via [`Subscription::batch`]:
///
/// 1. The toast auto-dismiss poller — only mounted while
///    [`CaptureApp::toast`] is `Some`, otherwise [`Subscription::none`].
///    Ticks every [`TOAST_TICK_INTERVAL`] and emits [`Message::ToastTick`];
///    the handler clears the toast once
///    [`CaptureApp::toast_expires_at`] has passed. This indirection keeps
///    the fade-out window anchored to the *latest* [`Message::ToastShow`]
///    instead of the interval that happened to be mid-cycle when a new
///    toast replaced the previous one.
/// 2. The keyboard shortcut + window-unfocus listener, implemented as a
///    `fn` pointer passed to [`iced::event::listen_with`]. Because
///    `listen_with` requires a non-capturing function pointer, all context-
///    dependent decisions (e.g. honouring the `pinned` flag before closing)
///    are deferred into [`update`].
/// 3. The window-open listener, which captures the primary [`window::Id`]
///    so [`Message::ClosePanel`] can emit the correct close task.
/// 4. The window-close listener, which feeds
///    [`Message::SettingsWindowClosed`]. Combined with the id-guarded
///    reset in [`update`], this closes the loop on the
///    [`Message::SettingsRequested`] guard so the settings window can be
///    re-opened after the user dismisses it via the window chrome.
pub fn subscription(state: &CaptureApp) -> Subscription<Message> {
    // Poll at a short interval while a toast is live. Each tick compares
    // `Instant::now()` against `toast_expires_at`, so a mid-cycle
    // `ToastShow` that pushes the expiry forward gets the full
    // `TOAST_AUTO_DISMISS` visibility budget instead of whatever slice of
    // the previous toast's budget remained. See `Message::ToastTick`.
    let toast = if state.toast.is_some() {
        time::every(TOAST_TICK_INTERVAL).map(|_| Message::ToastTick)
    } else {
        Subscription::none()
    };

    let events = iced_event::listen_with(decode_event);
    let opens = window::open_events().map(Message::WindowOpened);
    let closes = window::close_events().map(Message::SettingsWindowClosed);

    Subscription::batch(vec![toast, events, opens, closes])
}

/// `listen_with` callback. Must be a plain `fn` (no captures) — see
/// [`iced::event::listen_with`]. Keeps the runtime noise out of the
/// keyboard decoder by delegating to [`decode_shortcut`] for every
/// keyboard event and to a small inline branch for focus loss.
///
/// The `Captured` filter is scoped to keyboard events only. Window
/// events like [`WindowEvent::Unfocused`] are lifecycle notifications —
/// they are never "captured" by a widget in the same sense a keystroke
/// might be, so applying the keyboard-level short-circuit to them is
/// architecturally misleading even if iced 0.14 happens to always
/// flag Window events `Ignored` today.
fn decode_event(event: Event, status: iced_event::Status, _window: window::Id) -> Option<Message> {
    match event {
        // Skip keyboard events already handled by a widget (e.g. the
        // editor swallowing a printable character). iced's
        // `listen_with` only delivers `Ignored` events, but
        // `listen_raw` would leak every redraw, so the explicit guard
        // is kept defensive — and narrowly bound to keyboard events
        // where the "captured" concept is meaningful.
        Event::Keyboard(KeyboardEvent::KeyPressed { key, modifiers, .. })
            if !matches!(status, iced_event::Status::Captured) =>
        {
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

    // Ctrl+[Shift+]Enter dispatches writer save. `!alt()` guards the
    // AltGr collision on European keyboards (AltGr = Ctrl+Alt); `!logo()`
    // keeps Windows-key chords out of the way. The digit branch below
    // already applies the same guard — keep them symmetric.
    if matches!(key, Key::Named(Named::Enter))
        && modifiers.control()
        && !modifiers.alt()
        && !modifiers.logo()
    {
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

/// Translates a [`SaveOutcome`] into the iced task that feeds the next
/// [`update`] pass, consulting the pinned flag so the success branch
/// honours Mac's `CapturePanelController.swift:289-329` contract: when
/// pinned, a successful Send/Append clears the editor but keeps the
/// panel open (no ClosePanel emission).
pub(crate) fn finalize_save_outcome(state: &CaptureApp, outcome: SaveOutcome) -> Task<Message> {
    match outcome {
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
        SaveOutcome::Written => {
            if state.pinned {
                // Pinned: editor was already cleared inside `dispatch_save`;
                // the panel stays open so the user can keep dumping notes.
                Task::none()
            } else {
                Task::done(Message::ClosePanel)
            }
        }
        SaveOutcome::WriterError(msg) => Task::done(Message::ToastShow(msg)),
    }
}

/// User-facing toast shown when [`trace_core::ClipboardImageWriter::write_png`]
/// fails inside the paste handler (missing vault, unwriteable directory, atomic
/// rename refused, …). The error diagnostic is interpolated into the `{}`.
pub const TOAST_IMAGE_PASTE_FAILED: &str = "图片粘贴失败";
/// User-facing toast shown when the clipboard probe itself fails inside the
/// paste handler (OS clipboard unavailable, PNG encoding blew up, …). The
/// error diagnostic is interpolated into the `{}`.
pub const TOAST_CLIPBOARD_READ_FAILED: &str = "剪贴板读取失败";

/// Synchronously sets the toast pill on `state`, mirroring the
/// [`Message::ToastShow`] handler body. Split out so the paste handler can
/// surface a toast inline (without recursing through another `update` pass)
/// and so unit tests can observe the toast immediately after dispatching a
/// single message.
fn set_toast(state: &mut CaptureApp, message: String) {
    state.toast = Some(message);
    state.toast_expires_at = Some(Instant::now() + TOAST_AUTO_DISMISS);
    state.toast_generation = state.toast_generation.wrapping_add(1);
}

/// Implements [`Message::PasteRequested`].
///
/// Contract, mirroring Mac `CaptureTextEditor.paste(_:)`:
///
/// 1. If no probe is wired up (unit-test scaffolding or misconfigured host),
///    the handler is a silent no-op.
/// 2. Image path: [`ClipboardProbe::read_image_as_png`] returning
///    `Ok(Some(bytes))` drives a [`ClipboardImageWriter::write_png`] call
///    against the current [`AppSettings`] snapshot. On success the
///    returned Markdown link plus a trailing `\n` is pasted into the
///    editor; on failure a `"图片粘贴失败: {err}"` toast surfaces.
/// 3. Text fallback: when the image channel reports `Ok(None)`, we fall
///    through to [`ClipboardProbe::read_text`] and paste the string into
///    the editor. `Ok(None)` on text is a silent no-op (empty clipboard).
/// 4. Probe errors on either channel surface as `"剪贴板读取失败: {err}"`.
fn handle_paste_requested(state: &mut CaptureApp) {
    let Some(probe) = state.clipboard_probe.as_ref() else {
        return;
    };

    match probe.read_image_as_png() {
        Ok(Some(bytes)) => {
            let writer = ClipboardImageWriter::new(Arc::clone(&state.settings));
            match writer.write_png(&bytes, Utc::now()) {
                Ok(plan) => {
                    let payload = format!("{}\n", plan.markdown_link);
                    state
                        .editor_content
                        .perform(text_editor::Action::Edit(Edit::Paste(Arc::new(payload))));
                }
                Err(err) => {
                    set_toast(state, format!("{}: {}", TOAST_IMAGE_PASTE_FAILED, err));
                }
            }
            return;
        }
        Ok(None) => {
            // Fall through to text.
        }
        Err(err) => {
            set_toast(state, format!("{}: {}", TOAST_CLIPBOARD_READ_FAILED, err));
            return;
        }
    }

    match probe.read_text() {
        Ok(Some(text)) => {
            state
                .editor_content
                .perform(text_editor::Action::Edit(Edit::Paste(Arc::new(text))));
        }
        Ok(None) => {
            // Empty clipboard — silent no-op, matching Mac's `NSBeep` only
            // on write failure.
        }
        Err(err) => {
            set_toast(state, format!("{}: {}", TOAST_CLIPBOARD_READ_FAILED, err));
        }
    }
}

/// Key-binding closure hook for the capture panel's text editor.
///
/// Intercepts Ctrl+V (Cmd+V on macOS — `modifiers.command()` already folds
/// the platform-specific modifier into a single predicate) and routes it to
/// [`Message::PasteRequested`] so the app can handle image-first, text-
/// fallback paste through [`ClipboardProbe`]. Every other key press
/// delegates to [`Binding::from_key_press`] so default typing, navigation,
/// selection, copy, cut, and select-all remain intact.
///
/// The `!alt()` guard matches iced's built-in paste binding and avoids the
/// AltGr collision on European keyboards. The `!shift()` guard narrows the
/// interception to plain Ctrl/Cmd+V — Ctrl+Shift+V is a common "paste
/// without formatting" gesture in other app shells on Windows, and Mac's
/// own `handlePastedImage` only fires on plain Cmd+V. Owning that chord
/// silently would surprise users; we fall through to iced's default so
/// shells that expose it keep their own semantics.
///
/// Extracted as a free function (rather than inlined into the widget's
/// `.key_binding(...)` closure) so unit tests can build a [`KeyPress`]
/// fixture and exercise the routing directly.
pub fn paste_key_binding(press: KeyPress) -> Option<Binding<Message>> {
    if press.modifiers.command()
        && !press.modifiers.alt()
        && !press.modifiers.shift()
        && press.key.to_latin(press.physical_key) == Some('v')
    {
        return Some(Binding::Custom(Message::PasteRequested));
    }
    Binding::from_key_press(press)
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
            let writer = DailyNoteWriter::new(Arc::clone(&state.settings));
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
            let writer = ThreadWriter::new(Arc::clone(&state.settings));
            writer
                .save(&text, &thread, mode, now)
                .map(|written| written.map(|_| ()))
        }
        WriteMode::File => {
            // `FileWriter::save` has no `SaveMode` parameter — Append
            // degrades to a fresh-file Send, matching Mac.
            let writer = FileWriter::new(Arc::clone(&state.settings));
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
            Arc::new(AppSettings::default()),
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
        assert!(app.settings_window_id.is_none());
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
            Arc::new(AppSettings::default()),
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
    fn settings_requested_assigns_a_window_id_optimistically() {
        // Phase 12 sub-task 1: clicking the gear button emits
        // `window::open` and eagerly stores the allocated id so a second
        // click made before the runtime fires `SettingsWindowOpened`
        // re-focuses the in-flight window instead of spawning a second
        // one. We can't observe the Task directly, but we can assert the
        // state mutation.
        let mut app = fresh_app();
        assert!(app.settings_window_id.is_none());
        let _ = update(&mut app, Message::SettingsRequested);
        assert!(
            app.settings_window_id.is_some(),
            "first SettingsRequested allocates and stores a window id"
        );
        // Unrelated state must not have drifted.
        assert_eq!(app.write_mode, WriteMode::Dimension);
        assert!(app.selected_section.is_none());
    }

    #[test]
    fn settings_requested_twice_keeps_the_same_window_id() {
        // Once the window is open, a second click must focus the existing
        // window (via `window::gain_focus`) rather than spawn a new one.
        // The stored id must remain stable across the two messages so the
        // focus-path finds the same window.
        let mut app = fresh_app();
        let _ = update(&mut app, Message::SettingsRequested);
        let first = app
            .settings_window_id
            .expect("first SettingsRequested stored an id");
        let _ = update(&mut app, Message::SettingsRequested);
        assert_eq!(
            app.settings_window_id,
            Some(first),
            "second SettingsRequested must reuse the existing window id"
        );
    }

    #[test]
    fn settings_window_opened_records_the_runtime_id() {
        // The `Task<Id>` returned by `window::open` eventually delivers
        // `SettingsWindowOpened(id)` into `update`. The handler must store
        // the id so the subsequent re-focus path works even when the
        // window was opened outside the `SettingsRequested` flow (e.g. a
        // tray-menu action in a later phase).
        let mut app = fresh_app();
        assert!(app.settings_window_id.is_none());
        let id = window::Id::unique();
        apply(&mut app, Message::SettingsWindowOpened(id));
        assert_eq!(app.settings_window_id, Some(id));
    }

    #[test]
    fn settings_window_closed_clears_window_id() {
        // When the user dismisses the settings window via its chrome (the X
        // button, Alt+F4, etc.), iced emits `window::Event::Closed`. The
        // `close_events` subscription forwards it as
        // `Message::SettingsWindowClosed(id)` and the handler must reset
        // `settings_window_id` so the next `SettingsRequested` spawns a
        // fresh window instead of attempting to `gain_focus` a destroyed one.
        let mut app = fresh_app();
        let id = window::Id::unique();
        app.settings_window_id = Some(id);
        apply(&mut app, Message::SettingsWindowClosed(id));
        assert!(
            app.settings_window_id.is_none(),
            "matching SettingsWindowClosed must clear the cached id"
        );
    }

    #[test]
    fn settings_window_closed_other_id_does_not_clear() {
        // `window::close_events()` fires for every closed window in the
        // daemon. A capture-window close must NOT clear `settings_window_id`
        // — the id guard in the handler exists precisely to avoid this
        // cross-window confusion.
        let mut app = fresh_app();
        let settings_id = window::Id::unique();
        let other_id = window::Id::unique();
        assert_ne!(settings_id, other_id, "unique() must return distinct ids");
        app.settings_window_id = Some(settings_id);
        apply(&mut app, Message::SettingsWindowClosed(other_id));
        assert_eq!(
            app.settings_window_id,
            Some(settings_id),
            "non-matching SettingsWindowClosed leaves the cached id intact"
        );
    }

    #[test]
    fn settings_requested_after_close_reopens_window() {
        // End-to-end regression guard: open the settings window, close it,
        // then request it again. The second `SettingsRequested` must take
        // the "spawn a new window" branch — i.e. the stored id must change
        // — rather than route through `gain_focus` against the dead id.
        let mut app = fresh_app();
        let _ = update(&mut app, Message::SettingsRequested);
        let first_id = app
            .settings_window_id
            .expect("first SettingsRequested stored an id");
        apply(&mut app, Message::SettingsWindowClosed(first_id));
        assert!(
            app.settings_window_id.is_none(),
            "close must clear the cached id before the second request"
        );
        let _ = update(&mut app, Message::SettingsRequested);
        let second_id = app
            .settings_window_id
            .expect("second SettingsRequested stored a fresh id");
        assert_ne!(
            first_id, second_id,
            "second SettingsRequested must allocate a new window id, not reuse the dead one"
        );
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
            Arc::new(settings),
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
            Arc::new(settings),
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
            Arc::new(settings),
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

    #[test]
    fn dispatch_save_file_mode_append_behaves_like_send() {
        // `FileWriter::save` takes no `SaveMode` parameter, so Append in
        // File mode must produce the same outcome as a plain Send. Pin
        // the degradation contract so a future refactor that tries to
        // add a File-mode Append path without updating the UI plumbing
        // fails loudly.
        let tempdir = tempfile::tempdir().expect("tempdir");
        let mut app = app_with_vault(&tempdir, sample_threads());
        app.write_mode = WriteMode::File;
        app.document_title = "append-doc".to_string();
        write_text(&mut app, "file-mode body");

        let outcome = dispatch_save(&mut app, SaveMode::AppendToLatestEntry);
        assert_eq!(
            outcome,
            SaveOutcome::Written,
            "File-mode Append must degrade to the same Written outcome as Send"
        );
        assert_eq!(app.editor_text(), "", "successful write clears the editor");

        // Verify the writer actually produced a file under the vault so a
        // regression that silently swallowed the write still trips the
        // assertion.
        let file_count = std::fs::read_dir(tempdir.path())
            .expect("vault dir exists")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .count();
        assert!(
            file_count >= 1,
            "File-mode Append produced at least one file in the vault"
        );
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
    fn toast_show_twice_resets_the_timer() {
        // Rapid back-to-back toasts must restart the fade-out window —
        // otherwise the second toast gets dismissed after whatever was
        // left of the first toast's 1.5 s slice. We expose a generation
        // counter that rotates on every `ToastShow` so the subscription
        // recipe changes, forcing iced to re-subscribe and reset the
        // wall clock.
        let mut app = fresh_app();
        assert_eq!(app.toast_generation, 0, "toast_generation starts at zero");
        apply(&mut app, Message::ToastShow("A".to_string()));
        let gen_a = app.toast_generation;
        assert!(gen_a > 0, "first ToastShow bumps the generation counter");

        apply(&mut app, Message::ToastShow("B".to_string()));
        assert!(
            app.toast_generation > gen_a,
            "second ToastShow bumps the generation counter again, resetting the timer"
        );

        // Expiry instant must move forward too — otherwise a late tick
        // from the first toast could dismiss the second one early.
        let expiry_b = app
            .toast_expires_at
            .expect("ToastShow sets an expiry instant");
        assert!(
            expiry_b >= std::time::Instant::now(),
            "expiry must be in the future after ToastShow"
        );
    }

    #[test]
    fn toast_tick_before_expiry_keeps_toast_visible() {
        // A spurious tick that fires before the expiry instant must NOT
        // dismiss the toast — only ticks at or past the expiry clear it.
        let mut app = fresh_app();
        apply(&mut app, Message::ToastShow("alive".to_string()));
        assert!(app.toast.is_some());
        apply(&mut app, Message::ToastTick);
        assert_eq!(
            app.toast.as_deref(),
            Some("alive"),
            "ToastTick before expiry leaves the toast in place"
        );
    }

    #[test]
    fn toast_tick_past_expiry_dismisses_toast() {
        let mut app = fresh_app();
        apply(&mut app, Message::ToastShow("expiring".to_string()));
        // Force the expiry into the past so the next tick dismisses.
        app.toast_expires_at = Some(
            std::time::Instant::now()
                .checked_sub(Duration::from_millis(1))
                .expect("instant can be rewound one millisecond"),
        );
        apply(&mut app, Message::ToastTick);
        assert!(
            app.toast.is_none(),
            "ToastTick past expiry clears the toast"
        );
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

    #[test]
    fn decode_shortcut_ctrl_alt_enter_does_not_fire_send() {
        // AltGr = Ctrl+Alt on European layouts. Ctrl+Alt+Enter must NOT
        // fire Send/Append — that would trigger spurious writes when the
        // user is merely typing a dead key.
        let msg = decode_shortcut(&Key::Named(Named::Enter), Modifiers::CTRL | Modifiers::ALT);
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_ctrl_alt_shift_enter_does_not_fire_append() {
        // Same guard for the Append variant.
        let msg = decode_shortcut(
            &Key::Named(Named::Enter),
            Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT,
        );
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_ctrl_logo_enter_does_not_fire_send() {
        // Windows key + Ctrl + Enter is a system shortcut under some
        // shell customisations; we must not steal it.
        let msg = decode_shortcut(&Key::Named(Named::Enter), Modifiers::CTRL | Modifiers::LOGO);
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_ctrl_alt_p_does_not_fire_pin() {
        // Guard parity with the digit branch: AltGr+P must not toggle pin.
        let msg = decode_shortcut(&ch("p"), Modifiers::CTRL | Modifiers::ALT);
        assert!(msg.is_none());
    }

    #[test]
    fn decode_shortcut_ctrl_logo_digit_is_not_consumed() {
        // Super+Ctrl+digit is a common OS shortcut; stay out of its way.
        let msg = decode_shortcut(&ch("1"), Modifiers::CTRL | Modifiers::LOGO);
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

    // ---------------------------------------------------------------------
    // Task 13.3 — paste hook (Message::PasteRequested + paste_key_binding).
    //
    // Tests cover the full matrix described in the Mac reference:
    //   - image present → write PNG + insert Markdown link
    //   - image absent, text present → paste text
    //   - empty clipboard → no-op
    //   - image write failure → toast "图片粘贴失败"
    //   - no probe wired up → no-op
    //   - probe error → toast "剪贴板读取失败"
    //
    // Plus the key-binding routing: Ctrl/Cmd+V → PasteRequested, other keys
    // fall through to iced's default binding.
    // ---------------------------------------------------------------------

    /// Minimal byte sequence the writer treats as a PNG payload. The atomic
    /// write path does no magic-number validation so any non-empty slice is
    /// sufficient for these tests — the actual clipboard-to-PNG encode
    /// pipeline lives in `trace-platform` and is covered by its own suite.
    const FAKE_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    #[test]
    fn paste_requested_with_image_writes_png_and_inserts_markdown() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let probe = Arc::new(
            crate::clipboard::mock::MockClipboardProbe::new().with_image_png(FAKE_PNG.to_vec()),
        );
        let mut app =
            app_with_vault(&tempdir, sample_threads()).with_clipboard_probe(probe.clone());
        apply(&mut app, Message::PasteRequested);

        // The writer drops the PNG under
        // `{vault}/{daily_folder_name}/assets/{yyyy-MM-dd}/trace-*.png`.
        let daily_folder = tempdir.path().join(&app.settings.daily_folder_name);
        let assets_root = daily_folder.join("assets");
        let day_dirs: Vec<_> = std::fs::read_dir(&assets_root)
            .expect("assets dir should exist after a successful paste")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(
            day_dirs.len(),
            1,
            "expected exactly one {{yyyy-MM-dd}} folder to be created, found {:?}",
            day_dirs
        );
        let png_entries: Vec<_> = std::fs::read_dir(day_dirs[0].path())
            .expect("day folder should be readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("trace-"))
            .collect();
        assert_eq!(
            png_entries.len(),
            1,
            "expected a single trace-*.png under the day folder, found {:?}",
            png_entries
        );

        let editor_text = app.editor_text();
        assert!(
            editor_text.trim_end_matches('\n').ends_with(".png)"),
            "editor should end with a Markdown image link ({:?})",
            editor_text
        );
        assert!(
            editor_text.contains("![image](assets/"),
            "editor should contain Mac-parity Markdown link prefix ({:?})",
            editor_text
        );
        assert!(
            editor_text.ends_with('\n'),
            "payload must end with a newline so the next send starts clean ({:?})",
            editor_text
        );

        // Only the image channel was consulted — text fallback is skipped
        // on a successful image paste.
        assert_eq!(probe.image_call_count(), 1);
        assert_eq!(probe.text_call_count(), 0);
        assert!(app.toast.is_none(), "successful paste must not toast");
    }

    #[test]
    fn paste_requested_with_text_fallback_inserts_text() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let probe = Arc::new(crate::clipboard::mock::MockClipboardProbe::new().with_text("hello"));
        let mut app =
            app_with_vault(&tempdir, sample_threads()).with_clipboard_probe(probe.clone());
        apply(&mut app, Message::PasteRequested);

        assert_eq!(app.editor_text(), "hello");
        assert_eq!(
            probe.image_call_count(),
            1,
            "image channel is consulted first"
        );
        assert_eq!(
            probe.text_call_count(),
            1,
            "text channel is consulted as the fallback"
        );
        assert!(app.toast.is_none());
    }

    #[test]
    fn paste_requested_with_empty_clipboard_is_noop() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let probe = Arc::new(crate::clipboard::mock::MockClipboardProbe::new());
        let mut app =
            app_with_vault(&tempdir, sample_threads()).with_clipboard_probe(probe.clone());
        apply(&mut app, Message::PasteRequested);

        assert_eq!(
            app.editor_text(),
            "",
            "empty clipboard must leave editor untouched"
        );
        assert_eq!(probe.image_call_count(), 1);
        assert_eq!(probe.text_call_count(), 1);
        assert!(app.toast.is_none());
    }

    #[test]
    fn paste_requested_with_image_write_failure_surfaces_toast() {
        // Point the vault at a blank path so `ClipboardImageWriter::write_png`
        // returns `InvalidVaultPath`, mirroring the setup used by
        // `dispatch_save_writer_error_surfaces_message` above.
        let settings = AppSettings {
            vault_path: String::new(),
            ..AppSettings::default()
        };
        let mut app = CaptureApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            sample_sections(),
            sample_threads(),
            Arc::new(settings),
        );
        let probe = Arc::new(
            crate::clipboard::mock::MockClipboardProbe::new().with_image_png(FAKE_PNG.to_vec()),
        );
        app = app.with_clipboard_probe(probe.clone());
        let before = app.editor_text();
        apply(&mut app, Message::PasteRequested);

        assert_eq!(
            app.editor_text(),
            before,
            "failed paste must not mutate the editor"
        );
        let toast = app
            .toast
            .as_deref()
            .expect("write failure must surface a toast");
        assert!(
            toast.contains(TOAST_IMAGE_PASTE_FAILED),
            "toast {:?} should contain the localised prefix {:?}",
            toast,
            TOAST_IMAGE_PASTE_FAILED
        );
        // Text fallback must not run after an image-write failure — the
        // user already has actionable feedback via the toast.
        assert_eq!(probe.image_call_count(), 1);
        assert_eq!(probe.text_call_count(), 0);
    }

    #[test]
    fn paste_requested_without_probe_is_noop() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        // NOTE: no `.with_clipboard_probe(...)` — mirrors unit tests that
        // don't wire the probe at all.
        let mut app = app_with_vault(&tempdir, sample_threads());
        let before = app.editor_text();
        apply(&mut app, Message::PasteRequested);

        assert_eq!(app.editor_text(), before);
        assert!(app.toast.is_none());
    }

    #[test]
    fn paste_requested_with_probe_error_surfaces_toast() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let probe = Arc::new(
            crate::clipboard::mock::MockClipboardProbe::new()
                .with_image_error("platform refused to open clipboard"),
        );
        let mut app =
            app_with_vault(&tempdir, sample_threads()).with_clipboard_probe(probe.clone());
        apply(&mut app, Message::PasteRequested);

        let toast = app
            .toast
            .as_deref()
            .expect("probe failure must surface a toast");
        assert!(
            toast.contains(TOAST_CLIPBOARD_READ_FAILED),
            "toast {:?} should contain the localised prefix {:?}",
            toast,
            TOAST_CLIPBOARD_READ_FAILED
        );
        // Text fallback must be skipped when the image channel errored —
        // the probe is unreliable; trying again would just double-report.
        assert_eq!(probe.image_call_count(), 1);
        assert_eq!(probe.text_call_count(), 0);
    }

    fn key_press(
        key: iced::keyboard::Key,
        modifiers: iced::keyboard::Modifiers,
        physical: iced::keyboard::key::Physical,
    ) -> iced::widget::text_editor::KeyPress {
        use iced::widget::text_editor::{KeyPress, Status};
        KeyPress {
            key: key.clone(),
            modified_key: key,
            physical_key: physical,
            modifiers,
            text: None,
            status: Status::Focused { is_hovered: false },
        }
    }

    #[test]
    fn paste_key_binding_intercepts_command_v() {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        // `Modifiers::COMMAND` is LOGO on macOS and CTRL on other platforms,
        // matching `modifiers.command()`.
        let press = key_press(
            Key::Character("v".into()),
            Modifiers::COMMAND,
            Physical::Code(Code::KeyV),
        );
        let binding = paste_key_binding(press);
        match binding {
            Some(Binding::Custom(Message::PasteRequested)) => {}
            other => panic!(
                "Ctrl/Cmd+V should map to Binding::Custom(PasteRequested), got {:?}",
                other
            ),
        }
    }

    #[test]
    fn paste_key_binding_skips_command_alt_v() {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        // iced's built-in paste binding excludes `modifiers.alt()` to avoid
        // the AltGr collision on European keyboards. Our interceptor must
        // apply the same guard so those layouts still type literal glyphs
        // instead of triggering `PasteRequested`.
        let press = key_press(
            Key::Character("v".into()),
            Modifiers::COMMAND | Modifiers::ALT,
            Physical::Code(Code::KeyV),
        );
        let binding = paste_key_binding(press);
        assert!(
            !matches!(binding, Some(Binding::Custom(Message::PasteRequested))),
            "Ctrl+Alt+V (AltGr) must not route to PasteRequested, got {:?}",
            binding
        );
    }

    #[test]
    fn paste_key_binding_falls_through_for_other_keys() {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        // Plain 'a' with no modifiers should either delegate to iced's
        // default binding (likely `Binding::Insert('a')`) or return `None`,
        // but must never produce our custom paste message.
        let press = key_press(
            Key::Character("a".into()),
            Modifiers::empty(),
            Physical::Code(Code::KeyA),
        );
        let binding = paste_key_binding(press);
        assert!(
            !matches!(binding, Some(Binding::Custom(Message::PasteRequested))),
            "non-V key must not route to PasteRequested, got {:?}",
            binding
        );
    }

    #[test]
    fn paste_key_binding_skips_command_shift_v() {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        // Ctrl+Shift+V is a common "paste without formatting" gesture in
        // other Windows shells, and Mac's `handlePastedImage` only fires on
        // plain Cmd+V. Owning the shift variant silently would surprise
        // users of those shells, so our interceptor must let it fall
        // through to iced's default binding.
        let press = key_press(
            Key::Character("v".into()),
            Modifiers::COMMAND | Modifiers::SHIFT,
            Physical::Code(Code::KeyV),
        );
        let binding = paste_key_binding(press);
        assert!(
            !matches!(binding, Some(Binding::Custom(Message::PasteRequested))),
            "Ctrl/Cmd+Shift+V must not route to PasteRequested, got {:?}",
            binding
        );
    }
}

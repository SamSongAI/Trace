//! Executable entry point for the Trace Windows client.
//!
//! Phase 12 sub-task 1 wires the two iced windows (capture + settings) behind
//! a single [`iced::daemon`] so message routing between them flows through one
//! top-level `update` without the callers in `trace-ui` having to know about
//! multi-window plumbing. The daemon model is a deliberate match for the
//! Trace shape: a background process that spawns windows on demand and
//! doesn't terminate when all windows close (a later phase attaches a tray
//! icon and global hotkey listener to the same daemon).
//!
//! # Routing model
//!
//! * [`TraceApp`] owns both sub-states: [`trace_ui::app::CaptureApp`] and an
//!   `Option<trace_ui::settings::SettingsApp>` (lazy — built only when the
//!   capture panel asks for it).
//! * [`Message`] is a `Capture(...)` / `Settings(...)` sum — every variant is
//!   routed to the sub-state it mutates.
//! * `view(state, window::Id)` dispatches on the id by comparing against
//!   [`CaptureApp::capture_window_id`] and
//!   [`TraceApp::settings_window_id`]. When no id has been recorded yet
//!   (first frame), the capture view is returned as the default.
//!
//! # Cross-window bridging
//!
//! The capture panel emits
//! [`trace_ui::app::Message::SettingsWindowOpened`] to record the id of the
//! newly-opened settings window. This top-level `update` intercepts the same
//! runtime event at the daemon level — when a `Message::Capture(...)` variant
//! carries the `SettingsWindowOpened` id we also lazily initialise the
//! [`trace_ui::settings::SettingsApp`] so the settings window's first frame
//! has valid state.
//!
//! Platform-specific wiring (tray icon, global hotkeys, Win32 topmost bit)
//! lands in later phases; Phase 12 only needs the multi-window shell.

use std::sync::Arc;

use iced::{window, Element, Subscription, Task, Theme};
use trace_core::{AppSettings, NoteSection, ThemePreset, ThreadConfig, TraceTheme};
use trace_ui::app::{self as capture_app, CaptureApp};
use trace_ui::settings::{self as settings_app, SettingsApp};

/// Messages for the top-level daemon. Tagged on the sub-state they mutate so
/// the `update` dispatcher can route without type-switching.
#[derive(Debug, Clone)]
enum Message {
    /// A message destined for the capture panel's `update` function.
    Capture(capture_app::Message),
    /// A message destined for the settings window's `update` function.
    Settings(settings_app::SettingsMessage),
}

/// Owns both sub-states so iced's `daemon` can render either window from the
/// same top-level closure. `settings` is lazy: we don't pay the
/// [`TraceTheme`] + [`AppSettings`] clone until the user actually opens the
/// settings window.
struct TraceApp {
    capture: CaptureApp,
    settings: Option<SettingsApp>,
    /// Shared settings snapshot. Held at the top level so we can hand the
    /// same `Arc` to a fresh `SettingsApp` when the capture panel asks for a
    /// new one without touching the `CaptureApp` internals.
    shared_settings: Arc<AppSettings>,
    /// Theme snapshot used to seed a fresh `SettingsApp`. Kept in sync with
    /// [`CaptureApp::theme`] so both windows render under the same preset.
    theme: TraceTheme,
}

impl TraceApp {
    fn new() -> (Self, Task<Message>) {
        // Seed defaults. Later phases read these from
        // `trace-platform`-resolved `AppSettings` on startup.
        let theme = TraceTheme::for_preset(ThemePreset::Dark);
        let shared_settings = Arc::new(AppSettings::default());
        let sections = default_sections();
        let threads: Vec<ThreadConfig> = Vec::new();

        let capture = CaptureApp::new(theme, sections, threads, Arc::clone(&shared_settings));

        (
            Self {
                capture,
                settings: None,
                shared_settings,
                theme,
            },
            Task::none(),
        )
    }
}

/// Seeds the capture panel with the default dimension sections. Later
/// phases load the user-configured list from persisted `AppSettings`.
fn default_sections() -> Vec<NoteSection> {
    (0..NoteSection::DEFAULT_TITLES.len())
        .map(|i| NoteSection::new(i, NoteSection::DEFAULT_TITLES[i]))
        .collect()
}

fn update(state: &mut TraceApp, message: Message) -> Task<Message> {
    match message {
        Message::Capture(capture_message) => {
            // Intercept the `SettingsWindowOpened` event so we can lazily
            // build the settings sub-state the first time the window opens.
            // The capture app still handles the same message so its own
            // `settings_window_id` cache stays in sync.
            if let capture_app::Message::SettingsWindowOpened(_) = &capture_message {
                if state.settings.is_none() {
                    state.settings = Some(SettingsApp::new(
                        state.theme,
                        Arc::clone(&state.shared_settings),
                    ));
                }
            }
            let task =
                capture_app::update(&mut state.capture, capture_message).map(Message::Capture);
            // After the capture layer has processed the message, mirror the
            // "settings window is gone" state at the top level. The capture
            // `update` clears `settings_window_id` when a matching close
            // event arrives; we take the same cue to drop the `SettingsApp`
            // so its memory is released and a subsequent `SettingsRequested`
            // rebuilds a fresh instance that reads the latest shared
            // `AppSettings`. Both fields must move to `None` together —
            // otherwise the next request would build a new window but reuse
            // a stale `SettingsApp` against the new id.
            if state.capture.settings_window_id.is_none() {
                state.settings = None;
            }
            task
        }
        Message::Settings(settings_message) => {
            if let Some(settings) = state.settings.as_mut() {
                settings_app::settings_update(settings, settings_message).map(Message::Settings)
            } else {
                // The capture panel is the only path that opens the settings
                // window, and we initialise `state.settings` on
                // `SettingsWindowOpened` (above). Reaching this branch means
                // a settings message arrived before the window opened —
                // safe to drop.
                Task::none()
            }
        }
    }
}

fn view(state: &TraceApp, window: window::Id) -> Element<'_, Message> {
    // Route by id. The settings window has priority because it's the one
    // that can be selectively open; the capture panel is the default
    // surface.
    if let Some(settings_id) = state.capture.settings_window_id {
        if window == settings_id {
            if let Some(settings) = state.settings.as_ref() {
                return settings_app::settings_view(settings).map(Message::Settings);
            }
        }
    }
    // Fallback: unknown `window_id` renders the capture view. Today the
    // daemon only owns two windows (capture + settings), so any id that
    // isn't the settings one must be the capture one. This is a **known
    // trade-off**: Phase 14's tray bubble window (if added) must extend
    // this routing before relying on the default branch — otherwise the
    // bubble would silently render the capture panel.
    capture_app::view(&state.capture).map(Message::Capture)
}

fn theme(state: &TraceApp, window: window::Id) -> Theme {
    if let Some(settings_id) = state.capture.settings_window_id {
        if window == settings_id {
            if let Some(settings) = state.settings.as_ref() {
                return settings_app::settings_theme(settings);
            }
        }
    }
    capture_app::theme(&state.capture)
}

fn title(state: &TraceApp, window: window::Id) -> String {
    if let Some(settings_id) = state.capture.settings_window_id {
        if window == settings_id {
            return trace_core::L10n::settings(state.shared_settings.language).to_string();
        }
    }
    // Capture panel uses a plain brand string; later phases may localise it.
    "Trace".to_string()
}

fn subscription(state: &TraceApp) -> Subscription<Message> {
    let capture = capture_app::subscription(&state.capture).map(Message::Capture);
    let settings = state
        .settings
        .as_ref()
        .map(|s| settings_app::settings_subscription(s).map(Message::Settings))
        .unwrap_or_else(Subscription::none);
    Subscription::batch(vec![capture, settings])
}

fn main() -> iced::Result {
    iced::daemon(TraceApp::new, update, view)
        .title(title)
        .theme(theme)
        .subscription(subscription)
        .run()
}

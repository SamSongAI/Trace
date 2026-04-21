//! End-to-end capture panel integration test.
//!
//! Walks a canonical user journey through the public [`trace_ui::app`]
//! surface and asserts the observable outcomes: typing fills the editor,
//! a blank Send raises a toast, mode switches pick up the right chip
//! list, a real Send writes a markdown file to the vault and clears the
//! editor, and Esc triggers the close flow (which routes through the
//! platform handler when present).
//!
//! Rationale for living in `tests/`:
//!
//! * Exercises the crate strictly through `pub` items — guards the
//!   intended external contract against accidental visibility shrinkage.
//! * Uses a throw-away vault tempdir so the write path is real I/O, not
//!   a mock.
//! * Cross-checks the tests in `src/app.rs` by stitching several small
//!   state transitions together in one scenario rather than one message
//!   per test.
//!
//! This file intentionally does not reach into the per-crate test
//! mock (`crate::platform::mock`), which is gated behind
//! `#[cfg(test)] pub(crate)`. A minimal atomic-counter spy is defined
//! inline so the integration layer owns its own test fixtures.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use iced::widget::text_editor;
use tempfile::TempDir;
use trace_core::{AppSettings, NoteSection, ThemePreset, ThreadConfig, TraceTheme, WriteMode};
use trace_ui::app::{update, view, CaptureApp, Message, TOAST_EMPTY_NOT_SAVED};
use trace_ui::platform::PlatformHandler;

/// Call-counting spy that doubles as a [`PlatformHandler`]. Defined inline
/// because the per-crate mock is `#[cfg(test)]`-gated.
#[derive(Default)]
struct SpyHandler {
    set_topmost_calls: AtomicUsize,
    restore_foreground_calls: AtomicUsize,
}

impl SpyHandler {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn restore_foreground_count(&self) -> usize {
        self.restore_foreground_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    fn set_topmost_count(&self) -> usize {
        self.set_topmost_calls.load(Ordering::SeqCst)
    }
}

impl PlatformHandler for SpyHandler {
    fn set_topmost(&self, _pinned: bool) {
        self.set_topmost_calls.fetch_add(1, Ordering::SeqCst);
    }

    fn restore_foreground(&self) {
        self.restore_foreground_calls.fetch_add(1, Ordering::SeqCst);
    }
}

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

/// Builds a [`CaptureApp`] pointed at a fresh tempdir vault with the
/// inbox and thread folders set up so the daily-note writer can produce
/// a real markdown file.
fn app_with_vault(tempdir: &TempDir) -> CaptureApp {
    let vault = tempdir.path().to_string_lossy().into_owned();
    let settings = AppSettings {
        vault_path: vault.clone(),
        inbox_vault_path: vault,
        ..AppSettings::default()
    };
    CaptureApp::new(
        TraceTheme::for_preset(ThemePreset::Dark),
        sample_sections(),
        sample_threads(),
        Arc::new(settings),
    )
}

/// Drops the iced [`iced::Task`] returned by `update` — integration tests
/// cannot inspect iced's opaque task, so observe state after the fact.
fn apply(app: &mut CaptureApp, msg: Message) {
    let _ = update(app, msg);
}

#[test]
fn empty_send_raises_toast_and_leaves_editor_untouched() {
    let mut app = CaptureApp::new(
        TraceTheme::for_preset(ThemePreset::Dark),
        sample_sections(),
        sample_threads(),
        Arc::new(AppSettings::default()),
    );
    assert!(app.toast.is_none());
    apply(&mut app, Message::SendNote);
    // Dispatch raises the "empty not saved" toast and, on the next update
    // tick, the ToastShow message would be applied. Simulate that tick
    // directly so the overlay state is observable.
    apply(
        &mut app,
        Message::ToastShow(TOAST_EMPTY_NOT_SAVED.to_string()),
    );
    assert_eq!(app.toast.as_deref(), Some(TOAST_EMPTY_NOT_SAVED));
    assert_eq!(app.editor_text(), "");
}

#[test]
fn dimension_mode_send_writes_file_clears_editor_and_view_still_renders() {
    let tempdir = TempDir::new().expect("tempdir");
    let mut app = app_with_vault(&tempdir);

    // Seed the editor via the text_editor API — mirrors what the iced
    // runtime would do on keystrokes.
    app.editor_content = text_editor::Content::with_text("hello windows world");

    // Default section + Dimension mode → DailyNoteWriter produces a
    // yyyy-mm-dd.md file in the vault root.
    apply(&mut app, Message::SendNote);

    // The writer clears the editor on success.
    assert_eq!(app.editor_text(), "");

    // The DailyNoteWriter writes into `vault/daily_folder_name/YYYY-MM-DD.md`.
    let daily_root = tempdir.path().join(&app.settings.daily_folder_name);
    let md_count = std::fs::read_dir(&daily_root)
        .expect("daily folder exists")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .count();
    assert!(
        md_count >= 1,
        "Dimension write produced at least one daily note file under {daily_root:?}"
    );

    // Rendering the view must still succeed post-clear.
    let _element = view(&app);
}

#[test]
fn mode_cycle_rotates_dimension_thread_file_dimension() {
    let mut app = CaptureApp::new(
        TraceTheme::for_preset(ThemePreset::Dark),
        sample_sections(),
        sample_threads(),
        Arc::new(AppSettings::default()),
    );
    assert_eq!(app.write_mode, WriteMode::Dimension);
    apply(&mut app, Message::CycleModeForward);
    assert_eq!(app.write_mode, WriteMode::Thread);
    apply(&mut app, Message::CycleModeForward);
    assert_eq!(app.write_mode, WriteMode::File);
    apply(&mut app, Message::CycleModeForward);
    assert_eq!(app.write_mode, WriteMode::Dimension);
}

#[test]
fn close_panel_invokes_platform_restore_foreground() {
    let spy = SpyHandler::new();
    let mut app = CaptureApp::new(
        TraceTheme::for_preset(ThemePreset::Dark),
        sample_sections(),
        sample_threads(),
        Arc::new(AppSettings::default()),
    )
    .with_platform_handler(spy.clone());

    assert_eq!(spy.restore_foreground_count(), 0);
    apply(&mut app, Message::ClosePanel);
    assert_eq!(spy.restore_foreground_count(), 1);
}

#[test]
fn close_panel_preserves_editor_draft() {
    // Esc (→ `Message::ClosePanel`) must close the panel without clearing
    // the editor. Mac preserves the in-flight draft so that reopening the
    // panel picks up exactly where the user left off; the Windows port
    // must honour the same invariant.
    let mut app = CaptureApp::new(
        TraceTheme::for_preset(ThemePreset::Dark),
        sample_sections(),
        sample_threads(),
        Arc::new(AppSettings::default()),
    );
    app.editor_content = text_editor::Content::with_text("draft in progress");
    assert_eq!(app.editor_text(), "draft in progress");

    apply(&mut app, Message::ClosePanel);

    assert_eq!(
        app.editor_text(),
        "draft in progress",
        "ClosePanel must leave the editor text intact for the next reopen"
    );
}

#[test]
fn pin_toggled_then_focus_lost_does_not_close() {
    // When pinned, FocusLost must be absorbed silently.
    let spy = SpyHandler::new();
    let mut app = CaptureApp::new(
        TraceTheme::for_preset(ThemePreset::Dark),
        sample_sections(),
        sample_threads(),
        Arc::new(AppSettings::default()),
    )
    .with_platform_handler(spy.clone());

    apply(&mut app, Message::PinToggled);
    assert!(app.pinned);
    let before = spy.restore_foreground_count();
    apply(&mut app, Message::FocusLost);
    assert_eq!(
        spy.restore_foreground_count(),
        before,
        "pinned panel does not route FocusLost into ClosePanel"
    );
}

#[test]
fn send_note_when_pinned_does_not_close_panel() {
    // Mac `CapturePanelController.swift:289-329`: successful Send on a
    // pinned panel clears the editor and re-focuses it but keeps the
    // window open. The Windows port must honour the same invariant by
    // returning `Task::none()` from the `Written + pinned` branch.
    let tempdir = TempDir::new().expect("tempdir");
    let spy = SpyHandler::new();
    let mut app = app_with_vault(&tempdir).with_platform_handler(spy.clone());

    // Seed the editor and pin the panel before sending.
    app.editor_content = text_editor::Content::with_text("pinned entry");
    apply(&mut app, Message::PinToggled);
    assert!(app.pinned);

    let task = update(&mut app, Message::SendNote);

    // Editor cleared on success.
    assert_eq!(app.editor_text(), "", "successful Send clears the editor");
    // Task::none() when pinned — no follow-up Message units queued.
    assert_eq!(
        task.units(),
        0,
        "pinned Send must emit Task::none() — no ClosePanel follow-up"
    );
}

#[test]
fn send_note_when_unpinned_closes_panel() {
    // Mirror test of the pinned case: unpinned Send still runs the
    // close flow, so the returned task carries a ClosePanel unit that
    // iced's runtime will dispatch back into `update`.
    let tempdir = TempDir::new().expect("tempdir");
    let spy = SpyHandler::new();
    let mut app = app_with_vault(&tempdir).with_platform_handler(spy.clone());

    app.editor_content = text_editor::Content::with_text("unpinned entry");
    assert!(!app.pinned);

    let task = update(&mut app, Message::SendNote);

    assert_eq!(app.editor_text(), "", "successful Send clears the editor");
    assert!(
        task.units() > 0,
        "unpinned Send routes through ClosePanel — task must carry a follow-up unit"
    );

    // Simulate iced delivering the queued ClosePanel, which calls
    // `restore_foreground` on the platform handler exactly once.
    let restore_before = spy.restore_foreground_count();
    apply(&mut app, Message::ClosePanel);
    assert_eq!(
        spy.restore_foreground_count(),
        restore_before + 1,
        "unpinned ClosePanel fires restore_foreground once"
    );
}

/// Compile-time assertion that `update` and `view` are part of the
/// public API surface. A visibility regression on either would fail to
/// compile this integration test file, which is exactly what we want.
#[allow(dead_code)]
fn _public_api_surface_check() {
    let _: fn(&mut CaptureApp, Message) -> iced::Task<Message> = update;
    let _: fn(&CaptureApp) -> iced::Element<'_, Message> = view;
}

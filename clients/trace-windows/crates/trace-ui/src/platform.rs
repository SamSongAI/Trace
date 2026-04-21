//! Platform-side hooks consumed by [`crate::app`].
//!
//! `trace-ui` is intentionally platform-free: it does not depend on
//! `trace-platform`, WinAPI, or any OS-specific crate. Effects that need the
//! operating system (toggling the window's topmost bit, restoring keyboard
//! focus to whatever app was frontmost before the capture panel appeared) are
//! described by the [`PlatformHandler`] trait and supplied by `trace-app` at
//! wire-up time.
//!
//! # Contract
//!
//! The UI calls the trait from within [`crate::app::update`], which runs on
//! iced's main thread. Implementations must therefore either complete quickly
//! or spawn their own worker — the UI does **not** spawn tasks on the
//! handler's behalf.
//!
//! Phase 11 defines two operations:
//!
//! 1. [`PlatformHandler::set_topmost`] — called whenever [`crate::app::CaptureApp::pinned`]
//!    changes, so the window's Z-order bit can be kept in sync with the UI.
//! 2. [`PlatformHandler::restore_foreground`] — called right before the
//!    capture window closes, so the app that was frontmost before the panel
//!    was summoned regains keyboard focus.
//!
//! # Thread-safety
//!
//! The iced runtime clones the handler across tasks, so the trait bound is
//! `Send + Sync`. Sub-task 2 supplies a reference test mock; production impls
//! land in `trace-app` + `trace-platform`.

/// Side-effect hook supplied by the host application. See the module-level
/// docs for the contract.
pub trait PlatformHandler {
    /// Asks the platform to raise or lower the capture window's topmost bit.
    /// Called from the pin-toggle handler, so implementations should treat
    /// repeated calls with the same value as idempotent.
    fn set_topmost(&self, pinned: bool);

    /// Restores keyboard focus to whichever application was frontmost before
    /// the capture panel appeared. Called during the close flow so the user's
    /// previous context wins back focus once Trace disappears.
    fn restore_foreground(&self);
}

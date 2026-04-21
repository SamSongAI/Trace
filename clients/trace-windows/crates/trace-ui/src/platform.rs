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

/// Test-only spy handler that records every call made to
/// [`PlatformHandler`]. Built as a shareable `Arc` so unit tests can
/// hand one copy to [`crate::app::CaptureApp::with_platform_handler`] and
/// keep a second copy for assertions.
///
/// The spy is intentionally minimal: a count per method plus the last value
/// passed to [`PlatformHandler::set_topmost`]. Anything richer would make
/// the test assertions noisier without adding signal — the plan calls out
/// "idempotent" and "records call count" as the contract we exercise.
#[cfg(test)]
pub(crate) mod mock {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::PlatformHandler;

    /// Default value returned by [`MockPlatformHandler::last_set_topmost`]
    /// before any `set_topmost` call. Mirrors the default production value of
    /// [`crate::app::CaptureApp::pinned`] so the spy baseline matches UI
    /// state.
    const DEFAULT_TOPMOST: bool = false;

    /// Call-recording implementation of [`PlatformHandler`] for unit tests.
    ///
    /// The atomics let the tests share the handler with `trace-ui`'s `Arc`
    /// without a mutex — `CaptureApp` only ever calls the trait from the
    /// iced UI thread, and the spy fields are cheap atomic reads on the
    /// assertion side.
    #[derive(Debug, Default)]
    pub struct MockPlatformHandler {
        set_topmost_calls: AtomicUsize,
        last_set_topmost: AtomicBool,
        restore_foreground_calls: AtomicUsize,
    }

    impl MockPlatformHandler {
        /// Creates a fresh handler with zeroed counters, wrapped in `Arc`
        /// because iced requires `Send + Sync` at the trait-object boundary.
        pub fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        /// Number of times [`PlatformHandler::set_topmost`] has been called
        /// since construction.
        pub fn set_topmost_call_count(&self) -> usize {
            self.set_topmost_calls.load(Ordering::SeqCst)
        }

        /// Value passed to the most recent
        /// [`PlatformHandler::set_topmost`] call, or [`DEFAULT_TOPMOST`] if
        /// the method has never been called.
        pub fn last_set_topmost(&self) -> bool {
            self.last_set_topmost.load(Ordering::SeqCst)
        }

        /// Number of times [`PlatformHandler::restore_foreground`] has been
        /// called since construction.
        pub fn restore_foreground_call_count(&self) -> usize {
            self.restore_foreground_calls.load(Ordering::SeqCst)
        }
    }

    impl PlatformHandler for MockPlatformHandler {
        fn set_topmost(&self, pinned: bool) {
            self.last_set_topmost.store(pinned, Ordering::SeqCst);
            self.set_topmost_calls.fetch_add(1, Ordering::SeqCst);
        }

        fn restore_foreground(&self) {
            self.restore_foreground_calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn defaults_are_zero_and_last_topmost_is_default() {
            let handler = MockPlatformHandler::new();
            assert_eq!(handler.set_topmost_call_count(), 0);
            assert_eq!(handler.restore_foreground_call_count(), 0);
            assert_eq!(handler.last_set_topmost(), DEFAULT_TOPMOST);
        }

        #[test]
        fn set_topmost_is_recorded_and_last_value_wins() {
            let handler = MockPlatformHandler::new();
            handler.set_topmost(true);
            assert_eq!(handler.set_topmost_call_count(), 1);
            assert!(handler.last_set_topmost());
            handler.set_topmost(false);
            assert_eq!(handler.set_topmost_call_count(), 2);
            assert!(!handler.last_set_topmost());
        }

        #[test]
        fn set_topmost_is_idempotent_at_the_contract_level() {
            // "Idempotent" here means: calling with the same value twice does
            // not change the observable "last value" and only bumps the
            // counter — the underlying OS bit may skip the second write.
            let handler = MockPlatformHandler::new();
            handler.set_topmost(true);
            handler.set_topmost(true);
            assert_eq!(handler.set_topmost_call_count(), 2);
            assert!(handler.last_set_topmost());
        }

        #[test]
        fn restore_foreground_counts_up() {
            let handler = MockPlatformHandler::new();
            handler.restore_foreground();
            handler.restore_foreground();
            assert_eq!(handler.restore_foreground_call_count(), 2);
        }

        #[test]
        fn mock_can_be_shared_via_arc_dyn_platform_handler() {
            let handler = MockPlatformHandler::new();
            let dyn_ref: Arc<dyn PlatformHandler + Send + Sync> = handler.clone();
            dyn_ref.set_topmost(true);
            dyn_ref.restore_foreground();
            // Assert via the concrete spy — both references point to the
            // same underlying counters.
            assert_eq!(handler.set_topmost_call_count(), 1);
            assert_eq!(handler.restore_foreground_call_count(), 1);
            assert!(handler.last_set_topmost());
        }
    }
}

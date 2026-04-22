//! Abstraction that lets the Settings window toggle the host OS's launch-at-login
//! state without pinning `trace-ui` to the `trace-platform` autostart
//! implementation.
//!
//! The production implementation lives in `trace-app/src/main.rs` and forwards
//! to [`trace_platform::autostart::enable`] / [`trace_platform::autostart::disable`].
//! Unit tests inject a [`NoopLaunchAtLoginSink`] (the default wired through
//! [`super::SettingsApp::new`] and [`super::SettingsApp::new_with_save_path`])
//! or a custom recording mock so the UI layer can verify toggle semantics
//! without touching the Windows registry.
//!
//! The sink mirrors Mac `AppSettings.updateLaunchAtLogin`'s contract: failures
//! are logged (there, via `NSLog`; here, via `tracing::warn!`) and never
//! surfaced to the UI. That's why the trait method returns `()` — there is no
//! failure path the Settings window reacts to.

/// Applies the desired launch-at-login state to the host OS.
///
/// Kept behind a trait so `trace-ui` does not take a direct dependency on the
/// Windows-only `trace_platform::autostart` module, and tests can inject a
/// mock that records calls without touching the real registry. Production
/// wiring in `trace-app` forwards to `trace_platform::autostart::enable` /
/// `::disable`.
///
/// Implementations **must** swallow errors internally (logging via
/// [`tracing::warn!`]) and return — the UI does not surface autostart
/// failures, mirroring Mac `AppSettings.updateLaunchAtLogin` which only
/// `NSLog`s on error.
pub trait LaunchAtLoginSink: Send + Sync {
    /// Called once per UI-triggered toggle. `enabled` is the value the
    /// Settings window's shadow field just committed to; the implementation
    /// should make the host registry / login-item state match it.
    fn apply(&self, enabled: bool);
}

/// Default sink: does nothing. Used by the legacy
/// [`super::SettingsApp::new`] / [`super::SettingsApp::new_with_save_path`]
/// constructors so tests and non-Windows dev builds keep UI parity without
/// touching the host's autostart mechanism. Production `trace-app` replaces
/// this with a real sink via
/// [`super::SettingsApp::new_with_dependencies`].
pub struct NoopLaunchAtLoginSink;

impl LaunchAtLoginSink for NoopLaunchAtLoginSink {
    fn apply(&self, _enabled: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Recording sink used by the `SettingsApp`-level tests to assert the
    /// sequence of `apply` calls produced by a series of toggle messages.
    /// Kept here (rather than in `mod.rs`'s `tests` block) so a future
    /// autostart-specific test file can reuse it if needed.
    #[derive(Default)]
    pub(crate) struct RecordingSink {
        pub calls: Mutex<Vec<bool>>,
    }

    impl LaunchAtLoginSink for RecordingSink {
        fn apply(&self, enabled: bool) {
            self.calls.lock().expect("recording sink mutex").push(enabled);
        }
    }

    #[test]
    fn noop_sink_does_not_panic_on_either_value() {
        // The default sink must be safe to call with either polarity; the
        // legacy constructors wire this exact sink and the old tests assume
        // `LaunchAtLoginToggled` stays infallible.
        let sink = NoopLaunchAtLoginSink;
        sink.apply(true);
        sink.apply(false);
    }

    #[test]
    fn recording_sink_tracks_every_call_in_order() {
        // Sanity check on the test helper itself — the `SettingsApp` tests
        // rely on the recorded `Vec<bool>` matching the dispatch order.
        let sink = Arc::new(RecordingSink::default());
        (&*sink as &dyn LaunchAtLoginSink).apply(true);
        (&*sink as &dyn LaunchAtLoginSink).apply(false);
        (&*sink as &dyn LaunchAtLoginSink).apply(true);
        assert_eq!(
            *sink.calls.lock().expect("recording sink mutex"),
            vec![true, false, true],
        );
    }

    #[test]
    fn sink_trait_object_is_send_and_sync() {
        // Compile-time proof: `Arc<dyn LaunchAtLoginSink>` must be
        // `Send + Sync` so `SettingsApp` (which holds one) can satisfy the
        // same auto-traits iced expects from daemon state. If this fn
        // compiles the invariant holds.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn LaunchAtLoginSink>>();
    }
}

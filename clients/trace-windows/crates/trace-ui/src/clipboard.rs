//! Clipboard probe abstraction for paste routing.
//!
//! The capture panel's paste handler needs to ask the platform two questions
//! in order: "is there an image on the clipboard?", then, if not, "is there
//! text on the clipboard?". Doing those reads directly inside
//! [`crate::app::update`] would drag `arboard` and the Win32 clipboard APIs
//! into `trace-ui`, making the iced update logic impossible to unit-test on
//! the macOS dev host.
//!
//! Instead the production implementation lives in `trace-app`, where it
//! delegates to `trace_platform::clipboard_image` for images and `arboard`
//! for text. Tests inject [`mock::MockClipboardProbe`] so the iced `update`
//! stays OS-independent and exercisable on every workspace host.
//!
//! ## Contract
//!
//! * [`ClipboardProbe::read_image_as_png`] returns `Ok(None)` when the
//!   clipboard does not hold image data and `Ok(Some(bytes))` when a PNG-
//!   encoded frame is available. `Err(_)` is reserved for OS-level failures
//!   (clipboard unavailable, encoding blew up, etc.) — callers should
//!   surface them via a toast rather than swallow.
//! * [`ClipboardProbe::read_text`] mirrors the same tri-state shape for
//!   plain text.
//!
//! Both methods take `&self` so the trait object can live behind an `Arc`
//! without interior mutability leaking into the UI layer.

use std::fmt;

use thiserror::Error;

/// Read-only clipboard façade consumed by the capture panel's paste handler.
///
/// Returns `Ok(None)` when the clipboard does not carry the requested content
/// type. Any OS-level failure is folded into `Err(ClipboardProbeError)` so the
/// `update` handler can surface it to the user without panicking.
pub trait ClipboardProbe: fmt::Debug + Send + Sync {
    /// Reads the current clipboard and returns a PNG-encoded frame if an
    /// image is present.
    fn read_image_as_png(&self) -> Result<Option<Vec<u8>>, ClipboardProbeError>;

    /// Reads the current clipboard as plain text.
    fn read_text(&self) -> Result<Option<String>, ClipboardProbeError>;
}

/// Errors returned by [`ClipboardProbe`] implementations.
///
/// Kept intentionally narrow — the UI layer only needs to distinguish
/// "read failed" from "no content" (the latter is `Ok(None)`). Production
/// impls fold `arboard`/`trace_platform::clipboard_image` error messages
/// into the single `ReadFailed` variant via `e.to_string()`.
#[derive(Debug, Error)]
pub enum ClipboardProbeError {
    /// The platform clipboard could not be read. Payload is the underlying
    /// diagnostic string.
    #[error("clipboard read failed: {0}")]
    ReadFailed(String),
}

#[cfg(test)]
pub(crate) mod mock {
    //! Test-only [`ClipboardProbe`] implementation that serves pre-configured
    //! values to the iced `update` handler.
    //!
    //! Designed as a builder so tests read top-down:
    //! `MockClipboardProbe::new().with_image_png(bytes).with_text("x")`.
    //! Call counts are exposed so tests can assert on the image-before-text
    //! fallback ordering without spying on the editor state.

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::{ClipboardProbe, ClipboardProbeError};

    /// In-memory [`ClipboardProbe`] for unit tests. Holds the result each
    /// accessor should return and counts how many times it was queried so
    /// the `update` handler's image-before-text fallback can be verified.
    #[derive(Debug)]
    pub(crate) struct MockClipboardProbe {
        /// Result handed back by every [`ClipboardProbe::read_image_as_png`]
        /// call. `Ok(None)` by default so a bare `MockClipboardProbe::new()`
        /// behaves like an empty clipboard.
        image: Mutex<Result<Option<Vec<u8>>, String>>,
        /// Result handed back by every [`ClipboardProbe::read_text`] call.
        text: Mutex<Result<Option<String>, String>>,
        /// Number of times `read_image_as_png` has been invoked.
        image_calls: AtomicUsize,
        /// Number of times `read_text` has been invoked.
        text_calls: AtomicUsize,
    }

    impl MockClipboardProbe {
        /// Creates a probe whose `read_image_as_png` and `read_text` both
        /// return `Ok(None)` — the "empty clipboard" baseline.
        pub(crate) fn new() -> Self {
            Self {
                image: Mutex::new(Ok(None)),
                text: Mutex::new(Ok(None)),
                image_calls: AtomicUsize::new(0),
                text_calls: AtomicUsize::new(0),
            }
        }

        /// Seeds the image channel with a successful PNG byte buffer.
        pub(crate) fn with_image_png(self, bytes: Vec<u8>) -> Self {
            *self.image.lock().unwrap() = Ok(Some(bytes));
            self
        }

        /// Seeds the text channel with a successful text payload.
        pub(crate) fn with_text(self, text: impl Into<String>) -> Self {
            *self.text.lock().unwrap() = Ok(Some(text.into()));
            self
        }

        /// Seeds the image channel so every read returns
        /// [`ClipboardProbeError::ReadFailed`] with `msg` as the payload.
        pub(crate) fn with_image_error(self, msg: impl Into<String>) -> Self {
            *self.image.lock().unwrap() = Err(msg.into());
            self
        }

        /// Seeds the text channel so every read returns
        /// [`ClipboardProbeError::ReadFailed`] with `msg` as the payload.
        #[allow(dead_code)]
        pub(crate) fn with_text_error(self, msg: impl Into<String>) -> Self {
            *self.text.lock().unwrap() = Err(msg.into());
            self
        }

        /// Number of times [`ClipboardProbe::read_image_as_png`] has been
        /// invoked since construction.
        pub(crate) fn image_call_count(&self) -> usize {
            self.image_calls.load(Ordering::SeqCst)
        }

        /// Number of times [`ClipboardProbe::read_text`] has been invoked
        /// since construction.
        pub(crate) fn text_call_count(&self) -> usize {
            self.text_calls.load(Ordering::SeqCst)
        }
    }

    impl ClipboardProbe for MockClipboardProbe {
        fn read_image_as_png(&self) -> Result<Option<Vec<u8>>, ClipboardProbeError> {
            self.image_calls.fetch_add(1, Ordering::SeqCst);
            match &*self.image.lock().unwrap() {
                Ok(value) => Ok(value.clone()),
                Err(msg) => Err(ClipboardProbeError::ReadFailed(msg.clone())),
            }
        }

        fn read_text(&self) -> Result<Option<String>, ClipboardProbeError> {
            self.text_calls.fetch_add(1, Ordering::SeqCst);
            match &*self.text.lock().unwrap() {
                Ok(value) => Ok(value.clone()),
                Err(msg) => Err(ClipboardProbeError::ReadFailed(msg.clone())),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn default_probe_returns_none_for_both_channels() {
            let probe = MockClipboardProbe::new();
            assert!(matches!(probe.read_image_as_png(), Ok(None)));
            assert!(matches!(probe.read_text(), Ok(None)));
        }

        #[test]
        fn with_image_png_serves_configured_bytes() {
            let probe = MockClipboardProbe::new().with_image_png(vec![0x89, b'P', b'N', b'G']);
            let bytes = probe
                .read_image_as_png()
                .expect("probe should return Ok when seeded with image bytes")
                .expect("seeded probe should return Some(bytes)");
            assert_eq!(bytes, vec![0x89, b'P', b'N', b'G']);
        }

        #[test]
        fn with_text_serves_configured_string() {
            let probe = MockClipboardProbe::new().with_text("hello");
            let text = probe
                .read_text()
                .expect("probe should return Ok when seeded with text")
                .expect("seeded probe should return Some(text)");
            assert_eq!(text, "hello");
        }

        #[test]
        fn with_image_error_surfaces_read_failed_variant() {
            let probe = MockClipboardProbe::new().with_image_error("boom");
            let err = probe
                .read_image_as_png()
                .expect_err("seeded with image error must surface Err");
            match err {
                ClipboardProbeError::ReadFailed(msg) => assert_eq!(msg, "boom"),
            }
        }

        #[test]
        fn with_text_error_surfaces_read_failed_variant() {
            let probe = MockClipboardProbe::new().with_text_error("ugh");
            let err = probe
                .read_text()
                .expect_err("seeded with text error must surface Err");
            match err {
                ClipboardProbeError::ReadFailed(msg) => assert_eq!(msg, "ugh"),
            }
        }

        #[test]
        fn call_counters_increment_per_invocation() {
            let probe = MockClipboardProbe::new();
            assert_eq!(probe.image_call_count(), 0);
            assert_eq!(probe.text_call_count(), 0);
            let _ = probe.read_image_as_png();
            let _ = probe.read_image_as_png();
            let _ = probe.read_text();
            assert_eq!(probe.image_call_count(), 2);
            assert_eq!(probe.text_call_count(), 1);
        }
    }
}

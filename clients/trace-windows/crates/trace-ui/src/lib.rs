//! iced-based UI layer for the Trace Windows client.
//!
//! Modules:
//!
//! * [`app`] — the `CaptureApp` state, `Message` enum, and `update/view/subscription`
//!   functions driving the iced application.
//! * [`clipboard`] — `ClipboardProbe` trait consumed by the capture panel's
//!   paste handler. Keeps `trace-ui` free of `arboard` / Win32 imports; the
//!   production implementation lives in `trace-app` and delegates to
//!   `trace-platform::clipboard_image` for images and `arboard` for text.
//! * [`fonts`] — font asset bundling (Lora).
//! * [`platform`] — side-effect hooks (topmost bit, foreground restore) that
//!   `trace-app` wires up to `trace-platform`. Keeps `trace-ui` free of
//!   platform dependencies.
//! * [`settings`] — `SettingsApp` state, `SettingsMessage`, and the
//!   `update/view/theme/subscription` functions that drive the second iced
//!   window (settings). Opened by [`app::CaptureApp`] and routed through
//!   `iced::daemon` in `trace-app`.
//! * [`theme`] — pure conversions from [`trace_core::TraceTheme`] into
//!   `iced::Theme` plus per-widget style functions.
//! * [`widgets`] — pure factories for header, editor, footer, and the toast
//!   overlay.

pub mod app;
pub mod clipboard;
pub mod fonts;
pub mod platform;
pub mod settings;
pub mod theme;
pub mod widgets;

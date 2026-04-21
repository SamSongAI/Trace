//! iced-based UI layer for the Trace Windows client.
//!
//! Modules:
//!
//! * [`theme`] — pure conversions from [`trace_core::TraceTheme`] into
//!   `iced::Theme` plus per-widget style functions.
//! * [`fonts`] — font asset bundling (Lora).
//! * [`app`] — the `CaptureApp` state, `Message` enum, and `update/view/subscription`
//!   functions driving the iced application.
//! * [`widgets`] — pure factories for header, editor, footer, and the toast
//!   overlay.
//! * [`settings`] — `SettingsApp` state, `SettingsMessage`, and the
//!   `update/view/theme/subscription` functions that drive the second iced
//!   window (settings). Opened by [`app::CaptureApp`] and routed through
//!   `iced::daemon` in `trace-app`.
//! * [`platform`] — side-effect hooks (topmost bit, foreground restore) that
//!   `trace-app` wires up to `trace-platform`. Keeps `trace-ui` free of
//!   platform dependencies.

pub mod app;
pub mod fonts;
pub mod platform;
pub mod settings;
pub mod theme;
pub mod widgets;

//! iced-based UI layer for the Trace Windows client.
//!
//! Phase 10 scope:
//!
//! * [`theme`] — pure conversions from [`trace_core::TraceTheme`] into
//!   `iced::Theme` plus per-widget style functions.
//! * [`fonts`] — font asset bundling (Lora). See module docs for the Phase 10
//!   status of the font bundle.
//! * [`app`] — the `CaptureApp` state, `Message` enum, and `update/view`
//!   functions driving the iced application.
//! * [`widgets`] — pure factories for header, editor, and footer fragments.
//!
//! Interaction logic (global hotkeys, toast rendering, image paste, window
//! behavior, settings persistence) is intentionally out of scope for Phase 10
//! and will land in Phase 11.

pub mod app;
pub mod fonts;
pub mod theme;
pub mod widgets;

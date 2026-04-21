//! Windows-specific system integration for Trace.
//!
//! All real implementations live behind `#[cfg(windows)]`. On macOS/Linux the
//! crate compiles to a near-empty shell so developers can still run
//! `cargo check --workspace` locally (the `windows` crate itself is only
//! pulled in on Windows targets — see `Cargo.toml`).
//!
//! ## Modules
//!
//! - [`global_hotkey`] — Win32 `RegisterHotKey` integration that wakes the
//!   capture panel. The error type is cross-platform so higher layers can
//!   compile on any host; the [`global_hotkey::GlobalHotkey`] handle itself
//!   is only available on `#[cfg(windows)]`.
//! - [`system_tray`] — Win32 `Shell_NotifyIconW` tray icon with a localized
//!   three-item context menu (New Note / Open Settings / Quit). Mirrors the
//!   macOS `NSStatusItem` menu set up in `AppDelegate.setupStatusItem()`.
//!   As with `global_hotkey`, the error and event enums are cross-platform
//!   and the [`system_tray::SystemTray`] handle is Windows-only.
//! - [`window`] — Synchronous Win32 helpers for the capture panel: topmost
//!   / tool-window styles, foreground activation (with the
//!   `AttachThreadInput` fallback), previous-foreground capture & restore,
//!   and monitor-work-area enumeration. The pure monitor math
//!   ([`window::ScreenRect`], [`window::place_on_best_monitor`]) is
//!   cross-platform and testable on any host; the HWND-taking functions
//!   are Windows-only.
//! - [`app_paths`] — Resolves the Windows known folders
//!   `FOLDERID_RoamingAppData` and `FOLDERID_LocalAppData` to
//!   `%APPDATA%\Trace` and `%LOCALAPPDATA%\Trace` respectively, creating
//!   the sub-directory when absent. Convenience helpers
//!   [`app_paths::settings_file_path`] and [`app_paths::log_dir`] cover
//!   the two most common lookups. The error type
//!   [`app_paths::AppPathsError`] is cross-platform; the path functions
//!   themselves are Windows-only.

#![cfg_attr(not(windows), allow(dead_code))]

pub mod app_paths;
pub mod global_hotkey;
pub mod system_tray;
pub mod window;

//! Windows-specific system integration for Trace.
//!
//! All real implementations live behind `#[cfg(windows)]`. On macOS/Linux the
//! crate compiles to a near-empty shell so developers can still run
//! `cargo check --workspace` locally (the `windows` crate itself is only
//! pulled in on Windows targets — see `Cargo.toml`).
//!
//! ## Modules
//!
//! The list is alphabetised so new modules slot in against the
//! `pub mod` declaration order below without reshuffling prose.
//!
//! - [`app_paths`] — Resolves the Windows known folders
//!   `FOLDERID_RoamingAppData` and `FOLDERID_LocalAppData` to
//!   `%APPDATA%\Trace` and `%LOCALAPPDATA%\Trace` respectively, creating
//!   the sub-directory when absent. The cross-platform spec-shaped entry
//!   points [`app_paths::app_data_dir`] and
//!   [`app_paths::settings_file_path`] return `Option<PathBuf>` (they
//!   also fall back to `$HOME/.config/trace` on dev hosts); the
//!   diagnostic-preserving counterparts
//!   [`app_paths::try_roaming_app_data_dir`] /
//!   [`app_paths::try_settings_file_path`] keep the original `Result`
//!   variant for startup code that wants to log the exact failure mode.
//!   [`app_paths::log_dir`] and the Win32-only
//!   [`app_paths::try_local_app_data_dir`] remain Windows-only; the
//!   error type [`app_paths::AppPathsError`] is cross-platform.
//! - [`autostart`] — Launch-at-login integration via the
//!   `HKCU\...\CurrentVersion\Run` registry key. User-scope (no
//!   elevation), mirrors the macOS `SMAppService.launchAtLogin` toggle.
//!   The error type [`autostart::AutostartError`] is cross-platform; the
//!   [`autostart::enable`] / [`autostart::disable`] / [`autostart::is_enabled`]
//!   functions are Windows-only.
//! - [`clipboard_image`] — Cross-platform clipboard → PNG pipeline for
//!   Phase 13 image paste. Reads the platform clipboard via `arboard`,
//!   encodes the RGBA8 frame as PNG bytes via the `png` crate, and hands
//!   the result to `trace_core::ClipboardImageWriter::write_png` (the
//!   disk-write layer). The error type
//!   [`clipboard_image::ClipboardImageError`] and both public entry
//!   points ([`clipboard_image::read_clipboard_image_as_png`] /
//!   [`clipboard_image::encode_rgba_as_png`]) are cross-platform, so the
//!   paste shortcut also works on macOS dev builds.
//! - [`global_hotkey`] — Win32 `RegisterHotKey` integration that wakes the
//!   capture panel. The error type is cross-platform so higher layers can
//!   compile on any host; the [`global_hotkey::GlobalHotkey`] handle itself
//!   is only available on `#[cfg(windows)]`.
//! - [`single_instance`] — Named-mutex based single-instance enforcement.
//!   macOS gets this from the bundle identifier; on Windows we must
//!   create a `Local\` scoped mutex at startup and exit if another copy
//!   is already running. The error type and the [`single_instance::SingleInstance`]
//!   outcome are cross-platform; [`single_instance::acquire`] and the
//!   guard type are Windows-only.
//! - [`system_tray`] — Win32 `Shell_NotifyIconW` tray icon with a localized
//!   three-item context menu (New Note / Open Settings / Quit). Mirrors the
//!   macOS `NSStatusItem` menu set up in `AppDelegate.setupStatusItem()`.
//!   As with `global_hotkey`, the error and event enums are cross-platform
//!   and the [`system_tray::SystemTray`] handle is Windows-only.
//! - [`vault_validation`] — Cross-platform vault-path classification.
//!   Layers `std::fs::metadata` + a write-probe on top of the
//!   [`trace_core::VaultPathValidationIssue::is_blank`] helper so the
//!   settings UI can distinguish Empty / DoesNotExist / NotDirectory /
//!   NotWritable without reaching into the filesystem itself.
//! - [`window`] — Synchronous Win32 helpers for the capture panel: topmost
//!   / tool-window styles, foreground activation (with the
//!   `AttachThreadInput` fallback), previous-foreground capture & restore,
//!   and monitor-work-area enumeration. The pure monitor math
//!   ([`window::ScreenRect`], [`window::place_on_best_monitor`]) is
//!   cross-platform and testable on any host; the HWND-taking functions
//!   are Windows-only.

#![cfg_attr(not(windows), allow(dead_code))]

pub mod app_paths;
pub mod autostart;
pub mod clipboard_image;
pub mod global_hotkey;
pub mod single_instance;
pub mod system_tray;
pub mod vault_validation;
pub mod window;

pub use vault_validation::validate_vault_path;

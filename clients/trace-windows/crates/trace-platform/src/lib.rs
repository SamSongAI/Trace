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

#![cfg_attr(not(windows), allow(dead_code))]

pub mod global_hotkey;

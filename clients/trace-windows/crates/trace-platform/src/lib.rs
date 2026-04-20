//! Windows-specific system integration for Trace.
//!
//! All real implementations live behind `#[cfg(windows)]`. On macOS/Linux the
//! crate compiles to an empty shell so developers can still run
//! `cargo check --workspace` locally (the `windows` crate itself is only
//! pulled in on Windows targets — see `Cargo.toml`).

#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
pub mod windows_impl {
    //! Placeholder module for Phase 6+ Windows integration.
}

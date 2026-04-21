//! Pure widget factories for the capture panel.
//!
//! Each submodule exposes a free function that builds an
//! `iced::Element` fragment from plain data plus the
//! [`trace_core::CapturePalette`]. Functions never mutate state and never
//! subscribe to iced events directly — wiring up [`crate::app::Message`] is
//! the caller's responsibility.

pub mod editor;
pub mod footer;
pub mod header;

//! Cross-platform domain logic for the Trace Windows client.
//!
//! This crate intentionally avoids any Windows-specific or UI dependencies so
//! its tests can run on Linux, macOS and Windows alike. Platform integration
//! lives in `trace-platform`; the iced UI layer lives in `trace-ui`.

pub mod error;
pub mod models;
pub mod paths;
pub mod settings;
pub mod theme;
pub mod writer;

pub use error::TraceError;
pub use models::{
    Entry, EntryTheme, Language, NoteSection, PanelFrame, SeparatorStyle, ThemePreset,
    ThreadConfig, WriteMode,
};
pub use paths::{
    format_date, resolve_within_vault, sanitize_filename, sanitize_filename_preserve_extension,
    translate_swift_pattern, Locale, MAC_DATE_FORMAT_PRESETS,
};
pub use settings::{
    AppSettings, VaultPathValidationIssue, CURRENT_SECTION_TITLE_ORDER_VERSION,
    DEFAULT_APPEND_NOTE_MODIFIERS, DEFAULT_APPEND_NOTE_VKEY, DEFAULT_GLOBAL_HOTKEY_MODIFIERS,
    DEFAULT_GLOBAL_HOTKEY_VKEY, DEFAULT_MODE_TOGGLE_MODIFIERS, DEFAULT_MODE_TOGGLE_VKEY,
    DEFAULT_SEND_NOTE_MODIFIERS, DEFAULT_SEND_NOTE_VKEY, PROJECT_SECTION_INDEX,
};
pub use theme::{CapturePalette, SettingsPalette, TraceColor, TraceTheme};
pub use writer::{
    write_atomic, ClipboardImageWriter, ClipboardImageWriterSettings, DailyNoteSettings,
    DailyNoteWriter, FileWriter, FileWriterSettings, ImageWritePlan, NoteWriter, SaveMode,
    ThreadSettings, ThreadWriter, WrittenNote,
};

/// Crate version, wired up here so downstream crates can display it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_manifest() {
        assert!(
            !VERSION.is_empty(),
            "CARGO_PKG_VERSION should be populated at compile time"
        );
    }
}

//! Cross-platform `AppSettings` — the user-facing configuration struct plus
//! JSON persistence and writer-trait impls.
//!
//! Mirrors `Sources/Trace/Services/AppSettings.swift` from the Mac reference.
//! The on-disk JSON uses the same dotted keys (`trace.vaultPath`,
//! `trace.panel.originX`, …) so the file is recognizable to anyone familiar
//! with the Mac `UserDefaults` layout, and so future tooling can read both
//! platforms' settings without a translation layer.
//!
//! # Scope
//!
//! - Data-oriented: no filesystem validation, no launch-at-login hook, no
//!   legacy `UserDefaults` migration. Those concerns live in `trace-platform`
//!   (Phase 9+) or are Mac-only.
//! - Pure normalization: section-title cleanup, title-order and project-title
//!   migrations, and the panel-shortcut-collision reset are implemented as
//!   deterministic transformations.
//! - Writer trait impls: `AppSettings` satisfies all four writer settings
//!   traits defined in `crate::writer`, so a single instance can be passed to
//!   any writer without glue code.
//!
//! # Forward compatibility
//!
//! Every field is `#[serde(default)]` so future fields added by newer builds
//! do not break older parsers, and partial or hand-edited JSON fills in
//! sensible defaults.

use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::TraceError;
use crate::models::{
    EntryTheme, Language, NoteSection, PanelFrame, SeparatorStyle, ThemePreset, ThreadConfig,
    WriteMode,
};
use crate::writer::{
    write_atomic, ClipboardImageWriterSettings, DailyNoteSettings, FileWriterSettings,
    ThreadSettings,
};

/// Classification of reasons a vault path cannot be used.
///
/// This enum carries only pure data — the actual filesystem probing
/// (existence / is-directory / is-writable) lives in `trace-platform`, which
/// chains [`Self::is_blank`] with its own filesystem checks. Keeping this in
/// `trace-core` lets the UI layer consume the variants without pulling a
/// platform dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VaultPathValidationIssue {
    /// Path is empty or whitespace-only.
    Empty,
    /// Path does not exist on disk.
    DoesNotExist,
    /// Path exists but is a regular file, not a directory.
    NotDirectory,
    /// Path is a directory but the current user cannot write to it.
    NotWritable,
}

impl VaultPathValidationIssue {
    /// Returns [`Some(Self::Empty)`] when `path` trims to empty, else
    /// [`None`]. Platform-layer callers chain the remaining checks.
    pub fn is_blank(path: &str) -> Option<Self> {
        if path.trim().is_empty() {
            Some(Self::Empty)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Hotkey default constants
//
// These are placeholders for Phase 6 to validate against actual Windows
// `RegisterHotKey` constants. `trace-core` stays platform-independent — the
// real `windows-sys` imports live in `trace-platform`. Documented with the
// Carbon equivalents so the Mac→Windows mapping is obvious to anyone chasing
// a behavioural diff.
// ---------------------------------------------------------------------------

/// Windows Virtual-Key code for the `N` key. Carbon equivalent on Mac is
/// `kVK_ANSI_N`.
pub const DEFAULT_GLOBAL_HOTKEY_VKEY: u32 = 0x4E;
/// Windows `MOD_*` bitmask. Mac uses Carbon `cmdKey`. The Mac `⌘` modifier
/// maps to Windows `Ctrl` for users coming from macOS.
pub const DEFAULT_GLOBAL_HOTKEY_MODIFIERS: u32 = 0x0002; // MOD_CONTROL
/// Windows `VK_RETURN` — mirrors Mac `kVK_Return`.
pub const DEFAULT_SEND_NOTE_VKEY: u32 = 0x0D;
/// Windows `MOD_CONTROL` — mirrors Mac `cmdKey`.
pub const DEFAULT_SEND_NOTE_MODIFIERS: u32 = 0x0002;
/// Windows `VK_RETURN` — mirrors Mac `kVK_Return`.
pub const DEFAULT_APPEND_NOTE_VKEY: u32 = 0x0D;
/// Windows `MOD_CONTROL | MOD_SHIFT` — mirrors Mac `cmdKey | shiftKey`.
pub const DEFAULT_APPEND_NOTE_MODIFIERS: u32 = 0x0002 | 0x0004;
/// Windows `VK_TAB` — mirrors Mac `kVK_Tab`.
pub const DEFAULT_MODE_TOGGLE_VKEY: u32 = 0x09;
/// Windows `MOD_SHIFT` — mirrors Mac `shiftKey`.
pub const DEFAULT_MODE_TOGGLE_MODIFIERS: u32 = 0x0004;

// ---------------------------------------------------------------------------
// Section title constants
// ---------------------------------------------------------------------------

/// Current version of the section-title ordering scheme. Values below this
/// trigger the Phase-2 migration (see [`AppSettings::normalize`]).
pub const CURRENT_SECTION_TITLE_ORDER_VERSION: u32 = 2;

/// Index of the legacy "project" section in post-migration order. Used by the
/// project-title migration to replace a stored `"TODO"` with the default
/// title for that index.
pub const PROJECT_SECTION_INDEX: usize = 4;

// ---------------------------------------------------------------------------
// Free helpers used by `#[serde(default = "...")]`
// ---------------------------------------------------------------------------

fn default_daily_folder_name() -> String {
    "Daily".to_string()
}

fn default_daily_file_date_format() -> String {
    "yyyy M月d日 EEEE".to_string()
}

fn default_inbox_folder_name() -> String {
    "inbox".to_string()
}

fn default_section_titles() -> Vec<String> {
    NoteSection::DEFAULT_TITLES
        .iter()
        .map(|&title| title.to_string())
        .collect()
}

/// Default thread configs. **Note**: each call generates fresh UUIDs — tests
/// should assert on `len` and `name` rather than on specific IDs.
fn default_thread_configs() -> Vec<ThreadConfig> {
    vec![
        ThreadConfig::new("想法", "想法.md", None, 0),
        ThreadConfig::new("读书笔记", "读书笔记.md", None, 1),
        ThreadConfig::new("产品设计", "产品设计.md", None, 2),
        ThreadConfig::new("技术研究", "技术研究.md", None, 3),
    ]
}

fn default_global_hotkey_vkey() -> u32 {
    DEFAULT_GLOBAL_HOTKEY_VKEY
}

fn default_global_hotkey_modifiers() -> u32 {
    DEFAULT_GLOBAL_HOTKEY_MODIFIERS
}

fn default_send_note_vkey() -> u32 {
    DEFAULT_SEND_NOTE_VKEY
}

fn default_send_note_modifiers() -> u32 {
    DEFAULT_SEND_NOTE_MODIFIERS
}

fn default_append_note_vkey() -> u32 {
    DEFAULT_APPEND_NOTE_VKEY
}

fn default_append_note_modifiers() -> u32 {
    DEFAULT_APPEND_NOTE_MODIFIERS
}

fn default_mode_toggle_vkey() -> u32 {
    DEFAULT_MODE_TOGGLE_VKEY
}

fn default_mode_toggle_modifiers() -> u32 {
    DEFAULT_MODE_TOGGLE_MODIFIERS
}

// ---------------------------------------------------------------------------
// AppSettings
// ---------------------------------------------------------------------------

/// Full user-configurable settings for the Trace Windows client.
///
/// The on-disk JSON uses **dotted Swift keys verbatim** (`trace.vaultPath`,
/// `trace.panel.originX`, …). Every field defaults in isolation via
/// `#[serde(default)]`, so missing or partial JSON falls back cleanly. This
/// matches the Mac reference `Sources/Trace/Services/AppSettings.swift`,
/// which reads individual keys from `UserDefaults` with per-key fallbacks.
///
/// Mutation is done directly on the fields; higher-level helpers for adding
/// and removing threads or sections live on the UI layer — this struct is
/// deliberately data-oriented to simplify testing and serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(rename = "trace.language", default)]
    pub language: Language,

    #[serde(rename = "trace.vaultPath", default)]
    pub vault_path: String,

    #[serde(rename = "trace.inboxVaultPath", default)]
    pub inbox_vault_path: String,

    #[serde(
        rename = "trace.dailyFolderName",
        default = "default_daily_folder_name"
    )]
    pub daily_folder_name: String,

    #[serde(
        rename = "trace.dailyFileDateFormat",
        default = "default_daily_file_date_format"
    )]
    pub daily_file_date_format: String,

    #[serde(rename = "trace.dailyEntryThemePreset", default)]
    pub daily_entry_theme_preset: EntryTheme,

    #[serde(rename = "trace.markdownEntrySeparatorStyle", default)]
    pub markdown_entry_separator_style: SeparatorStyle,

    #[serde(rename = "trace.sectionTitles", default = "default_section_titles")]
    pub section_titles: Vec<String>,

    #[serde(rename = "trace.sectionTitlesOrderVersion", default)]
    pub section_titles_order_version: u32,

    #[serde(rename = "trace.lastUsedSectionIndex", default)]
    pub last_used_section_index: usize,

    #[serde(rename = "trace.noteWriteMode", default)]
    pub note_write_mode: WriteMode,

    #[serde(
        rename = "trace.inboxFolderName",
        default = "default_inbox_folder_name"
    )]
    pub inbox_folder_name: String,

    #[serde(rename = "trace.threadConfigs", default = "default_thread_configs")]
    pub thread_configs: Vec<ThreadConfig>,

    #[serde(
        rename = "trace.lastUsedThreadId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_used_thread_id: Option<String>,

    #[serde(rename = "trace.appThemePreset", default)]
    pub app_theme_preset: ThemePreset,

    #[serde(rename = "trace.hotKeyCode", default = "default_global_hotkey_vkey")]
    pub hot_key_code: u32,
    #[serde(
        rename = "trace.hotKeyModifiers",
        default = "default_global_hotkey_modifiers"
    )]
    pub hot_key_modifiers: u32,

    #[serde(rename = "trace.sendNoteKeyCode", default = "default_send_note_vkey")]
    pub send_note_key_code: u32,
    #[serde(
        rename = "trace.sendNoteModifiers",
        default = "default_send_note_modifiers"
    )]
    pub send_note_modifiers: u32,

    #[serde(
        rename = "trace.appendNoteKeyCode",
        default = "default_append_note_vkey"
    )]
    pub append_note_key_code: u32,
    #[serde(
        rename = "trace.appendNoteModifiers",
        default = "default_append_note_modifiers"
    )]
    pub append_note_modifiers: u32,

    #[serde(
        rename = "trace.modeToggleKeyCode",
        default = "default_mode_toggle_vkey"
    )]
    pub mode_toggle_key_code: u32,
    #[serde(
        rename = "trace.modeToggleModifiers",
        default = "default_mode_toggle_modifiers"
    )]
    pub mode_toggle_modifiers: u32,

    #[serde(rename = "trace.launchAtLogin", default)]
    pub launch_at_login: bool,

    #[serde(
        rename = "trace.panel.originX",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub panel_origin_x: Option<f64>,
    #[serde(
        rename = "trace.panel.originY",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub panel_origin_y: Option<f64>,
    #[serde(
        rename = "trace.panel.width",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub panel_width: Option<f64>,
    #[serde(
        rename = "trace.panel.height",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub panel_height: Option<f64>,
}

impl Default for AppSettings {
    fn default() -> Self {
        // Delegate to serde so every `#[serde(default)]` and
        // `#[serde(default = "...")]` attribute contributes to the baseline.
        // This guarantees the `Default` impl never drifts from the parse path
        // that new JSON files take. `"{}"` always parses because every field
        // has a default.
        serde_json::from_str("{}").expect("AppSettings default must deserialize from `{}`")
    }
}

impl AppSettings {
    /// Applies all normalization rules that do not require filesystem access:
    ///
    /// 1. Section-title order migration (v0 → v2): indices 3 and 4 are
    ///    swapped when `section_titles_order_version < 2` and the array has
    ///    at least 5 entries.
    /// 2. Project-title migration: when the pre-migration project slot (index
    ///    [`PROJECT_SECTION_INDEX`]) held `"TODO"` (case/whitespace
    ///    insensitive), replace it with the default title for that index.
    /// 3. Section-title value normalization: truncate to
    ///    [`NoteSection::MAXIMUM_COUNT`] (9), replace CR/LF with spaces,
    ///    strip a leading `#+\s*` heading marker, trim whitespace, and fall
    ///    back to the default title for the slot if the result is empty.
    /// 4. Bump `section_titles_order_version` to the current value.
    /// 5. Panel-shortcut collision: if the send-note and append-note
    ///    shortcuts are identical (same key code *and* modifiers), reset the
    ///    append-note shortcut to its default.
    ///
    /// Mirrors the constructor-time logic in Swift `AppSettings.init(...)`
    /// plus `normalizePanelShortcutCollisionsIfNeeded`.
    pub fn normalize(&mut self) {
        let migrated_order =
            migrate_section_titles_order(&self.section_titles, self.section_titles_order_version);
        let migrated_project =
            migrate_project_title(&migrated_order, self.section_titles_order_version);
        self.section_titles = normalize_section_titles(&migrated_project);

        if self.section_titles_order_version < CURRENT_SECTION_TITLE_ORDER_VERSION {
            self.section_titles_order_version = CURRENT_SECTION_TITLE_ORDER_VERSION;
        }

        if self.send_note_key_code == self.append_note_key_code
            && self.send_note_modifiers == self.append_note_modifiers
        {
            self.append_note_key_code = DEFAULT_APPEND_NOTE_VKEY;
            self.append_note_modifiers = DEFAULT_APPEND_NOTE_MODIFIERS;
        }
    }

    /// Returns a [`NoteSection`] view over [`Self::section_titles`].
    ///
    /// Mirrors Swift's `var sections: [NoteSection]`. Index and title come
    /// from the stored titles as-is — the empty-fallback rule is only
    /// applied by [`Self::title_for`].
    pub fn sections(&self) -> Vec<NoteSection> {
        self.section_titles
            .iter()
            .enumerate()
            .map(|(index, title)| NoteSection::new(index, title))
            .collect()
    }

    /// Effective title for the section at `index`, applying the
    /// blank-fallback rule.
    ///
    /// Mirrors Swift `title(for:)`: the stored title is used verbatim unless
    /// it trims to empty, in which case [`NoteSection::default_title_for`]
    /// supplies a `"Section N"` fallback. Returns the per-index default when
    /// `index` is out of bounds so UI code cannot accidentally panic.
    pub fn title_for(&self, index: usize) -> String {
        match self.section_titles.get(index) {
            Some(stored) => {
                let trimmed = stored.trim();
                if trimmed.is_empty() {
                    NoteSection::default_title_for(index)
                } else {
                    stored.clone()
                }
            }
            None => NoteSection::default_title_for(index),
        }
    }

    /// Markdown `# {title}` header for the section at `index`.
    pub fn header_for_index(&self, index: usize) -> String {
        format!("# {}", self.title_for(index))
    }

    /// Whether another section slot is available (bounded by
    /// [`NoteSection::MAXIMUM_COUNT`] = 9).
    pub fn can_add_section(&self) -> bool {
        self.section_titles.len() < NoteSection::MAXIMUM_COUNT
    }

    /// Whether at least one section could be removed while still respecting
    /// [`NoteSection::MINIMUM_COUNT`] = 1.
    pub fn can_remove_section(&self) -> bool {
        self.section_titles.len() > NoteSection::MINIMUM_COUNT
    }

    /// Whether another thread slot is available (same 9-slot cap as
    /// sections).
    pub fn can_add_thread(&self) -> bool {
        self.thread_configs.len() < NoteSection::MAXIMUM_COUNT
    }

    /// Whether at least one thread could be removed while still leaving one
    /// thread configured.
    pub fn can_remove_thread(&self) -> bool {
        self.thread_configs.len() > NoteSection::MINIMUM_COUNT
    }

    /// Preferred thread to preselect in the UI.
    ///
    /// Mirrors Swift's `defaultThread`:
    /// - if `last_used_thread_id` parses as a UUID that matches a current
    ///   config, return that config;
    /// - otherwise, return the first configured thread;
    /// - returns `None` only when `thread_configs` is empty.
    pub fn default_thread(&self) -> Option<&ThreadConfig> {
        if let Some(stored) = self.last_used_thread_id.as_ref() {
            if let Ok(id) = Uuid::parse_str(stored) {
                if let Some(config) = self.thread_configs.iter().find(|c| c.id == id) {
                    return Some(config);
                }
            }
        }
        self.thread_configs.first()
    }

    /// Merges the four panel-frame fields into a single [`PanelFrame`].
    ///
    /// Returns `None` when *any* of the four coordinates is missing — panel
    /// state is all-or-nothing on Mac, and the Swift equivalent
    /// (`savedPanelFrame`) bails as soon as one key is `nil`.
    pub fn panel_frame(&self) -> Option<PanelFrame> {
        match (
            self.panel_origin_x,
            self.panel_origin_y,
            self.panel_width,
            self.panel_height,
        ) {
            (Some(x), Some(y), Some(width), Some(height)) => {
                Some(PanelFrame::new(x, y, width, height))
            }
            _ => None,
        }
    }

    /// Stores `frame` as the four panel-frame fields.
    pub fn set_panel_frame(&mut self, frame: PanelFrame) {
        self.panel_origin_x = Some(frame.x);
        self.panel_origin_y = Some(frame.y);
        self.panel_width = Some(frame.width);
        self.panel_height = Some(frame.height);
    }

    /// Resolves the section the user asked for, clamped to the current
    /// section count.
    ///
    /// Mirrors Swift's `resolvedSection(for:)`:
    /// - `requested_index == None` → treated as `Some(0)` (matches Swift's
    ///   `section?.index ?? 0`);
    /// - out-of-bounds indices clamp to the last available slot;
    /// - when `section_titles` is empty, returns a synthesized section at
    ///   index 0 with the default title.
    pub fn resolved_section(&self, requested_index: Option<usize>) -> NoteSection {
        let sections = self.sections();
        if sections.is_empty() {
            return NoteSection::new(0, NoteSection::default_title_for(0));
        }

        let requested = requested_index.unwrap_or(0);
        let clamped = requested.min(sections.len() - 1);
        sections[clamped].clone()
    }

    /// Loads settings from `path`. Returns [`AppSettings::default`] if the
    /// file does not exist (first-launch behaviour) and propagates I/O or
    /// parse errors otherwise.
    ///
    /// Callers typically invoke [`Self::normalize`] after loading — this
    /// method deliberately does **not** normalize implicitly so tests and
    /// migration tools can observe the raw on-disk shape.
    pub fn load(path: &Path) -> Result<Self, TraceError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let settings = serde_json::from_str(&contents)?;
        Ok(settings)
    }

    /// Serializes to pretty-printed JSON and writes atomically via
    /// [`write_atomic`].
    ///
    /// The parent directory is **not** auto-created — callers should ensure
    /// the settings directory exists (platform layer handles this during
    /// first-launch bootstrap).
    pub fn save(&self, path: &Path) -> Result<(), TraceError> {
        let json = serde_json::to_string_pretty(self)?;
        write_atomic(path, json.as_bytes())
    }
}

// ---------------------------------------------------------------------------
// Private normalization helpers
//
// These are free functions rather than methods so `normalize` can call them
// without reborrowing `self`. They mirror the static helpers on Swift's
// `AppSettings`.
// ---------------------------------------------------------------------------

/// Truncates `titles` to at most [`NoteSection::MAXIMUM_COUNT`] entries,
/// falls back to the default titles when the input is empty, and normalizes
/// each remaining entry via [`normalize_section_title`].
fn normalize_section_titles(titles: &[String]) -> Vec<String> {
    let trimmed: Vec<String> = titles
        .iter()
        .take(NoteSection::MAXIMUM_COUNT)
        .cloned()
        .collect();
    let source: Vec<String> = if trimmed.is_empty() {
        default_section_titles()
    } else {
        trimmed
    };

    source
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let fallback = NoteSection::default_title_for(index);
            normalize_section_title(&raw, &fallback)
        })
        .collect()
}

/// Normalizes a single title: replace CR/LF with spaces, strip a leading
/// `#+\s*` heading marker, trim whitespace, and fall back to `fallback` if
/// the result is empty.
///
/// The heading-prefix scan is hand-rolled so `trace-core` avoids a regex
/// dependency. Swift uses `replacingOccurrences(of: #"^#+\s*"#, with:
/// "", options: .regularExpression)`.
fn normalize_section_title(raw: &str, fallback: &str) -> String {
    // Step 1: replace CR/LF with a single space each. Matches Swift's two
    // `replacingOccurrences` calls (`\r` and `\n` → `" "`).
    let single_line: String = raw
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect();

    // Step 2: strip a leading `#+\s*` prefix.
    let mut cursor = 0usize;
    let bytes = single_line.as_bytes();
    while cursor < bytes.len() && bytes[cursor] == b'#' {
        cursor += 1;
    }
    let head_was_hash = cursor > 0;
    if head_was_hash {
        // Consume any number of whitespace chars after the `#`s.
        while cursor < single_line.len() {
            let rest = &single_line[cursor..];
            match rest.chars().next() {
                Some(c) if c.is_whitespace() => {
                    cursor += c.len_utf8();
                }
                _ => break,
            }
        }
    }
    let without_heading = if head_was_hash {
        &single_line[cursor..]
    } else {
        single_line.as_str()
    };

    // Step 3: trim whitespace. Fall back to `fallback` if the result is
    // empty.
    let trimmed = without_heading.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Swaps indices 3 and 4 when `stored_version` is below
/// [`CURRENT_SECTION_TITLE_ORDER_VERSION`] and the array is long enough.
///
/// Mirrors Swift's `migrateLegacySectionTitleOrder`.
fn migrate_section_titles_order(titles: &[String], stored_version: u32) -> Vec<String> {
    if stored_version >= CURRENT_SECTION_TITLE_ORDER_VERSION || titles.len() < 5 {
        return titles.to_vec();
    }
    let mut migrated = titles.to_vec();
    migrated.swap(3, 4);
    migrated
}

/// Replaces the project-slot title with the per-index default when it
/// decodes (case-insensitively, whitespace-stripped) to `"TODO"`.
///
/// Mirrors Swift's `migrateLegacyProjectTitle`. The migration fires only
/// when `stored_version` is below [`CURRENT_SECTION_TITLE_ORDER_VERSION`]
/// and the array has a slot at [`PROJECT_SECTION_INDEX`].
fn migrate_project_title(titles: &[String], stored_version: u32) -> Vec<String> {
    if stored_version >= CURRENT_SECTION_TITLE_ORDER_VERSION {
        return titles.to_vec();
    }
    let Some(title) = titles.get(PROJECT_SECTION_INDEX) else {
        return titles.to_vec();
    };

    let compacted: String = title
        .trim()
        .chars()
        .filter(|c| *c != ' ')
        .collect::<String>()
        .to_uppercase();

    if compacted != "TODO" {
        return titles.to_vec();
    }

    let mut migrated = titles.to_vec();
    migrated[PROJECT_SECTION_INDEX] = NoteSection::default_title_for(PROJECT_SECTION_INDEX);
    migrated
}

// ---------------------------------------------------------------------------
// Writer-settings trait impls
//
// `AppSettings` satisfies every writer settings trait so a single instance
// can drive `DailyNoteWriter`, `ThreadWriter`, `FileWriter`, and
// `ClipboardImageWriter` without adapter code.
// ---------------------------------------------------------------------------

impl DailyNoteSettings for AppSettings {
    fn vault_path(&self) -> &Path {
        Path::new(&self.vault_path)
    }

    fn daily_folder_name(&self) -> &str {
        // Mirror Swift's `normalizedFolderName`: blank → `"Daily"`. The raw
        // field stays user-editable; only the writer path falls back.
        if self.daily_folder_name.trim().is_empty() {
            "Daily"
        } else {
            &self.daily_folder_name
        }
    }

    fn daily_file_date_format(&self) -> &str {
        &self.daily_file_date_format
    }

    fn entry_theme(&self) -> EntryTheme {
        self.daily_entry_theme_preset
    }

    fn header_for(&self, section: &NoteSection) -> String {
        self.header_for_index(section.index)
    }
}

impl ThreadSettings for AppSettings {
    fn vault_path(&self) -> &Path {
        Path::new(&self.vault_path)
    }

    fn entry_theme(&self) -> EntryTheme {
        self.daily_entry_theme_preset
    }
}

impl FileWriterSettings for AppSettings {
    fn inbox_vault_path(&self) -> &Path {
        Path::new(&self.inbox_vault_path)
    }
}

impl ClipboardImageWriterSettings for AppSettings {
    fn vault_path(&self) -> &Path {
        Path::new(&self.vault_path)
    }

    fn daily_folder_name(&self) -> &str {
        DailyNoteSettings::daily_folder_name(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
// `AppSettings::default()` + per-field mutation is the idiomatic test pattern
// here: it keeps each case focused on the fields that matter and compiles
// faster than struct-update syntax with 30+ fields. The clippy lint is
// safe-to-silence in test code where the `Default` impl has no side effects.
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::writer::{
        ClipboardImageWriter, DailyNoteWriter, FileWriter, NoteWriter, ThreadWriter,
    };
    use chrono::{TimeZone, Utc};
    use tempfile::TempDir;

    fn fixed_now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 20, 12, 30, 0).unwrap()
    }

    // -------------------------------------------------------------------
    // 1. Defaults pinned byte-for-byte
    // -------------------------------------------------------------------

    #[test]
    fn default_matches_mac_baseline() {
        let settings = AppSettings::default();
        assert_eq!(settings.language, Language::SystemDefault);
        assert_eq!(settings.vault_path, "");
        assert_eq!(settings.inbox_vault_path, "");
        assert_eq!(settings.daily_folder_name, "Daily");
        assert_eq!(settings.daily_file_date_format, "yyyy M月d日 EEEE");
        assert_eq!(
            settings.daily_entry_theme_preset,
            EntryTheme::CodeBlockClassic
        );
        assert_eq!(
            settings.markdown_entry_separator_style,
            SeparatorStyle::HorizontalRule
        );
        assert_eq!(
            settings.section_titles,
            vec!["Note", "Memo", "Link", "Task"]
        );
        assert_eq!(settings.section_titles_order_version, 0);
        assert_eq!(settings.last_used_section_index, 0);
        assert_eq!(settings.note_write_mode, WriteMode::Dimension);
        assert_eq!(settings.inbox_folder_name, "inbox");
        assert_eq!(settings.thread_configs.len(), 4);
        let thread_names: Vec<&str> = settings
            .thread_configs
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(
            thread_names,
            vec!["想法", "读书笔记", "产品设计", "技术研究"]
        );
        let thread_orders: Vec<i32> = settings.thread_configs.iter().map(|t| t.order).collect();
        assert_eq!(thread_orders, vec![0, 1, 2, 3]);
        assert!(settings.thread_configs.iter().all(|t| t.icon.is_none()));
        assert_eq!(settings.last_used_thread_id, None);
        assert_eq!(settings.app_theme_preset, ThemePreset::Dark);
        assert_eq!(settings.hot_key_code, DEFAULT_GLOBAL_HOTKEY_VKEY);
        assert_eq!(settings.hot_key_modifiers, DEFAULT_GLOBAL_HOTKEY_MODIFIERS);
        assert_eq!(settings.send_note_key_code, DEFAULT_SEND_NOTE_VKEY);
        assert_eq!(settings.send_note_modifiers, DEFAULT_SEND_NOTE_MODIFIERS);
        assert_eq!(settings.append_note_key_code, DEFAULT_APPEND_NOTE_VKEY);
        assert_eq!(
            settings.append_note_modifiers,
            DEFAULT_APPEND_NOTE_MODIFIERS
        );
        assert_eq!(settings.mode_toggle_key_code, DEFAULT_MODE_TOGGLE_VKEY);
        assert_eq!(
            settings.mode_toggle_modifiers,
            DEFAULT_MODE_TOGGLE_MODIFIERS
        );
        assert!(!settings.launch_at_login);
        assert_eq!(settings.panel_origin_x, None);
        assert_eq!(settings.panel_origin_y, None);
        assert_eq!(settings.panel_width, None);
        assert_eq!(settings.panel_height, None);
    }

    // -------------------------------------------------------------------
    // 2. JSON round-trips
    // -------------------------------------------------------------------

    #[test]
    fn json_round_trip_of_defaults() {
        let original = AppSettings::default();
        let json = serde_json::to_string(&original).unwrap();
        let decoded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.language, original.language);
        assert_eq!(decoded.vault_path, original.vault_path);
        assert_eq!(decoded.daily_folder_name, original.daily_folder_name);
        assert_eq!(decoded.section_titles, original.section_titles);
        assert_eq!(decoded.thread_configs.len(), original.thread_configs.len());
    }

    #[test]
    fn json_round_trip_of_custom_values() {
        let mut settings = AppSettings::default();
        settings.vault_path = "C:/vault".into();
        settings.inbox_vault_path = "C:/inbox".into();
        settings.daily_folder_name = "DayNotes".into();
        settings.section_titles = vec!["One".into(), "Two".into(), "Three".into()];
        settings.section_titles_order_version = 2;
        settings.last_used_section_index = 2;
        settings.app_theme_preset = ThemePreset::Paper;
        settings.launch_at_login = true;
        settings.hot_key_code = 0x50;
        settings.last_used_thread_id = Some(Uuid::new_v4().to_string());
        settings.set_panel_frame(PanelFrame::new(10.0, 20.0, 400.0, 300.0));

        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.vault_path, "C:/vault");
        assert_eq!(decoded.inbox_vault_path, "C:/inbox");
        assert_eq!(decoded.daily_folder_name, "DayNotes");
        assert_eq!(decoded.section_titles, settings.section_titles);
        assert_eq!(decoded.section_titles_order_version, 2);
        assert_eq!(decoded.last_used_section_index, 2);
        assert_eq!(decoded.app_theme_preset, ThemePreset::Paper);
        assert!(decoded.launch_at_login);
        assert_eq!(decoded.hot_key_code, 0x50);
        assert_eq!(decoded.last_used_thread_id, settings.last_used_thread_id);
        assert_eq!(decoded.panel_frame(), settings.panel_frame());
    }

    // -------------------------------------------------------------------
    // 3. Partial JSON backfills defaults
    // -------------------------------------------------------------------

    #[test]
    fn partial_json_fills_in_defaults() {
        let decoded: AppSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(decoded.daily_folder_name, "Daily");
        assert_eq!(decoded.daily_file_date_format, "yyyy M月d日 EEEE");
        assert_eq!(decoded.inbox_folder_name, "inbox");
        assert_eq!(decoded.section_titles, vec!["Note", "Memo", "Link", "Task"]);
        assert_eq!(decoded.thread_configs.len(), 4);
        assert_eq!(decoded.hot_key_code, DEFAULT_GLOBAL_HOTKEY_VKEY);
    }

    #[test]
    fn partial_json_respects_overrides() {
        let decoded: AppSettings =
            serde_json::from_str(r#"{"trace.vaultPath":"D:/foo","trace.launchAtLogin":true}"#)
                .unwrap();
        assert_eq!(decoded.vault_path, "D:/foo");
        assert!(decoded.launch_at_login);
        assert_eq!(decoded.daily_folder_name, "Daily"); // untouched
    }

    // -------------------------------------------------------------------
    // 4. Dotted Swift-style keys on disk
    // -------------------------------------------------------------------

    #[test]
    fn serializes_with_dotted_swift_keys() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(1.0, 2.0, 3.0, 4.0));
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"trace.vaultPath\""));
        assert!(json.contains("\"trace.panel.originX\""));
        assert!(json.contains("\"trace.panel.originY\""));
        assert!(json.contains("\"trace.panel.width\""));
        assert!(json.contains("\"trace.panel.height\""));
        assert!(json.contains("\"trace.hotKeyCode\""));
        assert!(json.contains("\"trace.sectionTitles\""));
    }

    #[test]
    fn omits_panel_fields_when_none() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(!json.contains("\"trace.panel.originX\""));
        assert!(!json.contains("\"trace.panel.originY\""));
        assert!(!json.contains("\"trace.panel.width\""));
        assert!(!json.contains("\"trace.panel.height\""));
    }

    #[test]
    fn omits_last_used_thread_id_when_none() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(!json.contains("\"trace.lastUsedThreadId\""));
    }

    // -------------------------------------------------------------------
    // 5. Section title normalization cases
    // -------------------------------------------------------------------

    #[test]
    fn normalize_truncates_to_nine_titles() {
        let input: Vec<String> = (0..12).map(|i| format!("T{i}")).collect();
        let result = normalize_section_titles(&input);
        assert_eq!(result.len(), 9);
    }

    #[test]
    fn normalize_empty_input_yields_defaults() {
        let result = normalize_section_titles(&[]);
        assert_eq!(result, vec!["Note", "Memo", "Link", "Task"]);
    }

    #[test]
    fn normalize_strips_leading_hash() {
        let result = normalize_section_title("## Header", "fallback");
        assert_eq!(result, "Header");
    }

    #[test]
    fn normalize_strips_multiple_hashes_and_whitespace() {
        let result = normalize_section_title("###   Title", "fallback");
        assert_eq!(result, "Title");
    }

    #[test]
    fn normalize_replaces_cr_and_lf_with_space() {
        let result = normalize_section_title("line\r\nbreak", "fallback");
        assert_eq!(result, "line  break");
        let result2 = normalize_section_title("line\nbreak", "fallback");
        assert_eq!(result2, "line break");
        let result3 = normalize_section_title("line\rbreak", "fallback");
        assert_eq!(result3, "line break");
    }

    #[test]
    fn normalize_trims_whitespace() {
        let result = normalize_section_title("   padded   ", "fallback");
        assert_eq!(result, "padded");
    }

    #[test]
    fn normalize_blank_after_stripping_falls_back() {
        let result = normalize_section_title("   ", "FB");
        assert_eq!(result, "FB");
        let result2 = normalize_section_title("#   ", "FB");
        assert_eq!(result2, "FB");
    }

    #[test]
    fn normalize_does_not_strip_interior_hashes() {
        let result = normalize_section_title("Tag #id", "fallback");
        assert_eq!(result, "Tag #id");
    }

    // -------------------------------------------------------------------
    // 6. Section title order migration
    // -------------------------------------------------------------------

    #[test]
    fn order_migration_v0_with_five_titles_swaps_3_and_4() {
        let titles: Vec<String> = ["A", "B", "C", "D", "E"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let migrated = migrate_section_titles_order(&titles, 0);
        assert_eq!(migrated, vec!["A", "B", "C", "E", "D"]);
    }

    #[test]
    fn order_migration_v0_with_four_titles_is_no_op() {
        let titles: Vec<String> = ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect();
        let migrated = migrate_section_titles_order(&titles, 0);
        assert_eq!(migrated, titles);
    }

    #[test]
    fn order_migration_v2_leaves_five_titles_untouched() {
        let titles: Vec<String> = ["A", "B", "C", "D", "E"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let migrated = migrate_section_titles_order(&titles, 2);
        assert_eq!(migrated, titles);
    }

    // -------------------------------------------------------------------
    // 7. Project title migration
    // -------------------------------------------------------------------

    fn titles_with_project(raw: &str) -> Vec<String> {
        vec!["A".into(), "B".into(), "C".into(), "D".into(), raw.into()]
    }

    #[test]
    fn project_migration_lowercase_todo_is_replaced() {
        let migrated = migrate_project_title(&titles_with_project("todo"), 0);
        assert_eq!(
            migrated[PROJECT_SECTION_INDEX],
            NoteSection::default_title_for(PROJECT_SECTION_INDEX)
        );
    }

    #[test]
    fn project_migration_whitespace_capitalized_todo_is_replaced() {
        let migrated = migrate_project_title(&titles_with_project("Todo "), 0);
        assert_eq!(
            migrated[PROJECT_SECTION_INDEX],
            NoteSection::default_title_for(PROJECT_SECTION_INDEX)
        );
    }

    #[test]
    fn project_migration_two_word_to_do_is_replaced() {
        let migrated = migrate_project_title(&titles_with_project("To Do"), 0);
        assert_eq!(
            migrated[PROJECT_SECTION_INDEX],
            NoteSection::default_title_for(PROJECT_SECTION_INDEX)
        );
    }

    #[test]
    fn project_migration_project_title_is_untouched() {
        let migrated = migrate_project_title(&titles_with_project("Project"), 0);
        assert_eq!(migrated[PROJECT_SECTION_INDEX], "Project");
    }

    #[test]
    fn project_migration_v2_does_not_fire_even_for_todo() {
        let migrated = migrate_project_title(&titles_with_project("TODO"), 2);
        assert_eq!(migrated[PROJECT_SECTION_INDEX], "TODO");
    }

    #[test]
    fn project_migration_skips_short_arrays() {
        let titles: Vec<String> = ["A", "B"].iter().map(|s| s.to_string()).collect();
        let migrated = migrate_project_title(&titles, 0);
        assert_eq!(migrated, titles);
    }

    #[test]
    fn project_migration_only_strips_ascii_space_not_tab_or_unicode_whitespace() {
        // Swift's `replacingOccurrences(of: " ", with: "")` only handles
        // U+0020. Tabs, NBSP, and full-width spaces must NOT be treated as
        // collapsible — otherwise "TO\tDO" and similar variants would trigger
        // migration on Windows but not on Mac, diverging the two clients.
        for exotic in ["TO\tDO", "TO\u{00A0}DO", "TO\u{3000}DO"] {
            let titles = titles_with_project(exotic);
            let migrated = migrate_project_title(&titles, 0);
            assert_eq!(
                migrated[PROJECT_SECTION_INDEX], exotic,
                "exotic whitespace variant {exotic:?} must be preserved, not migrated to default",
            );
        }
    }

    // -------------------------------------------------------------------
    // 8. Panel shortcut collision
    // -------------------------------------------------------------------

    #[test]
    fn panel_shortcut_collision_resets_append_to_default() {
        let mut settings = AppSettings::default();
        settings.send_note_key_code = 0x42;
        settings.send_note_modifiers = 0x0001;
        settings.append_note_key_code = 0x42;
        settings.append_note_modifiers = 0x0001;
        settings.normalize();
        assert_eq!(settings.append_note_key_code, DEFAULT_APPEND_NOTE_VKEY);
        assert_eq!(
            settings.append_note_modifiers,
            DEFAULT_APPEND_NOTE_MODIFIERS
        );
        // Send-note fields stay user-configured.
        assert_eq!(settings.send_note_key_code, 0x42);
        assert_eq!(settings.send_note_modifiers, 0x0001);
    }

    #[test]
    fn panel_shortcut_collision_leaves_distinct_shortcuts_alone() {
        let mut settings = AppSettings::default();
        settings.send_note_key_code = 0x42;
        settings.send_note_modifiers = 0x0001;
        settings.append_note_key_code = 0x42;
        settings.append_note_modifiers = 0x0003;
        settings.normalize();
        assert_eq!(settings.append_note_key_code, 0x42);
        assert_eq!(settings.append_note_modifiers, 0x0003);
    }

    // -------------------------------------------------------------------
    // 9. Panel frame merge
    // -------------------------------------------------------------------

    #[test]
    fn panel_frame_returns_none_when_all_fields_missing() {
        let settings = AppSettings::default();
        assert_eq!(settings.panel_frame(), None);
    }

    #[test]
    fn panel_frame_returns_some_when_all_fields_present() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(
            settings.panel_frame(),
            Some(PanelFrame::new(1.0, 2.0, 3.0, 4.0))
        );
    }

    #[test]
    fn panel_frame_missing_origin_x_yields_none() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(1.0, 2.0, 3.0, 4.0));
        settings.panel_origin_x = None;
        assert_eq!(settings.panel_frame(), None);
    }

    #[test]
    fn panel_frame_missing_origin_y_yields_none() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(1.0, 2.0, 3.0, 4.0));
        settings.panel_origin_y = None;
        assert_eq!(settings.panel_frame(), None);
    }

    #[test]
    fn panel_frame_missing_width_yields_none() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(1.0, 2.0, 3.0, 4.0));
        settings.panel_width = None;
        assert_eq!(settings.panel_frame(), None);
    }

    #[test]
    fn panel_frame_missing_height_yields_none() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(1.0, 2.0, 3.0, 4.0));
        settings.panel_height = None;
        assert_eq!(settings.panel_frame(), None);
    }

    #[test]
    fn set_panel_frame_populates_all_four_fields() {
        let mut settings = AppSettings::default();
        settings.set_panel_frame(PanelFrame::new(11.0, 22.0, 33.0, 44.0));
        assert_eq!(settings.panel_origin_x, Some(11.0));
        assert_eq!(settings.panel_origin_y, Some(22.0));
        assert_eq!(settings.panel_width, Some(33.0));
        assert_eq!(settings.panel_height, Some(44.0));
    }

    // -------------------------------------------------------------------
    // 10. Default thread resolution
    // -------------------------------------------------------------------

    #[test]
    fn default_thread_without_last_used_returns_first() {
        let settings = AppSettings::default();
        let thread = settings.default_thread().unwrap();
        assert_eq!(thread.name, "想法");
    }

    #[test]
    fn default_thread_with_matching_last_used_returns_match() {
        let mut settings = AppSettings::default();
        let target_id = settings.thread_configs[2].id;
        settings.last_used_thread_id = Some(target_id.to_string());
        let thread = settings.default_thread().unwrap();
        assert_eq!(thread.id, target_id);
        assert_eq!(thread.name, "产品设计");
    }

    #[test]
    fn default_thread_with_unknown_last_used_falls_back_to_first() {
        let mut settings = AppSettings::default();
        settings.last_used_thread_id = Some(Uuid::new_v4().to_string());
        let thread = settings.default_thread().unwrap();
        assert_eq!(thread.name, "想法");
    }

    #[test]
    fn default_thread_empty_configs_returns_none() {
        let mut settings = AppSettings::default();
        settings.thread_configs = vec![];
        assert!(settings.default_thread().is_none());
    }

    #[test]
    fn default_thread_with_malformed_uuid_falls_back_to_first() {
        let mut settings = AppSettings::default();
        settings.last_used_thread_id = Some("not-a-uuid".into());
        let thread = settings.default_thread().unwrap();
        assert_eq!(thread.name, "想法");
    }

    // -------------------------------------------------------------------
    // 11. Sections view
    // -------------------------------------------------------------------

    #[test]
    fn sections_view_has_sequential_indices() {
        let settings = AppSettings::default();
        let sections = settings.sections();
        assert_eq!(sections.len(), settings.section_titles.len());
        for (i, section) in sections.iter().enumerate() {
            assert_eq!(section.index, i);
        }
    }

    // -------------------------------------------------------------------
    // 12. title_for blank fallback
    // -------------------------------------------------------------------

    #[test]
    fn title_for_blank_stored_falls_back_to_default() {
        let mut settings = AppSettings::default();
        settings.section_titles = vec!["   ".into(), "Memo".into()];
        assert_eq!(settings.title_for(0), NoteSection::default_title_for(0));
        assert_eq!(settings.title_for(1), "Memo");
    }

    #[test]
    fn title_for_out_of_bounds_returns_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.title_for(99), NoteSection::default_title_for(99));
    }

    #[test]
    fn header_for_index_prefixes_hash() {
        let settings = AppSettings::default();
        assert_eq!(settings.header_for_index(0), "# Note");
    }

    // -------------------------------------------------------------------
    // 13. Load/save round-trip
    // -------------------------------------------------------------------

    #[test]
    fn save_then_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        let mut original = AppSettings::default();
        original.vault_path = "C:/vault".into();
        original.section_titles = vec!["Alpha".into(), "Beta".into()];
        original.set_panel_frame(PanelFrame::new(5.0, 6.0, 7.0, 8.0));
        original.save(&path).unwrap();

        let loaded = AppSettings::load(&path).unwrap();
        assert_eq!(loaded.vault_path, "C:/vault");
        assert_eq!(loaded.section_titles, vec!["Alpha", "Beta"]);
        assert_eq!(
            loaded.panel_frame(),
            Some(PanelFrame::new(5.0, 6.0, 7.0, 8.0))
        );
    }

    #[test]
    fn load_nonexistent_path_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing.json");
        let loaded = AppSettings::load(&path).unwrap();
        assert_eq!(loaded.daily_folder_name, "Daily");
        assert_eq!(loaded.section_titles, vec!["Note", "Memo", "Link", "Task"]);
    }

    #[test]
    fn load_malformed_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not json at all").unwrap();
        let err = AppSettings::load(&path).unwrap_err();
        assert!(matches!(err, TraceError::SerializationFailed(_)));
    }

    #[test]
    fn save_writes_pretty_printed_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        AppSettings::default().save(&path).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        // Pretty-printed JSON has newlines and indentation.
        assert!(raw.contains('\n'));
        assert!(raw.contains("  "));
    }

    // -------------------------------------------------------------------
    // 14. Writer trait integration smoke tests
    // -------------------------------------------------------------------

    #[test]
    fn daily_note_writer_accepts_app_settings() {
        let dir = TempDir::new().unwrap();
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().to_string_lossy().into_owned();
        settings.daily_file_date_format = "yyyy-MM-dd".into();
        let writer = DailyNoteWriter::new(settings);
        let section = NoteSection::new(0, "Note");
        let written = writer
            .save_new_entry("hello", &section, fixed_now())
            .unwrap()
            .expect("non-empty text produces a write");
        assert!(written.path.starts_with(dir.path()));
        assert!(written.path.extension().and_then(|e| e.to_str()) == Some("md"));
        assert!(written.bytes_written > 0);
    }

    #[test]
    fn thread_writer_accepts_app_settings() {
        let dir = TempDir::new().unwrap();
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().to_string_lossy().into_owned();
        let thread = settings.thread_configs[0].clone();
        let writer = ThreadWriter::new(settings);
        let written = writer
            .save(
                "hello",
                &thread,
                crate::writer::SaveMode::CreateNewEntry,
                fixed_now(),
            )
            .unwrap()
            .expect("non-empty text produces a write");
        assert!(written.path.starts_with(dir.path()));
    }

    #[test]
    fn file_writer_accepts_app_settings() {
        let dir = TempDir::new().unwrap();
        let mut settings = AppSettings::default();
        settings.inbox_vault_path = dir.path().to_string_lossy().into_owned();
        let writer = FileWriter::new(settings);
        let written = writer
            .save("body text", Some("my title"), None, fixed_now())
            .unwrap()
            .expect("non-empty text produces a write");
        assert!(written.path.starts_with(dir.path()));
    }

    #[test]
    fn clipboard_image_writer_plan_accepts_app_settings() {
        let dir = TempDir::new().unwrap();
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().to_string_lossy().into_owned();
        let writer = ClipboardImageWriter::new(settings);
        let plan = writer.plan(fixed_now()).unwrap();
        assert!(plan.target_path.starts_with(dir.path()));
        assert!(plan.relative_path.starts_with("assets/"));
        assert!(plan.markdown_link.starts_with("![image](assets/"));
    }

    // Ensure `NoteWriter` trait is in scope so impls are covered via trait
    // objects in future — this single assertion keeps the import live.
    #[test]
    fn note_writer_trait_is_usable() {
        fn _assert_writer<W: NoteWriter>(_w: &W) {}
        // Compile-check only; no runtime effect.
    }

    // -------------------------------------------------------------------
    // 15. Blank daily folder fallback at writer boundary
    // -------------------------------------------------------------------

    #[test]
    fn blank_daily_folder_falls_back_to_daily_at_writer_boundary() {
        let mut settings = AppSettings::default();
        settings.daily_folder_name = "   ".into();
        // Raw field preserves user input; trait impl substitutes the
        // fallback so writer paths never produce `vault//...` URLs.
        assert_eq!(settings.daily_folder_name, "   ");
        assert_eq!(DailyNoteSettings::daily_folder_name(&settings), "Daily");
        assert_eq!(
            ClipboardImageWriterSettings::daily_folder_name(&settings),
            "Daily"
        );
    }

    #[test]
    fn non_blank_daily_folder_is_used_verbatim() {
        let mut settings = AppSettings::default();
        settings.daily_folder_name = "MyDaily".into();
        assert_eq!(DailyNoteSettings::daily_folder_name(&settings), "MyDaily");
    }

    // -------------------------------------------------------------------
    // Additional: normalize end-to-end
    // -------------------------------------------------------------------

    #[test]
    fn normalize_runs_migration_and_bumps_version() {
        let mut settings = AppSettings::default();
        settings.section_titles = vec![
            "A".into(),
            "B".into(),
            "C".into(),
            "D".into(),
            "TODO".into(),
        ];
        settings.section_titles_order_version = 0;
        settings.normalize();

        // Order migration swaps 3 and 4, then project migration only fires
        // if post-swap slot 4 still reads "TODO"; after the swap it reads
        // "D", so the project-title migration is a no-op here — matching
        // Swift where both migrations run on the pre-normalization array
        // but the project check is done *after* the order swap too (see
        // `migratedStoredSectionTitles`).
        assert_eq!(settings.section_titles_order_version, 2);
        assert_eq!(settings.section_titles.len(), 5);
    }

    #[test]
    fn normalize_preserves_custom_sections_when_version_current() {
        let mut settings = AppSettings::default();
        settings.section_titles = vec!["Custom1".into(), "Custom2".into()];
        settings.section_titles_order_version = CURRENT_SECTION_TITLE_ORDER_VERSION;
        settings.normalize();
        assert_eq!(settings.section_titles, vec!["Custom1", "Custom2"]);
    }

    #[test]
    fn normalize_applies_heading_strip_via_mutation() {
        let mut settings = AppSettings::default();
        settings.section_titles = vec!["## Header".into(), "  Padded  ".into()];
        settings.normalize();
        assert_eq!(settings.section_titles[0], "Header");
        assert_eq!(settings.section_titles[1], "Padded");
    }

    // -------------------------------------------------------------------
    // can_add / can_remove helpers
    // -------------------------------------------------------------------

    #[test]
    fn can_add_section_is_true_when_under_cap() {
        let settings = AppSettings::default();
        assert!(settings.can_add_section());
    }

    #[test]
    fn can_add_section_is_false_at_cap() {
        let mut settings = AppSettings::default();
        settings.section_titles = (0..9).map(|i| format!("S{i}")).collect();
        assert!(!settings.can_add_section());
    }

    #[test]
    fn can_remove_section_is_true_above_floor() {
        let settings = AppSettings::default();
        assert!(settings.can_remove_section());
    }

    #[test]
    fn can_remove_section_is_false_at_floor() {
        let mut settings = AppSettings::default();
        settings.section_titles = vec!["Only".into()];
        assert!(!settings.can_remove_section());
    }

    #[test]
    fn can_add_thread_respects_max_count() {
        let settings = AppSettings::default();
        assert!(settings.can_add_thread());
        let mut full = AppSettings::default();
        full.thread_configs = (0..9)
            .map(|i| ThreadConfig::new(format!("T{i}"), format!("T{i}.md"), None, i))
            .collect();
        assert!(!full.can_add_thread());
    }

    #[test]
    fn can_remove_thread_respects_min_count() {
        let settings = AppSettings::default();
        assert!(settings.can_remove_thread());
        let mut one = AppSettings::default();
        one.thread_configs = vec![ThreadConfig::new("Only", "Only.md", None, 0)];
        assert!(!one.can_remove_thread());
    }

    // -------------------------------------------------------------------
    // resolved_section edge cases
    // -------------------------------------------------------------------

    #[test]
    fn resolved_section_none_returns_first() {
        let settings = AppSettings::default();
        let resolved = settings.resolved_section(None);
        assert_eq!(resolved.index, 0);
    }

    #[test]
    fn resolved_section_clamps_to_last_slot() {
        let settings = AppSettings::default();
        let resolved = settings.resolved_section(Some(99));
        assert_eq!(resolved.index, settings.section_titles.len() - 1);
    }

    #[test]
    fn resolved_section_empty_list_yields_index_zero_default() {
        let mut settings = AppSettings::default();
        settings.section_titles = vec![];
        let resolved = settings.resolved_section(Some(3));
        assert_eq!(resolved.index, 0);
        assert_eq!(resolved.title, NoteSection::default_title_for(0));
    }

    // -------------------------------------------------------------------
    // VaultPathValidationIssue helper
    // -------------------------------------------------------------------

    #[test]
    fn is_blank_detects_empty_and_whitespace_only() {
        assert_eq!(
            VaultPathValidationIssue::is_blank(""),
            Some(VaultPathValidationIssue::Empty)
        );
        assert_eq!(
            VaultPathValidationIssue::is_blank("   "),
            Some(VaultPathValidationIssue::Empty)
        );
        assert_eq!(
            VaultPathValidationIssue::is_blank("\t\n"),
            Some(VaultPathValidationIssue::Empty)
        );
    }

    #[test]
    fn is_blank_returns_none_for_non_blank() {
        assert_eq!(VaultPathValidationIssue::is_blank("/tmp/x"), None);
        assert_eq!(VaultPathValidationIssue::is_blank("C:/vault"), None);
    }

    // -------------------------------------------------------------------
    // Arc<AppSettings> forwarding
    //
    // The blanket impls in `writer/{daily,thread,file,clipboard_image}.rs`
    // let hosts share a single settings snapshot through an `Arc`
    // without cloning. Pin the contract here so a regression in either
    // the blanket impl or the per-field forwarding surfaces loudly.
    // -------------------------------------------------------------------
    #[test]
    fn arc_app_settings_forwards_daily_thread_file_and_image_traits() {
        use std::sync::Arc;

        let mut settings = AppSettings::default();
        settings.vault_path = "/tmp/trace-vault".to_string();
        settings.inbox_vault_path = "/tmp/trace-inbox".to_string();
        settings.daily_folder_name = "Daily".to_string();
        settings.daily_file_date_format = "yyyy-MM-dd".to_string();
        let shared = Arc::new(settings);

        // DailyNoteSettings: key scalar + header_for default path.
        assert_eq!(
            DailyNoteSettings::vault_path(&shared),
            Path::new("/tmp/trace-vault")
        );
        assert_eq!(DailyNoteSettings::daily_folder_name(&shared), "Daily");
        assert_eq!(
            DailyNoteSettings::daily_file_date_format(&shared),
            "yyyy-MM-dd"
        );
        let section = NoteSection::new(0, "Note");
        // header_for on AppSettings uses `header_for_index` — we only need
        // to confirm the forwarding returns a non-empty string.
        assert!(!DailyNoteSettings::header_for(&shared, &section).is_empty());

        // ThreadSettings: vault_path.
        assert_eq!(
            ThreadSettings::vault_path(&shared),
            Path::new("/tmp/trace-vault")
        );

        // FileWriterSettings: inbox_vault_path.
        assert_eq!(
            FileWriterSettings::inbox_vault_path(&shared),
            Path::new("/tmp/trace-inbox")
        );

        // ClipboardImageWriterSettings.
        assert_eq!(
            ClipboardImageWriterSettings::vault_path(&shared),
            Path::new("/tmp/trace-vault")
        );
        assert_eq!(
            ClipboardImageWriterSettings::daily_folder_name(&shared),
            "Daily"
        );
    }

    #[test]
    fn arc_app_settings_drives_a_daily_note_write() {
        use std::sync::Arc;

        // Round-trip: pass `Arc<AppSettings>` straight into the writer
        // constructor, issue a save, and confirm the file lands.
        let tmp = TempDir::new().unwrap();
        let mut settings = AppSettings::default();
        settings.vault_path = tmp.path().to_string_lossy().into_owned();
        let shared = Arc::new(settings);

        let writer = DailyNoteWriter::new(Arc::clone(&shared));
        let section = NoteSection::new(0, NoteSection::DEFAULT_TITLES[0]);
        let written = writer
            .save_new_entry("arc forwarded body", &section, fixed_now())
            .expect("save succeeds")
            .expect("writer returns a Written record");
        assert!(written.bytes_written > 0);
    }
}

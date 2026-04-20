//! Daily / dimension-mode note writer.
//!
//! Mirrors `DailyNoteWriter.saveToDailyNote`, `DailyNoteWriter.insert`, and
//! the three theme-specific `appendLatest*Entry` helpers from the Mac source
//! at `Sources/Trace/Services/DailyNoteWriter.swift`. Thread mode, file mode,
//! and clipboard image handling live in sibling modules.
//!
//! # Save modes
//!
//! - [`SaveMode::CreateNewEntry`]: insert a fresh entry at the top of the
//!   section (Phase 2.2).
//! - [`SaveMode::AppendToLatestEntry`]: splice new content *into* the latest
//!   entry under the section — theme-aware (code-block closing fence for
//!   `CodeBlockClassic`, timestamp line for `PlainTextTimestamp`, quote-block
//!   trailer for `MarkdownQuote`). Falls back to `CreateNewEntry` when the
//!   parser can't locate the expected anchor. (Phase 2.5)
//!
//! # Byte-level parity
//!
//! Every formatting decision is translated verbatim from Swift. The three
//! entry bodies and the section-insertion prefix rules are covered by
//! dedicated unit tests so regressions surface at the byte level. Timestamps
//! use `yyyy-MM-dd HH:mm` formatted via chrono (locale-independent). File
//! names use the user-configured Swift TR 35 pattern translated by
//! `paths::date_format` with `Locale::ZhCn` — this matches Mac's
//! `Locale(identifier: "zh_CN")` in `formattedFileName`.

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::error::TraceError;
use crate::models::{EntryTheme, NoteSection};
use crate::paths::{date_format, resolve_within_vault};
use crate::writer::{
    markdown_quote_body_thread, timestamp, validated_vault_path, write_atomic, SaveMode,
    WrittenNote,
};

/// Injectable configuration for the daily note writer.
///
/// The Mac source defines this as `DailyNoteSettingsProviding`. We only
/// surface the fields actually consumed by the `createNewEntry` path.
/// Thread-specific and inbox-specific settings stay on their respective
/// writers.
pub trait DailyNoteSettings {
    /// Absolute filesystem path to the note vault. Whitespace is trimmed by
    /// the writer before use; blank paths raise `InvalidVaultPath`.
    fn vault_path(&self) -> &Path;

    /// Subfolder inside the vault that holds daily notes. Blank/whitespace
    /// values fall back to `"Daily"` (same as Mac's `normalizedFolderName`).
    fn daily_folder_name(&self) -> &str;

    /// Swift TR 35 date pattern used to name daily files, e.g.
    /// `"yyyy M月d日 EEEE"`. Translated via `paths::date_format`.
    fn daily_file_date_format(&self) -> &str;

    /// Preset controlling entry body formatting (fenced code block, plain
    /// text + timestamp, or markdown blockquote).
    fn entry_theme(&self) -> EntryTheme;

    /// Returns the section header to insert under. Mirrors Swift's
    /// `header(for:)` on `DailyNoteSettingsProviding`. The default
    /// implementation uses `NoteSection::header()` (`# {title}`), but hosts
    /// may override to inject user-customised titles without mutating the
    /// `NoteSection` itself.
    fn header_for(&self, section: &NoteSection) -> String {
        section.header()
    }
}

/// Writes daily/dimension-mode entries. Holds a clock-independent reference
/// to settings plus file-system I/O delegated to `write_atomic`.
pub struct DailyNoteWriter<S: DailyNoteSettings> {
    settings: S,
}

impl<S: DailyNoteSettings> DailyNoteWriter<S> {
    /// Constructs a writer bound to the given settings. The `now` timestamp
    /// is passed per-call via `save_new_entry` so tests can inject a fixed
    /// clock without setting up a time-source abstraction.
    pub fn new(settings: S) -> Self {
        Self { settings }
    }

    /// Saves `text` as a new entry under `section`, creating the daily file
    /// and section header as needed.
    ///
    /// Equivalent to `self.save(text, section, SaveMode::CreateNewEntry, now)`
    /// and kept as a convenience for callers that only ever insert.
    ///
    /// Returns `Ok(None)` when `text` trims to empty — mirroring Swift's
    /// silent-return behaviour — and `Ok(Some(written))` on a successful
    /// write. Returns `Err` for invalid vault paths, path-escape attempts,
    /// unsupported date patterns, or filesystem errors.
    pub fn save_new_entry(
        &self,
        text: &str,
        section: &NoteSection,
        now: DateTime<Utc>,
    ) -> Result<Option<WrittenNote>, TraceError> {
        self.save(text, section, SaveMode::CreateNewEntry, now)
    }

    /// Saves `text` into the daily note under `section`, honouring `mode`.
    ///
    /// Mirrors Swift's `DailyNoteWriter.save(...)` dispatch:
    /// - [`SaveMode::CreateNewEntry`] inserts a fresh themed entry at the top
    ///   of the section (or appends a new section when the header is
    ///   missing).
    /// - [`SaveMode::AppendToLatestEntry`] splices new content into the most
    ///   recent entry under the section. When no entry anchor exists (no
    ///   section header, no opening fence, no timestamp line, no quote
    ///   block), it falls back to the create path.
    ///
    /// Returns `Ok(None)` for empty/whitespace `text`. The file is created
    /// and parent directories are auto-provisioned when missing.
    pub fn save(
        &self,
        text: &str,
        section: &NoteSection,
        mode: SaveMode,
        now: DateTime<Utc>,
    ) -> Result<Option<WrittenNote>, TraceError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let file_path = self.daily_file_path(now)?;
        let parent = file_path.parent().ok_or_else(|| {
            TraceError::AtomicWriteFailed(format!(
                "daily note path has no parent: {}",
                file_path.display()
            ))
        })?;
        fs::create_dir_all(parent)?;

        let existing = load_or_empty(&file_path)?;
        let header = self.settings.header_for(section);
        let ts = timestamp(now);
        let entry = self.entry_for_text(trimmed, now);

        let updated = match mode {
            SaveMode::CreateNewEntry => insert_entry(&existing, &header, &entry),
            SaveMode::AppendToLatestEntry => append_latest_entry(
                &existing,
                trimmed,
                &ts,
                &header,
                self.settings.entry_theme(),
            )
            .unwrap_or_else(|| insert_entry(&existing, &header, &entry)),
        };

        let bytes = updated.as_bytes();
        write_atomic(&file_path, bytes)?;

        Ok(Some(WrittenNote {
            path: file_path,
            bytes_written: bytes.len() as u64,
        }))
    }

    /// Returns the absolute path `{vault}/{folder}/{formatted_date}.md` after
    /// checking the relative part cannot escape the vault.
    pub fn daily_file_path(&self, now: DateTime<Utc>) -> Result<PathBuf, TraceError> {
        let vault = validated_vault_path(self.settings.vault_path(), "vault path")?;

        let folder = normalized_folder_name(self.settings.daily_folder_name(), "Daily");
        let filename = format!(
            "{}.md",
            date_format::format_date(
                self.settings.daily_file_date_format(),
                &now.date_naive(),
                date_format::Locale::ZhCn,
            )?
        );

        let relative = format!("{}/{}", folder, filename);
        resolve_within_vault(&vault, &relative)
    }

    fn entry_for_text(&self, text: &str, now: DateTime<Utc>) -> String {
        let ts = timestamp(now);
        let body = match self.settings.entry_theme() {
            EntryTheme::CodeBlockClassic => code_block_body(text, &ts),
            EntryTheme::PlainTextTimestamp => plain_text_body(text, &ts),
            EntryTheme::MarkdownQuote => markdown_quote_body(text, &ts),
        };
        markdown_entry(&body)
    }
}

/// Wraps an entry body in the trailing `\n\n` separator used by the Mac
/// client. Two newlines produce a blank line between entries.
fn markdown_entry(body: &str) -> String {
    format!("{body}\n\n")
}

fn code_block_body(text: &str, timestamp: &str) -> String {
    format!("```\n{text}\n{timestamp}\n```")
}

fn plain_text_body(text: &str, timestamp: &str) -> String {
    format!("{text}\n{timestamp}")
}

fn markdown_quote_body(text: &str, timestamp: &str) -> String {
    let quoted = markdown_quote_body_thread(text);
    format!("{quoted}\n>\n> {timestamp}")
}

fn normalized_folder_name(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn load_or_empty(path: &Path) -> Result<String, TraceError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => {
            let kind = e.kind();
            let wrapped = std::io::Error::new(kind, format!("reading {}: {}", path.display(), e));
            Err(TraceError::Io(wrapped))
        }
    }
}

/// Section insertion — byte-for-byte translation of Swift
/// `DailyNoteWriter.insert(_:into:under:)` at lines 247–278.
///
/// Given the existing file `content`, a target `header` (e.g. `"# Note"`),
/// and an `entry` already wrapped with its trailing `\n\n`, returns the new
/// file contents. The prefix rules match Swift exactly so the output is
/// byte-identical to the Mac client.
pub(crate) fn insert_entry(content: &str, header: &str, entry: &str) -> String {
    if let Some(header_start) = content.find(header) {
        let after_header = header_start + header.len();

        // Find the first newline after the header end. `None` means the
        // header sits at the very end of the file with no trailing `\n`.
        let line_break_pos = content[after_header..]
            .find('\n')
            .map(|offset| after_header + offset);

        let insert_index = match line_break_pos {
            Some(lb) if lb < content.len() => lb + 1,
            _ => content.len(),
        };

        let prefix = if insert_index < content.len() {
            if content.as_bytes()[insert_index] == b'\n' {
                ""
            } else {
                "\n"
            }
        } else if line_break_pos.is_none() {
            "\n\n"
        } else {
            "\n"
        };

        let mut out = String::with_capacity(content.len() + prefix.len() + entry.len());
        out.push_str(&content[..insert_index]);
        out.push_str(prefix);
        out.push_str(entry);
        out.push_str(&content[insert_index..]);
        return out;
    }

    // Header not present.
    if content.trim().is_empty() {
        return format!("{header}\n\n{entry}");
    }

    let mut out = String::with_capacity(content.len() + header.len() + entry.len() + 4);
    out.push_str(content);
    if !content.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(header);
    out.push_str("\n\n");
    out.push_str(entry);
    out
}

/// Append-to-latest-entry dispatch — byte-for-byte translation of Swift
/// `DailyNoteWriter.appendLatestEntry(_:at:into:under:)` at lines 292–301.
///
/// Returns `None` when the section header is missing or the theme-specific
/// anchor cannot be located; callers fall back to `insert_entry` in that
/// case so the user's text never vanishes silently.
pub(crate) fn append_latest_entry(
    content: &str,
    text: &str,
    ts: &str,
    header: &str,
    theme: EntryTheme,
) -> Option<String> {
    match theme {
        EntryTheme::CodeBlockClassic => append_latest_code_block(content, text, ts, header),
        EntryTheme::PlainTextTimestamp => append_latest_plain_text(content, text, ts, header),
        EntryTheme::MarkdownQuote => append_latest_markdown_quote(content, text, ts, header),
    }
}

/// Splices `---\n{text}\n{ts}\n` immediately before the first closing ``` fence
/// under `header`. Mirrors Swift `appendLatestCodeBlockEntry` (lines 303–326).
fn append_latest_code_block(content: &str, text: &str, ts: &str, header: &str) -> Option<String> {
    let section_range = section_body_range(content, header)?;
    let section = &content[section_range.clone()];

    let opening_rel = section.find("```")?;
    let after_opening = opening_rel + 3;
    let closing_rel = section[after_opening..]
        .find("```")
        .map(|p| after_opening + p)?;

    let insertion_abs = section_range.start + closing_rel;

    let prefix = if insertion_abs > 0 && content.as_bytes()[insertion_abs - 1] == b'\n' {
        ""
    } else {
        "\n"
    };
    let chunk = format!("{prefix}---\n{text}\n{ts}\n");

    Some(splice_at(content, insertion_abs, &chunk))
}

/// Splices `\n---\n{text}\n{ts}` immediately after the first `YYYY-MM-DD HH:MM`
/// line under `header`. Mirrors Swift `appendLatestPlainTextEntry` (lines
/// 354–373).
fn append_latest_plain_text(content: &str, text: &str, ts: &str, header: &str) -> Option<String> {
    let section_range = section_body_range(content, header)?;
    let section = &content[section_range.clone()];

    let ts_line = find_first_timestamp_line(section)?;
    let insertion_abs = section_range.start + ts_line.end;

    let chunk = format!("\n---\n{text}\n{ts}");
    Some(splice_at(content, insertion_abs, &chunk))
}

/// Splices `\n> ---\n{quoted}\n>\n> {ts}\n` at the end of the first callout
/// block (`>` prefix run) under `header`. Mirrors Swift
/// `appendLatestMarkdownQuoteEntry` (lines 375–393).
fn append_latest_markdown_quote(
    content: &str,
    text: &str,
    ts: &str,
    header: &str,
) -> Option<String> {
    let section_range = section_body_range(content, header)?;
    let section = &content[section_range.clone()];

    let quote_start = first_quote_block_start(section)?;
    let quote_end = callout_block_end(section, quote_start);
    let insertion_abs = section_range.start + quote_end;

    let quoted = markdown_quote_body_thread(text);
    let chunk = format!("\n> ---\n{quoted}\n>\n> {ts}\n");

    Some(splice_at(content, insertion_abs, &chunk))
}

/// Shared splicing helper: `content[..at] + chunk + content[at..]` with a
/// pre-sized allocation. Keeps all three appenders byte-identical.
fn splice_at(content: &str, at: usize, chunk: &str) -> String {
    let mut out = String::with_capacity(content.len() + chunk.len());
    out.push_str(&content[..at]);
    out.push_str(chunk);
    out.push_str(&content[at..]);
    out
}

/// Returns the byte range of the body under `header` in `content`: from the
/// char after the header's trailing `\n` up to the next `\n# ` (or EOF).
///
/// Mirrors Swift `sectionBodyRange(in:under:)` at lines 436–454. Returns
/// `None` when `header` is not found. A header at EOF with no trailing
/// newline returns a zero-length range positioned at EOF — the caller's
/// anchor search on the empty slice yields `None` and falls back to create.
fn section_body_range(content: &str, header: &str) -> Option<Range<usize>> {
    let header_start = content.find(header)?;
    let after_header = header_start + header.len();

    let line_break_rel = content[after_header..].find('\n');
    let section_start = match line_break_rel {
        Some(offset) => after_header + offset + 1,
        None => content.len(),
    };

    if section_start >= content.len() {
        return Some(section_start..section_start);
    }

    if let Some(next_header_rel) = content[section_start..].find("\n# ") {
        Some(section_start..(section_start + next_header_rel))
    } else {
        Some(section_start..content.len())
    }
}

/// Returns the byte offset of the first line in `section` whose first
/// character is `>`. Mirrors Swift `firstQuoteBlockStart(in:)` (lines
/// 395–411).
fn first_quote_block_start(section: &str) -> Option<usize> {
    let bytes = section.as_bytes();
    let mut line_start = 0;

    while line_start < section.len() {
        let rel_break = bytes[line_start..].iter().position(|&b| b == b'\n');
        let line_end = rel_break.map(|r| line_start + r).unwrap_or(section.len());

        if section[line_start..line_end].starts_with('>') {
            return Some(line_start);
        }

        let Some(lb) = rel_break else { break };
        line_start += lb + 1;
    }

    None
}

/// Returns the byte offset where the callout block starting at `start` ends.
/// Mirrors Swift `calloutBlockEnd(in:from:)` (lines 413–434).
///
/// The block ends:
/// - at `section.len()` if every subsequent line is a `>` line and the last
///   one has no trailing `\n`;
/// - at the byte after the trailing `\n` of the last `>` line when the block
///   is followed by `\n` plus any content that is NOT a `>` line;
/// - at the start of the first subsequent `> [!…` line (nested callout),
///   *except* when that line IS `start` (the anchor line itself can be a
///   callout header).
fn callout_block_end(section: &str, start: usize) -> usize {
    let bytes = section.as_bytes();
    let mut line_start = start;

    while line_start < section.len() {
        let rel_break = bytes[line_start..].iter().position(|&b| b == b'\n');
        let Some(rel) = rel_break else {
            let line = &section[line_start..];
            return if line.starts_with('>') {
                section.len()
            } else {
                line_start
            };
        };

        let line_break = line_start + rel;
        let line = &section[line_start..line_break];

        if line_start != start && line.starts_with("> [!") {
            return line_start;
        }
        if !line.starts_with('>') {
            return line_start;
        }

        line_start = line_break + 1;
    }

    section.len()
}

/// Returns the byte range of the first line in `section` that matches
/// `YYYY-MM-DD HH:MM`. Equivalent to Swift's regex
/// `(?m)^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$` — hand-rolled so `trace-core`
/// avoids depending on the `regex` crate for this one narrow pattern.
fn find_first_timestamp_line(section: &str) -> Option<Range<usize>> {
    let bytes = section.as_bytes();
    let mut line_start = 0;

    while line_start < section.len() {
        let rel_break = bytes[line_start..].iter().position(|&b| b == b'\n');
        let line_end_with_cr = rel_break.map(|r| line_start + r).unwrap_or(section.len());
        let line_end = if line_end_with_cr > line_start && bytes[line_end_with_cr - 1] == b'\r' {
            line_end_with_cr - 1
        } else {
            line_end_with_cr
        };

        if is_timestamp_line(&section[line_start..line_end]) {
            return Some(line_start..line_end);
        }

        let Some(lb) = rel_break else { break };
        line_start += lb + 1;
    }

    None
}

/// Returns `true` when `line` is exactly 16 ASCII bytes shaped like
/// `YYYY-MM-DD HH:MM` (the format emitted by `timestamp`).
fn is_timestamp_line(line: &str) -> bool {
    if line.len() != 16 {
        return false;
    }
    let b = line.as_bytes();
    b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4] == b'-'
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && b[7] == b'-'
        && b[8].is_ascii_digit()
        && b[9].is_ascii_digit()
        && b[10] == b' '
        && b[11].is_ascii_digit()
        && b[12].is_ascii_digit()
        && b[13] == b':'
        && b[14].is_ascii_digit()
        && b[15].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    /// In-memory settings used by every test. Fields mirror the trait
    /// surface so tests can tweak individual knobs without building a full
    /// `AppSettings`-style struct.
    struct FakeDailySettings {
        vault: PathBuf,
        folder: String,
        date_format: String,
        theme: EntryTheme,
        custom_header: Option<String>,
    }

    impl FakeDailySettings {
        fn new(vault: PathBuf, theme: EntryTheme) -> Self {
            Self {
                vault,
                folder: "Daily".to_string(),
                date_format: "yyyy-MM-dd".to_string(),
                theme,
                custom_header: None,
            }
        }
    }

    impl DailyNoteSettings for FakeDailySettings {
        fn vault_path(&self) -> &Path {
            &self.vault
        }
        fn daily_folder_name(&self) -> &str {
            &self.folder
        }
        fn daily_file_date_format(&self) -> &str {
            &self.date_format
        }
        fn entry_theme(&self) -> EntryTheme {
            self.theme
        }
        fn header_for(&self, section: &NoteSection) -> String {
            self.custom_header
                .clone()
                .unwrap_or_else(|| section.header())
        }
    }

    fn fixed_time(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    // ---------------------------------------------------------------------
    // End-to-end writer tests
    // ---------------------------------------------------------------------

    #[test]
    fn creates_file_under_daily_folder_with_formatted_name() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 2, 27, 22, 35);
        let section = NoteSection::new(0, "Note");

        let written = writer
            .save_new_entry("first note", &section, now)
            .unwrap()
            .expect("should write a file");

        let expected_path = tmp.path().join("Daily").join("2026-02-27.md");
        assert_eq!(written.path, expected_path);
        assert!(expected_path.exists());

        let actual = fs::read_to_string(&expected_path).unwrap();
        let expected = "# Note\n\n```\nfirst note\n2026-02-27 22:35\n```\n\n";
        assert_eq!(actual, expected);
        assert_eq!(written.bytes_written as usize, expected.len());
    }

    #[test]
    fn inserts_under_existing_section_header() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 2, 27, 22, 35);
        let file_path = writer.daily_file_path(now).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "# Note\n\n# Clip\n").unwrap();

        let section = NoteSection::new(1, "Clip");
        let _note = writer
            .save_new_entry("clip text", &section, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&file_path).unwrap();
        let expected = "# Note\n\n# Clip\n\n```\nclip text\n2026-02-27 22:35\n```\n\n";
        assert_eq!(actual, expected);
    }

    #[test]
    fn appends_new_section_when_header_missing() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 2, 27, 22, 35);
        let file_path = writer.daily_file_path(now).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "# Note\n\nmanual text\n").unwrap();

        let section = NoteSection::new(1, "Clip");
        let _note = writer
            .save_new_entry("new clip", &section, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&file_path).unwrap();
        let expected =
            "# Note\n\nmanual text\n\n# Clip\n\n```\nnew clip\n2026-02-27 22:35\n```\n\n";
        assert_eq!(actual, expected);
    }

    #[test]
    fn plain_text_timestamp_theme_has_no_fence() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::PlainTextTimestamp);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 3, 3, 23, 25);
        let section = NoteSection::new(0, "Note");
        let written = writer
            .save_new_entry("plain body", &section, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        assert!(
            !actual.contains("```"),
            "plainTextTimestamp theme must not emit a code fence: {actual:?}"
        );
        let body_segment = "plain body\n2026-03-03 23:25";
        assert!(
            actual.contains(body_segment),
            "expected body segment `{body_segment}` in `{actual}`"
        );
    }

    #[test]
    fn markdown_quote_theme_prefixes_every_line_with_gt() {
        let tmp = TempDir::new().unwrap();
        let settings = FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::MarkdownQuote);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 3, 3, 23, 25);
        let section = NoteSection::new(0, "Note");
        let written = writer
            .save_new_entry("quote body", &section, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        assert!(
            actual.contains("> quote body\n>\n> 2026-03-03 23:25"),
            "expected quoted body + timestamp in `{actual}`"
        );
        assert!(!actual.contains("```"), "must not emit a code fence");
        assert!(!actual.contains("[!"), "must not emit a callout marker");
    }

    #[test]
    fn markdown_quote_preserves_blank_lines_as_bare_gt() {
        let tmp = TempDir::new().unwrap();
        let settings = FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::MarkdownQuote);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 3, 3, 23, 25);
        let section = NoteSection::new(0, "Note");
        let written = writer
            .save_new_entry("a\n\nb", &section, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        let expected_body = "> a\n>\n> b\n>\n> 2026-03-03 23:25";
        assert!(
            actual.contains(expected_body),
            "expected multi-line quoted body in `{actual}`"
        );
    }

    #[test]
    fn supports_custom_section_title() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(5, "Ideas");
        let written = writer
            .save_new_entry("sixth bucket", &section, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        assert!(
            actual.starts_with("# Ideas\n\n"),
            "expected header `# Ideas` at start of `{actual}`"
        );
        assert!(
            actual.contains("sixth bucket"),
            "expected text body in `{actual}`"
        );
    }

    #[test]
    fn falls_back_to_default_daily_folder_when_name_blank() {
        let tmp = TempDir::new().unwrap();
        let mut settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        settings.folder = "   ".to_string();
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(0, "Note");
        let written = writer
            .save_new_entry("text", &section, now)
            .unwrap()
            .unwrap();

        assert_eq!(written.path, tmp.path().join("Daily").join("2026-04-20.md"));
    }

    #[test]
    fn rejects_vault_escape_in_folder_name() {
        let tmp = TempDir::new().unwrap();
        let mut settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        settings.folder = "../outside".to_string();
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(0, "Note");
        let err = writer.save_new_entry("text", &section, now).unwrap_err();
        assert!(
            matches!(err, TraceError::PathEscapesVault(_)),
            "expected PathEscapesVault, got {err:?}"
        );
    }

    #[test]
    fn rejects_empty_vault_path() {
        let mut settings = FakeDailySettings::new(PathBuf::from(""), EntryTheme::CodeBlockClassic);
        // Also exercise a whitespace-only path to ensure trimming kicks in.
        settings.vault = PathBuf::from("   ");
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(0, "Note");
        let err = writer.save_new_entry("text", &section, now).unwrap_err();
        assert!(
            matches!(err, TraceError::InvalidVaultPath(_)),
            "expected InvalidVaultPath, got {err:?}"
        );
    }

    #[test]
    fn empty_text_is_noop() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(0, "Note");

        // Pure whitespace and empty string both return Ok(None) without
        // creating a file, matching Swift's silent-return behaviour.
        assert!(writer.save_new_entry("", &section, now).unwrap().is_none());
        assert!(writer
            .save_new_entry("   \n\t  ", &section, now)
            .unwrap()
            .is_none());

        let expected_path = tmp.path().join("Daily").join("2026-04-20.md");
        assert!(
            !expected_path.exists(),
            "no file should have been created for blank text"
        );
    }

    #[test]
    fn date_format_uses_zh_cn_locale() {
        let tmp = TempDir::new().unwrap();
        let mut settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        settings.date_format = "yyyy M月d日 EEEE".to_string();
        let writer = DailyNoteWriter::new(settings);

        // 2026-02-27 is a Friday in UTC.
        let now = fixed_time(2026, 2, 27, 22, 35);
        let section = NoteSection::new(0, "Note");
        let written = writer
            .save_new_entry("text", &section, now)
            .unwrap()
            .unwrap();

        assert_eq!(
            written.path,
            tmp.path().join("Daily").join("2026 2月27日 星期五.md")
        );
    }

    #[test]
    fn entries_separated_by_blank_line_only() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let t1 = fixed_time(2026, 4, 20, 12, 0);
        let t2 = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");

        let _first = writer.save_new_entry("one", &section, t1).unwrap().unwrap();
        let written = writer.save_new_entry("two", &section, t2).unwrap().unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        assert!(
            !actual.contains("\n---\n"),
            "must not insert `---` separator: `{actual}`"
        );
        assert!(
            !actual.contains("\n***\n"),
            "must not insert `***` separator: `{actual}`"
        );
        // Two entries under the same section. The Swift insert rule places
        // the new entry immediately after the first newline of the header
        // line (index 7 in `# Note\n\n…`), producing the byte sequence
        // below. Verified against the Swift source by running
        // `DailyNoteWriter.insert` twice with the same inputs; we mirror
        // it exactly rather than "improving" the spacing.
        let expected =
            "# Note\n```\ntwo\n2026-04-20 12:05\n```\n\n\n```\none\n2026-04-20 12:00\n```\n\n";
        assert_eq!(actual, expected);
    }

    // ---------------------------------------------------------------------
    // Unit tests for body builders (no filesystem)
    // ---------------------------------------------------------------------

    #[test]
    fn code_block_body_matches_swift() {
        let out = code_block_body("hello", "2026-01-01 00:00");
        assert_eq!(out, "```\nhello\n2026-01-01 00:00\n```");
    }

    #[test]
    fn plain_text_body_matches_swift() {
        let out = plain_text_body("hello", "2026-01-01 00:00");
        assert_eq!(out, "hello\n2026-01-01 00:00");
    }

    #[test]
    fn markdown_entry_wrapper_appends_double_newline() {
        assert_eq!(markdown_entry("body"), "body\n\n");
    }

    #[test]
    fn quoted_text_handles_blank_lines() {
        assert_eq!(markdown_quote_body_thread("a\n\nb"), "> a\n>\n> b");
    }

    #[test]
    fn quoted_text_single_line() {
        assert_eq!(markdown_quote_body_thread("hello"), "> hello");
    }

    // ---------------------------------------------------------------------
    // Unit tests for section insertion
    // ---------------------------------------------------------------------

    #[test]
    fn insert_into_empty_content_creates_header_and_entry() {
        let out = insert_entry("", "# Note", "entry\n\n");
        assert_eq!(out, "# Note\n\nentry\n\n");
    }

    #[test]
    fn insert_into_whitespace_only_content_treats_as_empty() {
        let out = insert_entry("   \n\n  ", "# Note", "entry\n\n");
        assert_eq!(out, "# Note\n\nentry\n\n");
    }

    #[test]
    fn insert_under_existing_header_with_trailing_newline() {
        let out = insert_entry("# Note\n", "# Note", "entry\n\n");
        // `# Note\n` → line_break_pos = 6, insert_index = 7 == content.len()
        // → prefix = "\n" (line_break_pos is Some). Result keeps header +
        // newline, inserts "\n" + entry.
        assert_eq!(out, "# Note\n\nentry\n\n");
    }

    #[test]
    fn insert_under_existing_header_no_trailing_newline() {
        // Header is at end of file with NO trailing newline.
        let out = insert_entry("# Note", "# Note", "entry\n\n");
        // line_break_pos = None, insert_index = content.len() = 6,
        // prefix = "\n\n".
        assert_eq!(out, "# Note\n\nentry\n\n");
    }

    #[test]
    fn insert_appends_after_existing_content_when_header_missing() {
        let out = insert_entry("other content", "# Note", "entry\n\n");
        assert_eq!(out, "other content\n\n# Note\n\nentry\n\n");
    }

    #[test]
    fn insert_appends_with_single_newline_when_content_ends_with_newline() {
        let out = insert_entry("other\n", "# Note", "entry\n\n");
        assert_eq!(out, "other\n\n# Note\n\nentry\n\n");
    }

    // ---------------------------------------------------------------------
    // Phase 2.5: AppendToLatestEntry — end-to-end writer tests
    // ---------------------------------------------------------------------

    /// Helper: seeds a daily file with `initial` contents, then calls `save`
    /// with the given mode and returns the resulting file bytes.
    fn run_save(
        theme: EntryTheme,
        initial: Option<&str>,
        text: &str,
        section: &NoteSection,
        mode: SaveMode,
        now: DateTime<Utc>,
    ) -> (TempDir, String) {
        let tmp = TempDir::new().unwrap();
        let settings = FakeDailySettings::new(tmp.path().to_path_buf(), theme);
        let writer = DailyNoteWriter::new(settings);

        let file_path = writer.daily_file_path(now).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        if let Some(seed) = initial {
            fs::write(&file_path, seed).unwrap();
        }

        let _ = writer.save(text, section, mode, now).unwrap().unwrap();
        let actual = fs::read_to_string(&file_path).unwrap();
        (tmp, actual)
    }

    #[test]
    fn save_create_mode_matches_save_new_entry() {
        // Behavioural equivalence: save(..., CreateNewEntry, ...) must
        // produce the same bytes as save_new_entry(...).
        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(0, "Note");

        let (_a, out_a) = run_save(
            EntryTheme::CodeBlockClassic,
            None,
            "hello",
            &section,
            SaveMode::CreateNewEntry,
            now,
        );

        // Re-run through save_new_entry for parity.
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);
        let p = writer.daily_file_path(now).unwrap();
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        writer.save_new_entry("hello", &section, now).unwrap();
        let out_b = fs::read_to_string(&p).unwrap();

        assert_eq!(out_a, out_b);
    }

    #[test]
    fn append_code_block_inserts_before_closing_fence() {
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\n\n```\nhello\n2026-04-20 12:00\n```\n\n";

        let (_tmp, actual) = run_save(
            EntryTheme::CodeBlockClassic,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        // Byte-for-byte parity with the Swift splice: prev byte before
        // closing fence is `\n`, so prefix is "" and the chunk is inserted
        // as `---\n{text}\n{ts}\n`.
        let expected = concat!(
            "# Note\n\n",
            "```\n",
            "hello\n",
            "2026-04-20 12:00\n",
            "---\n",
            "follow up\n",
            "2026-04-20 12:05\n",
            "```\n\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_code_block_prefixes_newline_when_fence_lacks_preceding_break() {
        // Opening fence with content directly up against the closing fence
        // (no newline before ```) forces prefix = "\n".
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\n\n```hello```";

        let (_tmp, actual) = run_save(
            EntryTheme::CodeBlockClassic,
            Some(seed),
            "more",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = "# Note\n\n```hello\n---\nmore\n2026-04-20 12:05\n```".to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_plain_text_inserts_after_first_timestamp_line() {
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\n\nhello\n2026-04-20 12:00\n\n";

        let (_tmp, actual) = run_save(
            EntryTheme::PlainTextTimestamp,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\n\n",
            "hello\n",
            "2026-04-20 12:00\n",
            "---\n",
            "follow up\n",
            "2026-04-20 12:05\n\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_plain_text_handles_crlf_seed() {
        // Regression: a CRLF-saved daily note (Windows-native or imported
        // from Git-for-Windows) must still be recognised as having a prior
        // timestamp line. If we failed to strip the `\r` before matching
        // the 16-byte shape, the parser returned None and the writer
        // silently fell back to `CreateNewEntry`, producing a duplicate
        // `## HH:MM` block at the top instead of appending under the
        // existing one.
        //
        // Output line endings are intentionally mixed: existing CRLF
        // bytes are preserved verbatim, the spliced chunk is LF-only
        // (matching Swift's LF-only string literals). The `12:00` line's
        // original `\r\n` ends up visually "split" — `\n` terminates
        // `12:00`, and the `\r\n` that followed it now terminates the
        // new `12:05` line. Every `\r` in the output is still paired
        // with a following `\n`, so no orphan carriage returns are
        // introduced; downstream Markdown renderers accept mixed
        // terminators.
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\r\n\r\nhello\r\n2026-04-20 12:00\r\n\r\n";

        let (_tmp, actual) = run_save(
            EntryTheme::PlainTextTimestamp,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\r\n\r\n",
            "hello\r\n",
            "2026-04-20 12:00",
            "\n---\nfollow up\n2026-04-20 12:05",
            "\r\n\r\n",
        );
        assert_eq!(actual, expected);
        // Sanity: no bare `\r` (every `\r` followed by `\n`).
        let bytes = actual.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\r' {
                assert_eq!(
                    bytes.get(i + 1),
                    Some(&b'\n'),
                    "bare \\r at byte {i} in {actual:?}",
                );
            }
        }
    }

    #[test]
    fn append_code_block_handles_crlf_seed() {
        // CRLF seed — the opening/closing fences are located by ASCII
        // `str::find("```")` so CRLF is transparent to the anchor search.
        // The preceding-byte check for the prefix sees `\n` (LF half of
        // the final `\r\n`) and selects prefix = "", matching the LF
        // behaviour. Existing CRLF bytes preserved, spliced chunk LF.
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\r\n\r\n```\r\nhello\r\n2026-04-20 12:00\r\n```\r\n\r\n";

        let (_tmp, actual) = run_save(
            EntryTheme::CodeBlockClassic,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\r\n\r\n",
            "```\r\n",
            "hello\r\n",
            "2026-04-20 12:00\r\n",
            "---\nfollow up\n2026-04-20 12:05\n",
            "```\r\n\r\n",
        );
        assert_eq!(actual, expected);
        // No bare `\r`.
        let bytes = actual.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\r' {
                assert_eq!(bytes.get(i + 1), Some(&b'\n'), "bare \\r at byte {i}");
            }
        }
    }

    #[test]
    fn append_markdown_quote_handles_crlf_seed() {
        // CRLF seed. `first_quote_block_start` tests `starts_with('>')`
        // on the line (which includes the trailing `\r`), so it still
        // recognises quoted lines. `callout_block_end` stops at the
        // trailing blank (`\r`-prefixed line fails `starts_with(">")`).
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\r\n\r\n> hello\r\n>\r\n> 2026-04-20 12:00\r\n\r\n";

        let (_tmp, actual) = run_save(
            EntryTheme::MarkdownQuote,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\r\n\r\n",
            "> hello\r\n",
            ">\r\n",
            "> 2026-04-20 12:00\r\n",
            "\n> ---\n> follow up\n>\n> 2026-04-20 12:05\n",
            "\r\n",
        );
        assert_eq!(actual, expected);
        // No bare `\r`.
        let bytes = actual.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\r' {
                assert_eq!(bytes.get(i + 1), Some(&b'\n'), "bare \\r at byte {i}");
            }
        }
    }

    #[test]
    fn append_plain_text_first_timestamp_wins_when_multiple() {
        // Two timestamps present. Swift's regex matches the first one
        // (earliest line offset), and we must mirror that — even if the
        // second is logically newer.
        let now = fixed_time(2026, 4, 20, 12, 10);
        let section = NoteSection::new(0, "Note");
        let seed = concat!(
            "# Note\n\n",
            "first\n",
            "2026-04-20 12:00\n",
            "---\n",
            "second\n",
            "2026-04-20 12:05\n\n",
        );

        let (_tmp, actual) = run_save(
            EntryTheme::PlainTextTimestamp,
            Some(seed),
            "third",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\n\n",
            "first\n",
            "2026-04-20 12:00\n",
            "---\n",
            "third\n",
            "2026-04-20 12:10\n",
            "---\n",
            "second\n",
            "2026-04-20 12:05\n\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_markdown_quote_extends_callout_block() {
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        // Seed has a trailing blank line after the callout, which Swift's
        // `calloutBlockEnd` stops at — so the chunk's leading `\n` lands
        // *before* the existing blank, producing `\n\n` between the old
        // timestamp and the new `> ---`.
        let seed = "# Note\n\n> hello\n>\n> 2026-04-20 12:00\n\n";

        let (_tmp, actual) = run_save(
            EntryTheme::MarkdownQuote,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\n\n",
            "> hello\n",
            ">\n",
            "> 2026-04-20 12:00\n",
            "\n",
            "> ---\n",
            "> follow up\n",
            ">\n",
            "> 2026-04-20 12:05\n",
            "\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_markdown_quote_stops_before_nested_callout() {
        // A `> [!note]` line below the anchor terminates the callout block;
        // the splice must land before it, not at the true end of the
        // outermost block.
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = concat!(
            "# Note\n\n",
            "> hello\n",
            ">\n",
            "> 2026-04-20 12:00\n",
            "> [!note] nested\n",
            "> inner line\n\n",
        );

        let (_tmp, actual) = run_save(
            EntryTheme::MarkdownQuote,
            Some(seed),
            "follow up",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\n\n",
            "> hello\n",
            ">\n",
            "> 2026-04-20 12:00\n",
            "\n",
            "> ---\n",
            "> follow up\n",
            ">\n",
            "> 2026-04-20 12:05\n",
            "> [!note] nested\n",
            "> inner line\n\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_falls_back_to_create_when_section_missing() {
        // Seed has a different section. Append has no anchor, so behaviour
        // must match CreateNewEntry: add `# Note` + themed entry at EOF.
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Other\n\nother text\n";

        let (_tmp, actual) = run_save(
            EntryTheme::CodeBlockClassic,
            Some(seed),
            "new",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Other\n\n",
            "other text\n",
            "\n",
            "# Note\n\n",
            "```\nnew\n2026-04-20 12:05\n```\n\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_falls_back_to_create_when_anchor_missing_under_section() {
        // `# Note` exists but has no fence — code-block appender returns
        // None, caller falls back to create-new which inserts right under
        // the header.
        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(0, "Note");
        let seed = "# Note\n\nno fence here\n";

        let (_tmp, actual) = run_save(
            EntryTheme::CodeBlockClassic,
            Some(seed),
            "new",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        // Fallback uses insert_entry with prefix="" (since byte after
        // header+newline is `\n`), so the entry slots in between the header
        // blank line and the existing body, matching
        // `entries_separated_by_blank_line_only`'s insertion pattern.
        let expected = concat!(
            "# Note\n",
            "```\nnew\n2026-04-20 12:05\n```\n\n",
            "\nno fence here\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_scopes_to_current_section_only() {
        // Two sections, each with their own fence. Appending under the
        // second section must NOT touch the first.
        let now = fixed_time(2026, 4, 20, 12, 10);
        let section = NoteSection::new(1, "Clip");
        let seed = concat!(
            "# Note\n\n",
            "```\nalpha\n2026-04-20 12:00\n```\n\n",
            "# Clip\n\n",
            "```\nbeta\n2026-04-20 12:05\n```\n\n",
        );

        let (_tmp, actual) = run_save(
            EntryTheme::CodeBlockClassic,
            Some(seed),
            "gamma",
            &section,
            SaveMode::AppendToLatestEntry,
            now,
        );

        let expected = concat!(
            "# Note\n\n",
            "```\nalpha\n2026-04-20 12:00\n```\n\n",
            "# Clip\n\n",
            "```\n",
            "beta\n",
            "2026-04-20 12:05\n",
            "---\n",
            "gamma\n",
            "2026-04-20 12:10\n",
            "```\n\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn append_empty_text_is_noop() {
        // Matches save_new_entry: whitespace-only text never touches disk.
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let section = NoteSection::new(0, "Note");

        assert!(writer
            .save("", &section, SaveMode::AppendToLatestEntry, now)
            .unwrap()
            .is_none());
        assert!(writer
            .save("   \n\t  ", &section, SaveMode::AppendToLatestEntry, now)
            .unwrap()
            .is_none());
        assert!(!writer.daily_file_path(now).unwrap().exists());
    }

    #[test]
    fn append_honors_custom_section_header() {
        // When `header_for` returns a custom title, the append parser must
        // locate *that* header rather than the default `NoteSection::header`.
        let tmp = TempDir::new().unwrap();
        let mut settings =
            FakeDailySettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        settings.custom_header = Some("# Ideas".to_string());
        let writer = DailyNoteWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 5);
        let section = NoteSection::new(5, "Ignored");

        let file_path = writer.daily_file_path(now).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(
            &file_path,
            "# Ideas\n\n```\nfirst\n2026-04-20 12:00\n```\n\n",
        )
        .unwrap();

        let _ = writer
            .save("second", &section, SaveMode::AppendToLatestEntry, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&file_path).unwrap();
        let expected = concat!(
            "# Ideas\n\n",
            "```\nfirst\n2026-04-20 12:00\n",
            "---\nsecond\n2026-04-20 12:05\n",
            "```\n\n",
        );
        assert_eq!(actual, expected);
    }

    // ---------------------------------------------------------------------
    // Phase 2.5: unit tests for the parser helpers
    // ---------------------------------------------------------------------

    #[test]
    fn section_body_range_slices_between_headers() {
        let content = "# A\n\nfirst body\n\n# B\n\nsecond body\n";
        let a = section_body_range(content, "# A").unwrap();
        assert_eq!(&content[a.clone()], "\nfirst body\n");
        let b = section_body_range(content, "# B").unwrap();
        assert_eq!(&content[b.clone()], "\nsecond body\n");
    }

    #[test]
    fn section_body_range_returns_none_when_header_missing() {
        assert_eq!(section_body_range("# Other\n\nbody\n", "# Missing"), None);
    }

    #[test]
    fn section_body_range_empty_body_at_eof() {
        // Header sits at EOF with NO trailing newline: range collapses to
        // `content.len()..content.len()`.
        let content = "# Note";
        let r = section_body_range(content, "# Note").unwrap();
        assert_eq!(r, content.len()..content.len());
        assert_eq!(&content[r], "");
    }

    #[test]
    fn section_body_range_includes_leading_newline_when_blank_line_follows() {
        // Header is `# Note\n` and next char is `\n` — section starts AT
        // that second `\n`, so the leading `\n` is part of the range. This
        // matches Swift exactly (we traced it through
        // `content.index(after: afterHeaderLineBreak)`).
        let content = "# Note\n\nbody\n";
        let r = section_body_range(content, "# Note").unwrap();
        assert_eq!(&content[r], "\nbody\n");
    }

    #[test]
    fn first_quote_block_start_finds_leading_gt_line() {
        assert_eq!(first_quote_block_start("\n> hello\n> world\n"), Some(1));
        assert_eq!(first_quote_block_start("plain\n> quote\n"), Some(6));
    }

    #[test]
    fn first_quote_block_start_returns_none_without_gt() {
        assert_eq!(first_quote_block_start(""), None);
        assert_eq!(first_quote_block_start("no quote here\n"), None);
        // A line that contains `>` somewhere but doesn't start with it is
        // not a quote line.
        assert_eq!(first_quote_block_start("foo > bar\n"), None);
    }

    #[test]
    fn callout_block_end_stops_on_non_gt_line() {
        let section = "> a\n> b\nplain\n";
        let end = callout_block_end(section, 0);
        assert_eq!(&section[..end], "> a\n> b\n");
    }

    #[test]
    fn callout_block_end_returns_section_len_when_all_gt() {
        let section = "> a\n> b\n";
        assert_eq!(callout_block_end(section, 0), section.len());
    }

    #[test]
    fn callout_block_end_stops_before_nested_callout_but_not_at_start() {
        // The anchor line MAY be `> [!…` (that's how nested blocks are
        // identified in the first place). But a subsequent `> [!…` ends
        // the outer block.
        let section = "> [!quote] outer\n> body\n> [!note] inner\n> inner body\n";
        let end = callout_block_end(section, 0);
        // End should be at the start of the `> [!note] inner` line.
        let expected_end = section.find("> [!note]").unwrap();
        assert_eq!(end, expected_end);
    }

    #[test]
    fn callout_block_end_handles_trailing_gt_line_without_newline() {
        let section = "> a\n> b";
        assert_eq!(callout_block_end(section, 0), section.len());
    }

    #[test]
    fn find_first_timestamp_line_matches_single_line() {
        let section = "\nhello\n2026-04-20 12:00\n\n";
        let r = find_first_timestamp_line(section).unwrap();
        assert_eq!(&section[r], "2026-04-20 12:00");
    }

    #[test]
    fn find_first_timestamp_line_returns_first_of_many() {
        let section = "\nhello\n2026-04-20 12:00\n---\nx\n2026-04-20 12:05\n";
        let r = find_first_timestamp_line(section).unwrap();
        assert_eq!(r.start, section.find("2026-04-20 12:00").unwrap());
    }

    #[test]
    fn find_first_timestamp_line_none_when_absent() {
        assert_eq!(find_first_timestamp_line(""), None);
        assert_eq!(find_first_timestamp_line("no timestamps here\n"), None);
        // Suffix/prefix junk on the same line disqualifies it.
        assert_eq!(find_first_timestamp_line("2026-04-20 12:00 x\n"), None);
        assert_eq!(find_first_timestamp_line("x 2026-04-20 12:00\n"), None);
    }

    #[test]
    fn find_first_timestamp_line_handles_crlf_line_endings() {
        // Windows clipboard / existing CRLF-saved notes. Swift's `$` anchor
        // matches before either `\n` or `\r\n`, so the parser must treat the
        // trailing `\r` as part of the line terminator, not the content.
        let section = "\r\nhello\r\n2026-04-20 12:00\r\n\r\n";
        let r = find_first_timestamp_line(section).unwrap();
        assert_eq!(&section[r.clone()], "2026-04-20 12:00");
        // The returned range ends *before* the `\r`, so splicing a new chunk
        // at `r.end` lands between the timestamp and the CRLF pair.
        assert_eq!(&section[r.end..r.end + 2], "\r\n");
    }

    #[test]
    fn is_timestamp_line_rejects_wrong_shapes() {
        assert!(is_timestamp_line("2026-04-20 12:00"));
        assert!(!is_timestamp_line("2026-04-20 12:0")); // too short
        assert!(!is_timestamp_line("2026-04-20 12:000")); // too long
        assert!(!is_timestamp_line("2026/04/20 12:00")); // wrong sep
        assert!(!is_timestamp_line("20 6-04-20 12:00")); // space in year
        assert!(!is_timestamp_line("")); // empty
    }
}

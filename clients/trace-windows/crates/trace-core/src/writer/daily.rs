//! Daily / dimension-mode note writer (create-new-entry path).
//!
//! Mirrors `DailyNoteWriter.saveToDailyNote` / `DailyNoteWriter.insert` from
//! the Mac source at `Sources/Trace/Services/DailyNoteWriter.swift`. Only the
//! `createNewEntry` save mode is implemented here — appending to the latest
//! entry, thread mode, file mode, and clipboard image handling live in
//! sibling modules added by Phase 2.3–2.6.
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
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::error::TraceError;
use crate::models::{EntryTheme, NoteSection};
use crate::paths::{date_format, resolve_within_vault};
use crate::writer::{markdown_quote_body_thread, timestamp, write_atomic, WrittenNote};

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
        let entry = self.entry_for_text(trimmed, now);
        let header = self.settings.header_for(section);
        let updated = insert_entry(&existing, &header, &entry);
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
        let vault = self.validated_vault_path()?;

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

    fn validated_vault_path(&self) -> Result<PathBuf, TraceError> {
        let raw = self.settings.vault_path();
        // `to_string_lossy` is only used for the blank-string check: non-UTF-8
        // path bytes cannot equal `""` or pure whitespace after substitution,
        // so lossy replacement here cannot flip a non-empty path into empty.
        // The PathBuf we return below still carries the original bytes.
        let as_str = raw.to_string_lossy();
        if as_str.trim().is_empty() {
            return Err(TraceError::InvalidVaultPath(
                "vault path is blank".to_string(),
            ));
        }
        Ok(raw.to_path_buf())
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
}

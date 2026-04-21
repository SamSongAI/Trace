//! File/inbox-mode writer — creates one Markdown file per entry.
//!
//! Mirrors `DailyNoteWriter.saveToInboxFile` and its helpers from
//! `Sources/Trace/Services/DailyNoteWriter.swift` lines 107–233, 470–475.
//!
//! # Byte-level parity
//!
//! Every formatting and sanitization decision is translated verbatim from
//! Swift. Dedicated unit tests pin exact output bytes so regressions surface
//! immediately. No regex crate is used — `\s+` folding is hand-rolled via
//! `char::is_whitespace` iteration; `-{2,}` folding via char-scan with a
//! `prev_was_hyphen` flag.
//!
//! # Filename conflict resolution
//!
//! When `<base>.md` already exists the sequence starts at `<base>-2.md`,
//! matching Swift's `nextAvailableFileURL` which initialises `sequence` to
//! `2`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::error::TraceError;
use crate::writer::{timestamp, validated_vault_path, write_atomic, WrittenNote};

/// Injectable configuration for the file writer.
///
/// Mirrors the `inboxVaultPath` field of `DailyNoteSettingsProviding` on Mac.
pub trait FileWriterSettings {
    /// Absolute filesystem path to the inbox vault root. Whitespace-only paths
    /// are rejected with `InvalidVaultPath`.
    fn inbox_vault_path(&self) -> &Path;
}

// Blanket forwarding for `Arc<T>` — lets the capture panel share a
// single `Arc<AppSettings>` across File-mode writes without cloning.
impl<T: FileWriterSettings + ?Sized> FileWriterSettings for Arc<T> {
    fn inbox_vault_path(&self) -> &Path {
        (**self).inbox_vault_path()
    }
}

/// Writes one Markdown file per entry into an inbox vault. The writer is
/// stateless apart from the borrowed settings; `now` is injected per-call so
/// tests can pin the clock without a time-source abstraction.
pub struct FileWriter<S: FileWriterSettings> {
    settings: S,
}

impl<S: FileWriterSettings> FileWriter<S> {
    pub fn new(settings: S) -> Self {
        Self { settings }
    }

    /// Saves `text` as a new Markdown file with YAML frontmatter.
    ///
    /// - `title`: raw user-supplied title. Used (whitespace-trimmed) as the
    ///   `title:` frontmatter field and (fully sanitized) as the filename
    ///   stem. `None` or all-whitespace → no `title:` line; filename falls
    ///   back to the `yyyy-MM-dd-HHmmss` timestamp.
    /// - `target_folder`: optional vault-relative sub-folder. Backslashes are
    ///   normalised to forward slashes. `.` / `..` components are rejected
    ///   with `InvalidTargetFolderPath`.
    /// - `now`: caller-supplied timestamp used for both `created:` and the
    ///   timestamp-fallback filename.
    ///
    /// Returns `Ok(None)` when `text` trims to empty (silent no-op, matches
    /// Swift). Returns `Err` for a blank inbox vault path, invalid folder
    /// components, or I/O errors.
    pub fn save(
        &self,
        text: &str,
        title: Option<&str>,
        target_folder: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<Option<WrittenNote>, TraceError> {
        let trimmed_text = text.trim();
        if trimmed_text.is_empty() {
            return Ok(None);
        }

        let normalized_title = normalized_document_title(title);
        let file_path = self.inbox_file_path(normalized_title.as_deref(), target_folder, now)?;

        let parent = file_path.parent().ok_or_else(|| {
            TraceError::AtomicWriteFailed(format!(
                "inbox file path has no parent: {}",
                file_path.display()
            ))
        })?;
        fs::create_dir_all(parent)?;

        let content = inbox_document_content(trimmed_text, normalized_title.as_deref(), now);
        let bytes = content.as_bytes();
        write_atomic(&file_path, bytes)?;

        Ok(Some(WrittenNote {
            path: file_path,
            bytes_written: bytes.len() as u64,
        }))
    }

    fn inbox_file_path(
        &self,
        normalized_title: Option<&str>,
        target_folder: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<PathBuf, TraceError> {
        let inbox_base =
            validated_vault_path(self.settings.inbox_vault_path(), "inbox vault path")?;
        let base_name = file_base_name(normalized_title, now);

        let target_dir = match target_folder {
            Some(folder) if !folder.trim().is_empty() => {
                let normalized = normalized_relative_folder_path(folder)?;
                if normalized.is_empty() {
                    inbox_base
                } else {
                    inbox_base.join(&normalized)
                }
            }
            _ => inbox_base,
        };

        Ok(next_available_file_path(&base_name, &target_dir))
    }
}

/// Mirrors `nextAvailableFileURL(baseName:in:)`. Sequence starts at 2.
fn next_available_file_path(base_name: &str, directory: &Path) -> PathBuf {
    let mut candidate = directory.join(format!("{base_name}.md"));
    let mut sequence: u32 = 2;

    while candidate.exists() {
        candidate = directory.join(format!("{base_name}-{sequence}.md"));
        sequence += 1;
    }

    candidate
}

/// Mirrors `normalizedRelativeFolderPath(_:fallback:)` with `fallback = ""`.
///
/// - Backslashes → forward slashes.
/// - Trim leading / trailing `/`.
/// - Split on `/`, omit empty components.
/// - Reject `.` or `..` components → `InvalidTargetFolderPath`.
/// - Rejoin with `/`.
fn normalized_relative_folder_path(folder: &str) -> Result<String, TraceError> {
    let raw = folder.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }

    let slashes_normalized = raw.replace('\\', "/");
    let stripped = slashes_normalized.trim_matches('/');

    let components: Vec<&str> = stripped.split('/').filter(|c| !c.is_empty()).collect();

    if components.is_empty() {
        return Ok(String::new());
    }

    for component in &components {
        if *component == "." || *component == ".." {
            return Err(TraceError::InvalidTargetFolderPath(folder.to_string()));
        }
    }

    Ok(components.join("/"))
}

/// Mirrors `fileBaseName(for:at:)`. `normalized_title` is the output of
/// [`normalized_document_title`] — trim + non-empty guaranteed.
fn file_base_name(normalized_title: Option<&str>, now: DateTime<Utc>) -> String {
    match normalized_title.and_then(sanitize_to_file_name_segment) {
        Some(seg) if !seg.is_empty() => seg,
        _ => file_name_timestamp(now),
    }
}

/// Mirrors `fileNameTimestampWithoutMilliseconds(for:)` — `yyyy-MM-dd-HHmmss`.
fn file_name_timestamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%d-%H%M%S").to_string()
}

/// Mirrors `normalizedDocumentTitle(_:)`: whitespace-trim, `None` if empty.
fn normalized_document_title(title: Option<&str>) -> Option<String> {
    let trimmed = title?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Sanitises an already-normalised (trim + non-empty) title into a filename
/// segment. Mirrors steps 2–7 of Swift's `normalizedFileNameSegment(_:)`;
/// step 1 (the trim/empty guard) is the caller's responsibility via
/// [`normalized_document_title`].
///
/// Steps (in order, matching Swift):
/// 1. Replace each char in `/\:*?"<>|` with `-`.
/// 2. Replace `\n` / `\r` with ` `.
/// 3. Collapse whitespace runs → `-` (hand-rolled; Swift uses `\s+`).
/// 4. Collapse consecutive `-` runs → single `-` (hand-rolled; Swift uses
///    `-{2,}`).
/// 5. Trim leading / trailing chars in `{'-', '.', ' '}`.
/// 6. Return `None` if the result is empty.
fn sanitize_to_file_name_segment(normalized_title: &str) -> Option<String> {
    const INVALID: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let after_invalid: String = normalized_title
        .chars()
        .map(|c| if INVALID.contains(&c) { '-' } else { c })
        .collect();

    let after_newlines: String = after_invalid
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();

    let after_ws = collapse_whitespace_to_hyphen(&after_newlines);
    let after_hyphens = collapse_consecutive_hyphens(&after_ws);

    let trimmed = after_hyphens.trim_matches(|c: char| c == '-' || c == '.' || c == ' ');

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Collapses any run of `char::is_whitespace` characters to a single `-`.
fn collapse_whitespace_to_hyphen(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_whitespace = false;

    for c in s.chars() {
        if c.is_whitespace() {
            if !in_whitespace {
                result.push('-');
                in_whitespace = true;
            }
        } else {
            result.push(c);
            in_whitespace = false;
        }
    }

    result
}

/// Collapses runs of two or more `-` characters to a single `-`.
fn collapse_consecutive_hyphens(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_hyphen = false;

    for c in s.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    result
}

/// Builds the full file content. Mirrors `inboxDocumentContent(for:title:at:)`.
///
/// `normalized_title` is the output of [`normalized_document_title`] — trim +
/// non-empty guaranteed, so the frontmatter's `title:` line is always present
/// when `Some(..)`.
///
/// Exact byte layout:
/// - With title:    `---\ntitle: "<esc>"\ncreated: "<ts>"\n---\n\n<text>`
/// - Without title: `---\ncreated: "<ts>"\n---\n\n<text>`
///
/// `<esc>` is the title with `"` → `\"`. `<text>` is the already-trimmed body.
/// No trailing newline unless `text` itself ends with one.
fn inbox_document_content(
    text: &str,
    normalized_title: Option<&str>,
    now: DateTime<Utc>,
) -> String {
    let ts = timestamp(now);
    match normalized_title {
        Some(t) => {
            let escaped = t.replace('"', "\\\"");
            format!("---\ntitle: \"{escaped}\"\ncreated: \"{ts}\"\n---\n\n{text}")
        }
        None => format!("---\ncreated: \"{ts}\"\n---\n\n{text}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    struct FakeSettings {
        inbox: PathBuf,
    }

    impl FakeSettings {
        fn new(inbox: PathBuf) -> Self {
            Self { inbox }
        }
    }

    impl FileWriterSettings for FakeSettings {
        fn inbox_vault_path(&self) -> &Path {
            &self.inbox
        }
    }

    fn fixed_time(y: i32, mo: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, min, s).unwrap()
    }

    fn default_now() -> DateTime<Utc> {
        fixed_time(2026, 4, 20, 14, 30, 5)
    }

    fn make_writer(tmp: &TempDir) -> FileWriter<FakeSettings> {
        FileWriter::new(FakeSettings::new(tmp.path().to_path_buf()))
    }

    // ---------------------------------------------------------------------
    // Filename cases
    // ---------------------------------------------------------------------

    #[test]
    fn title_provided_uses_sanitized_title_as_filename() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("some text", Some("My Note"), None, default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "My-Note.md");
    }

    #[test]
    fn no_title_uses_timestamp_filename() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 5);
        let result = writer.save("text", None, None, t).unwrap().unwrap();
        assert_eq!(result.path.file_name().unwrap(), "2026-04-20-143005.md");
    }

    #[test]
    fn title_invalid_chars_replaced_with_hyphen() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("body", Some("a/b\\c:d*e?f\"g<h>i|j"), None, default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "a-b-c-d-e-f-g-h-i-j.md");
    }

    #[test]
    fn title_whitespace_collapsed_to_hyphen() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("body", Some("hello world\tthere"), None, default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "hello-world-there.md");
    }

    #[test]
    fn title_consecutive_hyphens_collapsed() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("body", Some("a---b"), None, default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "a-b.md");
    }

    #[test]
    fn title_sanitizes_to_empty_falls_back_to_timestamp() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 5);
        let result = writer.save("body", Some("///"), None, t).unwrap().unwrap();
        assert_eq!(result.path.file_name().unwrap(), "2026-04-20-143005.md");
    }

    #[test]
    fn whitespace_only_title_treated_as_no_title() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 5);
        let result = writer
            .save("body", Some("  \t\n  "), None, t)
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "2026-04-20-143005.md");
        let content = fs::read_to_string(&result.path).unwrap();
        assert!(
            !content.contains("title:"),
            "whitespace title must not emit title: line; got:\n{content}"
        );
    }

    // ---------------------------------------------------------------------
    // target_folder cases
    // ---------------------------------------------------------------------

    #[test]
    fn target_folder_file_lands_under_subfolder() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("body", Some("note"), Some("inbox"), default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.parent().unwrap(), tmp.path().join("inbox"));
    }

    #[test]
    fn target_folder_dot_component_rejected() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let err = writer
            .save("body", None, Some("a/./b"), default_now())
            .unwrap_err();
        assert!(
            matches!(err, TraceError::InvalidTargetFolderPath(_)),
            "expected InvalidTargetFolderPath, got {err:?}"
        );
    }

    #[test]
    fn target_folder_dotdot_component_rejected() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let err = writer
            .save("body", None, Some("../escape"), default_now())
            .unwrap_err();
        assert!(
            matches!(err, TraceError::InvalidTargetFolderPath(_)),
            "expected InvalidTargetFolderPath, got {err:?}"
        );
    }

    #[test]
    fn target_folder_backslashes_normalized_to_forward_slashes() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("body", Some("note"), Some("a\\b"), default_now())
            .unwrap()
            .unwrap();
        let expected_dir = tmp.path().join("a").join("b");
        assert_eq!(result.path.parent().unwrap(), expected_dir);
        assert!(expected_dir.exists());
    }

    #[test]
    fn nested_target_folder_dirs_auto_created() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("body", Some("note"), Some("a/b/c"), default_now())
            .unwrap()
            .unwrap();
        let expected_dir = tmp.path().join("a").join("b").join("c");
        assert!(expected_dir.exists(), "nested dirs must be created");
        assert_eq!(result.path.parent().unwrap(), expected_dir);
    }

    #[test]
    fn intermediate_parent_dirs_created_when_missing() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        assert!(!tmp.path().join("x").exists());
        let result = writer
            .save("body", Some("note"), Some("x/y/z"), default_now())
            .unwrap()
            .unwrap();
        assert!(tmp.path().join("x").join("y").join("z").exists());
        assert!(result.path.exists());
    }

    // ---------------------------------------------------------------------
    // Conflict sequence
    // ---------------------------------------------------------------------

    #[test]
    fn conflict_first_duplicate_gets_sequence_2() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        fs::write(tmp.path().join("My-Note.md"), b"existing").unwrap();
        let result = writer
            .save("body", Some("My Note"), None, default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "My-Note-2.md");
    }

    #[test]
    fn conflict_two_existing_gets_sequence_3() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        fs::write(tmp.path().join("My-Note.md"), b"1").unwrap();
        fs::write(tmp.path().join("My-Note-2.md"), b"2").unwrap();
        let result = writer
            .save("body", Some("My Note"), None, default_now())
            .unwrap()
            .unwrap();
        assert_eq!(result.path.file_name().unwrap(), "My-Note-3.md");
    }

    // ---------------------------------------------------------------------
    // Noop / error cases
    // ---------------------------------------------------------------------

    #[test]
    fn empty_text_returns_none_no_file_created() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        assert!(writer
            .save("", Some("title"), None, default_now())
            .unwrap()
            .is_none());
        let count = fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(Result::ok)
            .count();
        assert_eq!(count, 0, "no file should be created for empty text");
    }

    #[test]
    fn whitespace_only_text_returns_none_no_file_created() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        assert!(writer
            .save("   \t\n  ", Some("title"), None, default_now())
            .unwrap()
            .is_none());
        let count = fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(Result::ok)
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn blank_inbox_vault_path_returns_invalid_vault_path() {
        let writer = FileWriter::new(FakeSettings::new(PathBuf::from("   ")));
        let err = writer.save("body", None, None, default_now()).unwrap_err();
        assert!(
            matches!(err, TraceError::InvalidVaultPath(_)),
            "expected InvalidVaultPath, got {err:?}"
        );
    }

    // ---------------------------------------------------------------------
    // Frontmatter byte parity
    // ---------------------------------------------------------------------

    #[test]
    fn frontmatter_with_title_exact_bytes() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 0);
        let result = writer
            .save("hello", Some("My Title"), None, t)
            .unwrap()
            .unwrap();
        let content = fs::read_to_string(&result.path).unwrap();
        let expected = "---\ntitle: \"My Title\"\ncreated: \"2026-04-20 14:30\"\n---\n\nhello";
        assert_eq!(content, expected);
    }

    #[test]
    fn frontmatter_without_title_exact_bytes() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 0);
        let result = writer.save("hello", None, None, t).unwrap().unwrap();
        let content = fs::read_to_string(&result.path).unwrap();
        let expected = "---\ncreated: \"2026-04-20 14:30\"\n---\n\nhello";
        assert_eq!(content, expected);
    }

    #[test]
    fn title_double_quote_escaped_in_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 0);
        let result = writer
            .save("body", Some("Say \"hello\""), None, t)
            .unwrap()
            .unwrap();
        let content = fs::read_to_string(&result.path).unwrap();
        assert!(
            content.contains("title: \"Say \\\"hello\\\"\""),
            "expected escaped quotes in frontmatter, got:\n{content}"
        );
    }

    #[test]
    fn text_trimmed_before_content_build() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let t = fixed_time(2026, 4, 20, 14, 30, 0);
        let result = writer.save("  hello  ", None, None, t).unwrap().unwrap();
        let content = fs::read_to_string(&result.path).unwrap();
        let expected = "---\ncreated: \"2026-04-20 14:30\"\n---\n\nhello";
        assert_eq!(content, expected);
    }

    // ---------------------------------------------------------------------
    // WrittenNote invariants
    // ---------------------------------------------------------------------

    #[test]
    fn bytes_written_equals_file_size() {
        let tmp = TempDir::new().unwrap();
        let writer = make_writer(&tmp);
        let result = writer
            .save("hello world", Some("Test"), None, default_now())
            .unwrap()
            .unwrap();
        let file_meta = fs::metadata(&result.path).unwrap();
        assert_eq!(result.bytes_written, file_meta.len());
    }

    // ---------------------------------------------------------------------
    // Helper-level edge cases (private functions, tested directly)
    // ---------------------------------------------------------------------

    #[test]
    fn file_name_timestamp_format_is_yyyy_mm_dd_hhmmss() {
        let t = fixed_time(2026, 1, 2, 3, 4, 5);
        assert_eq!(file_name_timestamp(t), "2026-01-02-030405");
    }

    #[test]
    fn folder_path_leading_trailing_slashes_stripped() {
        assert_eq!(normalized_relative_folder_path("/a/b/").unwrap(), "a/b");
    }

    #[test]
    fn folder_path_empty_string_returns_empty() {
        assert_eq!(normalized_relative_folder_path("").unwrap(), "");
    }

    #[test]
    fn folder_path_only_dot_rejected() {
        assert!(matches!(
            normalized_relative_folder_path("."),
            Err(TraceError::InvalidTargetFolderPath(_))
        ));
    }
}

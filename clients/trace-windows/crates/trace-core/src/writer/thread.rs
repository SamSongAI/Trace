//! Thread-mode note writer (create-new-entry + append-to-latest-entry).
//!
//! Mirrors `ThreadWriter.save` from `Sources/Trace/Services/ThreadWriter.swift`
//! (Mac reference). A *thread* is a single `.md` file with a top-level
//! `# {thread.name}` heading followed by a stream of `## {timestamp}` entries
//! in reverse-chronological order (newest on top).
//!
//! # Byte-level parity
//!
//! The three entry bodies and the heading-detection rules are translated
//! byte-for-byte from Swift. Dedicated tests pin the exact output so parity
//! regressions surface as test failures. Unlike Daily mode, Thread does not
//! rely on a regex crate — the two heading scans are hand-rolled to avoid a
//! workspace dependency bump.
//!
//! # Path resolution
//!
//! `target_file` may be either:
//! - **Relative** (`"threads/work"`, `"notes/inbox.md"`): joined under
//!   `vault_path` via [`paths::safety::resolve_within_vault`], with a `.md`
//!   suffix auto-appended when missing.
//! - **Absolute** (`"/Users/x/notes/thread.md"`): accepted even if it lands
//!   outside the vault. Swift's intentional escape hatch for users who sync
//!   into a sibling Obsidian vault. We reject the path only if it contains
//!   `..` / `.` / empty components.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::error::TraceError;
use crate::models::{EntryTheme, ThreadConfig};
use crate::paths::resolve_within_vault;
use crate::writer::{markdown_quote_body_thread, timestamp, write_atomic, SaveMode, WrittenNote};

/// Injectable configuration for the thread writer.
///
/// Mirrors Swift's `ThreadSettingsProviding`. Only the fields actually used by
/// the thread writer are surfaced.
pub trait ThreadSettings {
    /// Absolute filesystem path to the note vault. Blank paths raise
    /// `InvalidVaultPath`.
    fn vault_path(&self) -> &Path;

    /// Preset controlling entry body formatting (fenced code block, plain
    /// text + timestamp, or markdown blockquote).
    fn entry_theme(&self) -> EntryTheme;
}

/// Writes thread-mode entries. Holds a clock-independent reference to
/// settings; the `now` timestamp is passed per-call so tests can inject a
/// fixed clock without a time-source abstraction.
pub struct ThreadWriter<S: ThreadSettings> {
    settings: S,
}

impl<S: ThreadSettings> ThreadWriter<S> {
    pub fn new(settings: S) -> Self {
        Self { settings }
    }

    /// Saves `text` to `thread`.
    ///
    /// - `CreateNewEntry`: inserts a fresh `## {ts}` entry above all existing
    ///   ones, creating the file and `# {thread.name}` heading as needed.
    /// - `AppendToLatestEntry`: inserts the new entry right above the first
    ///   `## ` heading so it becomes the newest. Falls back to create mode
    ///   when no `## ` heading exists yet.
    ///
    /// Returns `Ok(None)` when `text` trims to empty (silent no-op, mirrors
    /// Swift). Returns `Err` for invalid vault paths, malformed target files,
    /// path-escape attempts, or filesystem errors.
    pub fn save(
        &self,
        text: &str,
        thread: &ThreadConfig,
        mode: SaveMode,
        now: DateTime<Utc>,
    ) -> Result<Option<WrittenNote>, TraceError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let file_path = self.thread_file_path(thread)?;
        let parent = file_path.parent().ok_or_else(|| {
            TraceError::AtomicWriteFailed(format!(
                "thread file path has no parent: {}",
                file_path.display()
            ))
        })?;
        fs::create_dir_all(parent)?;

        let existing = load_or_empty(&file_path)?;
        let entry = self.entry_for_text(trimmed, now);

        let updated = match mode {
            SaveMode::CreateNewEntry => append_new_entry_to_top(&entry, &existing, &thread.name),
            SaveMode::AppendToLatestEntry => try_append_to_latest_entry(&entry, &existing)
                .unwrap_or_else(|| append_new_entry_to_top(&entry, &existing, &thread.name)),
        };

        let bytes = updated.as_bytes();
        write_atomic(&file_path, bytes)?;

        Ok(Some(WrittenNote {
            path: file_path,
            bytes_written: bytes.len() as u64,
        }))
    }

    /// Resolves the absolute `.md` path for `thread`. Separator normalisation,
    /// traversal checks, and `.md` suffix handling are byte-for-byte with the
    /// Swift reference.
    pub fn thread_file_path(&self, thread: &ThreadConfig) -> Result<PathBuf, TraceError> {
        let vault = self.validated_vault_path()?;

        let normalized = thread.target_file.replace('\\', "/");
        let normalized = normalized.trim();
        if normalized.is_empty() {
            return Err(TraceError::InvalidThreadConfig(
                "target_file is blank".to_string(),
            ));
        }

        let resolved = if normalized.starts_with('/') {
            resolve_absolute_target(normalized)?
        } else {
            resolve_within_vault(&vault, normalized)?
        };

        Ok(ensure_md_extension(resolved))
    }

    fn validated_vault_path(&self) -> Result<PathBuf, TraceError> {
        let raw = self.settings.vault_path();
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
        match self.settings.entry_theme() {
            EntryTheme::CodeBlockClassic => code_block_entry_thread(text, &ts),
            EntryTheme::PlainTextTimestamp => plain_text_entry_thread(text, &ts),
            EntryTheme::MarkdownQuote => markdown_quote_entry_thread(text, &ts),
        }
    }
}

/// Entry body for `codeBlockClassic` theme — mirrors the Swift multiline
/// literal `"""## {ts}\n\n```\n{text}\n```\n\n"""` which produces
/// `"## {ts}\n\n```\n{text}\n```\n\n"`.
fn code_block_entry_thread(text: &str, timestamp: &str) -> String {
    format!("## {timestamp}\n\n```\n{text}\n```\n\n")
}

/// Entry body for `plainTextTimestamp` theme.
fn plain_text_entry_thread(text: &str, timestamp: &str) -> String {
    format!("## {timestamp}\n\n{text}\n\n")
}

/// Entry body for `markdownQuote` theme. Delegates to the shared
/// `markdown_quote_body_thread` helper which prefixes every line with `>`.
fn markdown_quote_entry_thread(text: &str, timestamp: &str) -> String {
    let quoted = markdown_quote_body_thread(text);
    format!("## {timestamp}\n\n{quoted}\n\n")
}

/// Inserts `entry` at the top of `content` (right below the `# {thread_name}`
/// heading).
///
/// Swift reference: `ThreadWriter.append(_:to:for:)` at lines 149–164.
///
/// Contract:
/// 1. If `content` trims to empty, synthesise `"# {thread_name}\n\n{entry}"`.
/// 2. If `content` has a top-level heading line (`# ` or `#\t` at start of a
///    line), insert `entry` immediately after that heading line, with a blank
///    line between the heading and the entry. Leading newlines in the body
///    are dropped (matches Swift's `.drop(while: { $0 == '\n' })`).
/// 3. Otherwise synthesise a heading and keep the existing body after the new
///    entry.
pub(crate) fn append_new_entry_to_top(entry: &str, content: &str, thread_name: &str) -> String {
    if content.trim().is_empty() {
        return format!("# {thread_name}\n\n{entry}");
    }

    if let Some(heading_end) = find_top_level_heading_end(content) {
        let heading = &content[..heading_end];
        let body = &content[heading_end..];
        let body = body.trim_start_matches('\n');
        return format!("{heading}\n{entry}{body}");
    }

    let body = content.trim_start_matches('\n');
    format!("# {thread_name}\n\n{entry}{body}")
}

/// Inserts `entry` immediately before the first `## ` (or `##\t`) line in
/// `content`, so it becomes the newest entry on top. Returns `None` when no
/// `## ` heading is found (caller falls back to create mode).
///
/// Swift reference: `ThreadWriter.tryAppendToLatestEntry` at lines 203–216.
pub(crate) fn try_append_to_latest_entry(entry: &str, content: &str) -> Option<String> {
    let pos = find_h2_heading_start(content)?;
    let mut out = String::with_capacity(content.len() + entry.len());
    out.push_str(&content[..pos]);
    out.push_str(entry);
    out.push_str(&content[pos..]);
    Some(out)
}

/// Returns the byte offset just past the first top-level `# …\n` heading line
/// in `content`, or `None` if none exists.
///
/// A top-level heading is a line starting with `# ` or `#\t` (one `#`
/// followed by ASCII space or tab). The returned offset points one byte past
/// the `\n` that terminates the heading line; if the heading is not
/// terminated by `\n` (i.e. sits at end-of-file without trailing newline),
/// returns `None`.
fn find_top_level_heading_end(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();

    // Check start-of-file first.
    if starts_heading(bytes, 0) {
        return bytes[0..].iter().position(|b| *b == b'\n').map(|i| i + 1);
    }

    // Scan for "\n# " or "\n#\t" elsewhere in the file.
    let mut idx = 0;
    while idx + 2 < bytes.len() {
        if bytes[idx] == b'\n' && starts_heading(bytes, idx + 1) {
            let after = idx + 1;
            return bytes[after..]
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| after + i + 1);
        }
        idx += 1;
    }

    None
}

/// Returns `true` if `bytes[start..]` begins with `# ` or `#\t` — i.e. the
/// start of a top-level markdown heading line.
fn starts_heading(bytes: &[u8], start: usize) -> bool {
    if start + 1 >= bytes.len() {
        return false;
    }
    if bytes[start] != b'#' {
        return false;
    }
    let next = bytes[start + 1];
    next == b' ' || next == b'\t'
}

/// Returns the byte offset of the first `## ` (or `##\t`) occurrence that
/// starts a line in `content`, or `None` if none exists.
///
/// A line starts either at byte 0 or immediately after a `\n`. The `##` must
/// be followed by any whitespace character (ASCII space or tab is enough for
/// the patterns the Mac writer emits). `##text` without trailing whitespace
/// is NOT a heading.
fn find_h2_heading_start(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();

    if starts_h2(bytes, 0) {
        return Some(0);
    }

    let mut idx = 0;
    while idx + 3 < bytes.len() {
        if bytes[idx] == b'\n' && starts_h2(bytes, idx + 1) {
            return Some(idx + 1);
        }
        idx += 1;
    }

    None
}

/// Returns `true` if `bytes[start..]` begins with `## ` or `##\t`.
fn starts_h2(bytes: &[u8], start: usize) -> bool {
    if start + 2 >= bytes.len() {
        return false;
    }
    if bytes[start] != b'#' || bytes[start + 1] != b'#' {
        return false;
    }
    let next = bytes[start + 2];
    next == b' ' || next == b'\t'
}

/// Resolves an absolute user-provided target path. Rejects any `.` / `..` /
/// empty component (after the leading `/`). Canonicalises existing prefixes
/// so symlinks on the path are followed, mirroring Swift's
/// `resolvingSymlinksInPath()` behaviour.
fn resolve_absolute_target(normalized: &str) -> Result<PathBuf, TraceError> {
    debug_assert!(normalized.starts_with('/'));

    for component in normalized.split('/').skip(1) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(TraceError::PathEscapesVault(normalized.to_string()));
        }
    }

    let path = PathBuf::from(normalized);
    // Canonicalise the longest existing prefix so any symlink on the path is
    // resolved. If nothing exists yet we fall back to the literal path.
    let resolved = canonicalize_existing_prefix_of(&path).unwrap_or(path);
    Ok(resolved)
}

/// Best-effort prefix canonicalisation — identical in spirit to the helper in
/// `paths::safety`, duplicated here to avoid widening the module boundary.
fn canonicalize_existing_prefix_of(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(&current) {
            let suffix = path.strip_prefix(&current).unwrap_or(Path::new(""));
            return Some(canonical.join(suffix));
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Appends `.md` if the path string does not already end with `.md`.
/// Case-sensitive to match Swift's `hasSuffix(".md")` — `"work.MD"` becomes
/// `"work.MD.md"`, odd but intentional.
fn ensure_md_extension(path: PathBuf) -> PathBuf {
    let as_str = path.to_string_lossy();
    if as_str.ends_with(".md") {
        path
    } else {
        PathBuf::from(format!("{as_str}.md"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;
    use uuid::Uuid;

    struct FakeThreadSettings {
        vault: PathBuf,
        theme: EntryTheme,
    }

    impl FakeThreadSettings {
        fn new(vault: PathBuf, theme: EntryTheme) -> Self {
            Self { vault, theme }
        }
    }

    impl ThreadSettings for FakeThreadSettings {
        fn vault_path(&self) -> &Path {
            &self.vault
        }
        fn entry_theme(&self) -> EntryTheme {
            self.theme
        }
    }

    fn fixed_time(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    fn make_thread(name: &str, target: &str) -> ThreadConfig {
        ThreadConfig::with_id(Uuid::new_v4(), name, target, None, 0)
    }

    // ---------------------------------------------------------------------
    // Create-new-entry path
    // ---------------------------------------------------------------------

    #[test]
    fn creates_file_with_heading_on_first_save() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let thread = make_thread("Work", "threads/work.md");

        let written = writer
            .save("hello", &thread, SaveMode::CreateNewEntry, now)
            .unwrap()
            .expect("should write a file");

        let expected_path = tmp.path().join("threads").join("work.md");
        assert_eq!(written.path, expected_path);
        assert!(expected_path.exists());

        let actual = fs::read_to_string(&expected_path).unwrap();
        let expected = "# Work\n\n## 2026-04-20 12:00\n\n```\nhello\n```\n\n";
        assert_eq!(actual, expected);
        assert_eq!(written.bytes_written as usize, expected.len());
    }

    #[test]
    fn newest_entry_appears_above_older_entry() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let t1 = fixed_time(2026, 4, 20, 12, 0);
        let t2 = fixed_time(2026, 4, 20, 12, 5);
        let thread = make_thread("Work", "threads/work.md");

        let _first = writer
            .save("text1", &thread, SaveMode::CreateNewEntry, t1)
            .unwrap()
            .unwrap();
        let written = writer
            .save("text2", &thread, SaveMode::CreateNewEntry, t2)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        // Verified empirically against Swift `ThreadWriter.append` (see report):
        // the "# Work\n" heading gets a "\n" prefix injected, then entry2,
        // then the older body (with leading \n stripped) which starts with
        // "## {t1}…". Net result has exactly "\n\n" between the two `##`
        // headings (not three newlines).
        let expected = "# Work\n\n## 2026-04-20 12:05\n\n```\ntext2\n```\n\n## 2026-04-20 12:00\n\n```\ntext1\n```\n\n";
        assert_eq!(actual, expected);

        // Cross-check: newer entry's "## " appears at a lower byte offset.
        let pos_newer = actual.find("## 2026-04-20 12:05").unwrap();
        let pos_older = actual.find("## 2026-04-20 12:00").unwrap();
        assert!(pos_newer < pos_older, "newest should be above oldest");
    }

    #[test]
    fn plain_text_theme_omits_fence() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::PlainTextTimestamp);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let thread = make_thread("Work", "threads/work.md");

        let written = writer
            .save("body", &thread, SaveMode::CreateNewEntry, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        let expected = "# Work\n\n## 2026-04-20 12:00\n\nbody\n\n";
        assert_eq!(actual, expected);
        assert!(!actual.contains("```"), "plainText theme must not fence");
    }

    #[test]
    fn markdown_quote_theme_prefixes_every_line() {
        let tmp = TempDir::new().unwrap();
        let settings = FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::MarkdownQuote);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let thread = make_thread("Work", "threads/work.md");

        let written = writer
            .save("a\n\nb", &thread, SaveMode::CreateNewEntry, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&written.path).unwrap();
        let expected = "# Work\n\n## 2026-04-20 12:00\n\n> a\n>\n> b\n\n";
        assert_eq!(actual, expected);
    }

    #[test]
    fn synthesizes_heading_when_file_has_content_but_no_heading() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let thread = make_thread("Journal", "threads/journal.md");
        let file_path = writer.thread_file_path(&thread).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "loose text\n").unwrap();

        let _written = writer
            .save("hello", &thread, SaveMode::CreateNewEntry, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&file_path).unwrap();
        let expected = "# Journal\n\n## 2026-04-20 12:00\n\n```\nhello\n```\n\nloose text\n";
        assert_eq!(actual, expected);
    }

    #[test]
    fn empty_text_is_noop() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let thread = make_thread("Work", "threads/work.md");

        assert!(writer
            .save("", &thread, SaveMode::CreateNewEntry, now)
            .unwrap()
            .is_none());
        assert!(writer
            .save("   \n\t  ", &thread, SaveMode::CreateNewEntry, now)
            .unwrap()
            .is_none());

        let expected_path = tmp.path().join("threads").join("work.md");
        assert!(
            !expected_path.exists(),
            "no file should be created for blank text"
        );
    }

    // ---------------------------------------------------------------------
    // Append-to-latest path
    // ---------------------------------------------------------------------

    #[test]
    fn append_mode_places_new_entry_above_existing_h2() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 5);
        let thread = make_thread("Work", "threads/work.md");
        let file_path = writer.thread_file_path(&thread).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        // Pre-seed with one entry already in place.
        fs::write(
            &file_path,
            "# Work\n\n## 2026-04-20 11:00\n\n```\nolder\n```\n\n",
        )
        .unwrap();

        let _written = writer
            .save("newer", &thread, SaveMode::AppendToLatestEntry, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&file_path).unwrap();
        let expected = "# Work\n\n## 2026-04-20 12:05\n\n```\nnewer\n```\n\n## 2026-04-20 11:00\n\n```\nolder\n```\n\n";
        assert_eq!(actual, expected);
        assert!(
            actual.find("## 2026-04-20 12:05").unwrap()
                < actual.find("## 2026-04-20 11:00").unwrap()
        );
    }

    #[test]
    fn append_mode_falls_back_to_create_when_no_h2_exists() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let now = fixed_time(2026, 4, 20, 12, 0);
        let thread = make_thread("Work", "threads/work.md");
        let file_path = writer.thread_file_path(&thread).unwrap();
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "# Work\n\n").unwrap();

        let _written = writer
            .save("hello", &thread, SaveMode::AppendToLatestEntry, now)
            .unwrap()
            .unwrap();

        let actual = fs::read_to_string(&file_path).unwrap();
        // Fell back to create mode: appended entry right after the heading.
        let expected = "# Work\n\n## 2026-04-20 12:00\n\n```\nhello\n```\n\n";
        assert_eq!(actual, expected);
    }

    // ---------------------------------------------------------------------
    // Path resolution
    // ---------------------------------------------------------------------

    #[test]
    fn relative_target_joins_under_vault_and_adds_md() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let thread = make_thread("Work", "threads/work");
        let path = writer.thread_file_path(&thread).unwrap();
        assert_eq!(path, tmp.path().join("threads").join("work.md"));
    }

    /// Absolute-path handling — gated `#[cfg(unix)]` because Windows absolute
    /// paths start with a drive letter and the `starts_with('/')` branch
    /// never fires. Flagged in the task report so a Windows-side variant can
    /// be added later if needed.
    #[cfg(unix)]
    #[test]
    fn absolute_target_is_respected_even_outside_vault() {
        let tmp = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let absolute_target = outside
            .path()
            .join("manual.md")
            .to_string_lossy()
            .into_owned();
        let thread = make_thread("Work", &absolute_target);
        let path = writer.thread_file_path(&thread).unwrap();

        // Canonicalisation may prepend /private on macOS; accept either.
        let expected = outside.path().join("manual.md");
        let expected_canonical = std::fs::canonicalize(outside.path())
            .unwrap()
            .join("manual.md");
        assert!(
            path == expected || path == expected_canonical,
            "expected {:?} or {:?}, got {:?}",
            expected,
            expected_canonical,
            path
        );
    }

    #[test]
    fn rejects_blank_target_file() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let thread = make_thread("Work", "");
        let err = writer.thread_file_path(&thread).unwrap_err();
        assert!(
            matches!(err, TraceError::InvalidThreadConfig(_)),
            "expected InvalidThreadConfig, got {err:?}"
        );

        let thread_ws = make_thread("Work", "   \n\t");
        let err_ws = writer.thread_file_path(&thread_ws).unwrap_err();
        assert!(matches!(err_ws, TraceError::InvalidThreadConfig(_)));
    }

    #[test]
    fn rejects_traversal_in_relative_path() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let thread = make_thread("Work", "../outside.md");
        let err = writer.thread_file_path(&thread).unwrap_err();
        assert!(
            matches!(err, TraceError::PathEscapesVault(_)),
            "expected PathEscapesVault, got {err:?}"
        );
    }

    #[test]
    fn md_extension_is_appended_when_missing() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let thread = make_thread("Work", "work");
        let path = writer.thread_file_path(&thread).unwrap();
        assert!(
            path.to_string_lossy().ends_with("work.md"),
            "expected path to end with work.md, got {path:?}"
        );
    }

    #[test]
    fn md_extension_is_case_sensitive() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let thread = make_thread("Work", "work.MD");
        let path = writer.thread_file_path(&thread).unwrap();
        // Swift's `hasSuffix(".md")` is case-sensitive, so `.MD` does NOT
        // match and `.md` gets appended.
        assert!(
            path.to_string_lossy().ends_with("work.MD.md"),
            "expected path to end with work.MD.md (case-sensitive parity with Swift), got {path:?}"
        );
    }

    #[test]
    fn backslashes_are_normalized_to_forward_slashes() {
        let tmp = TempDir::new().unwrap();
        let settings =
            FakeThreadSettings::new(tmp.path().to_path_buf(), EntryTheme::CodeBlockClassic);
        let writer = ThreadWriter::new(settings);

        let thread = make_thread("Work", "subdir\\note.md");
        let path = writer.thread_file_path(&thread).unwrap();
        assert_eq!(path, tmp.path().join("subdir").join("note.md"));
    }

    // ---------------------------------------------------------------------
    // Theme parity with Daily
    // ---------------------------------------------------------------------

    #[test]
    fn thread_quote_text_matches_daily_quote_text() {
        // The shared helper in writer/mod.rs is used by both writers.
        // Spot-check several inputs to confirm byte-level parity.
        use crate::writer::markdown_quote_body_thread;
        assert_eq!(markdown_quote_body_thread("a\n\nb"), "> a\n>\n> b");
        assert_eq!(markdown_quote_body_thread("single"), "> single");
        assert_eq!(markdown_quote_body_thread(""), ">");
        assert_eq!(markdown_quote_body_thread("\n"), ">\n>");
    }
}

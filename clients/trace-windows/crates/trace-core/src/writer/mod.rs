//! Writer infrastructure shared by the daily, thread, and file writers.
//!
//! This module defines the cross-writer primitives: the [`NoteWriter`] trait,
//! the [`SaveMode`] enum shared by Daily 2.5 (future) and Thread 2.3, and a
//! handful of small helpers (`timestamp`, `markdown_quote_body_thread`) that
//! keep byte-level parity with the Swift reference. Concrete writers live in
//! sibling sub-modules.

pub mod atomic;
pub mod daily;
pub mod thread;

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::error::TraceError;
use crate::models::Entry;

pub use atomic::write_atomic;
pub use daily::{DailyNoteSettings, DailyNoteWriter};
pub use thread::{ThreadSettings, ThreadWriter};

/// Save-mode enum shared by writers that support both "create a new entry"
/// and "append to the latest entry" semantics.
///
/// Mac reference: `DailyNoteSaveMode` (reused by `ThreadWriter`). Daily's
/// `createNewEntry` path is implemented in Phase 2.2; its
/// `appendToLatestEntry` path lands in Phase 2.5. Thread uses both modes as
/// of Phase 2.3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SaveMode {
    /// Insert a fresh entry with its own `## {timestamp}` heading.
    #[default]
    CreateNewEntry,
    /// Append text to the most recent entry (newest on top for threads;
    /// under the current section for daily — see Phase 2.5).
    AppendToLatestEntry,
}

/// Formats `now` as `yyyy-MM-dd HH:mm` in UTC. Locale-independent so this is
/// byte-identical to Swift's `en_US_POSIX` formatter.
///
/// Shared between Daily and Thread writers to avoid drift.
pub(crate) fn timestamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%d %H:%M").to_string()
}

/// Prefixes every line of `text` with `>` (empty lines become a bare `>`).
///
/// Mirrors Swift's `quotedText(from:)` helper used by both `DailyNoteWriter`
/// and `ThreadWriter`. Exposed `pub(crate)` so both writers produce
/// byte-identical quoted bodies without duplicating the logic.
pub(crate) fn markdown_quote_body_thread(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Result of a successful note write. Callers use this to validate that the
/// expected path was touched and to assert byte-length parity against the
/// Mac implementation.
#[must_use = "WrittenNote records the path and byte count of the write; drop it only if you are certain you need neither"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenNote {
    /// Absolute path of the file that was written.
    pub path: PathBuf,
    /// Number of bytes written to `path` (i.e. the total file size after the
    /// write completed, not merely the size of the new entry).
    pub bytes_written: u64,
}

/// Common contract shared by all writers. Later sub-tasks add concrete
/// implementations for daily notes, threads, and free-form files.
pub trait NoteWriter {
    fn write(&self, entry: &Entry) -> Result<WrittenNote, TraceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    use crate::models::WriteMode;

    struct StubWriter;

    impl NoteWriter for StubWriter {
        fn write(&self, entry: &Entry) -> Result<WrittenNote, TraceError> {
            Ok(WrittenNote {
                path: PathBuf::from("/tmp/trace/test.md"),
                bytes_written: entry.content.len() as u64,
            })
        }
    }

    #[test]
    fn trait_can_be_implemented_and_returns_written_note() {
        let entry = Entry::new(
            "body",
            Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap(),
            WriteMode::Dimension,
        );
        let writer = StubWriter;
        let result = writer.write(&entry).unwrap();
        assert_eq!(result.path, PathBuf::from("/tmp/trace/test.md"));
        assert_eq!(result.bytes_written, 4);
    }

    #[test]
    fn written_note_equality_works() {
        let a = WrittenNote {
            path: PathBuf::from("/a.md"),
            bytes_written: 10,
        };
        let b = WrittenNote {
            path: PathBuf::from("/a.md"),
            bytes_written: 10,
        };
        assert_eq!(a, b);
    }
}

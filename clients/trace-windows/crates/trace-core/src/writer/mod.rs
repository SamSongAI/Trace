//! Writer infrastructure shared by the daily, thread, and file writers.
//!
//! This module intentionally stops at the trait and atomic-write primitive.
//! Concrete writers (daily notes, threads, free-form files, clipboard
//! images) live in sibling sub-modules added by subsequent sub-tasks.

pub mod atomic;
pub mod daily;

use std::path::PathBuf;

use crate::error::TraceError;
use crate::models::Entry;

pub use atomic::write_atomic;
pub use daily::{DailyNoteSettings, DailyNoteWriter};

/// Result of a successful note write. Callers use this to validate that the
/// expected path was touched and to assert byte-length parity against the
/// Mac implementation.
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

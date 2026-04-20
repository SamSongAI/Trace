//! Atomic file writes.
//!
//! Provides an atomic-replace write primitive analogous to Swift's
//! `String.write(to:atomically:true)`. Built on `std::fs::rename`, which on
//! Windows (Rust ≥1.70) uses `MOVEFILE_REPLACE_EXISTING` semantics internally
//! — a same-directory temp→final replace is atomic on NTFS. If a future bug
//! surfaces on network drives or legacy FAT32, the platform-specific fix
//! belongs in `trace-platform`, not here.
//!
//! Mac Trace uses `Data.write(to:options:.atomic)`, which writes to a
//! temporary file in the same directory and then `rename(2)`s it over the
//! target. We mirror the same semantics:
//!
//! 1. Write the bytes to `<target>.trace-tmp-<pid>-<nanos>` in the same
//!    directory as the final file. Same-directory placement is important —
//!    `rename` is only guaranteed to be atomic on the same filesystem, and
//!    NTFS does not guarantee atomic cross-volume renames.
//! 2. `fsync` the temp file, then rename onto the final path.
//! 3. If anything fails before the rename, remove the temp file.
//!
//! A hand-rolled solution is only ~40 lines and has no dependency
//! implications, so we do not pull in the `tempfile` crate here.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::TraceError;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `contents` to `path` atomically. The parent directory is **not**
/// created automatically — the caller is expected to ensure it exists
/// (mirrors the Mac implementation, which uses a separate
/// `ensureDailyDirectoryExists` step before writing).
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), TraceError> {
    if path.parent().is_none() {
        return Err(TraceError::AtomicWriteFailed(format!(
            "{} has no parent",
            path.display()
        )));
    }

    let temp_path = temp_sibling_path(path);

    let guard = TempFileGuard::new(temp_path.clone());

    {
        let mut file: File = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| {
                TraceError::AtomicWriteFailed(format!(
                    "open temp {} failed: {}",
                    temp_path.display(),
                    e
                ))
            })?;

        file.write_all(contents).map_err(|e| {
            TraceError::AtomicWriteFailed(format!(
                "write temp {} failed: {}",
                temp_path.display(),
                e
            ))
        })?;

        file.sync_all().map_err(|e| {
            TraceError::AtomicWriteFailed(format!(
                "fsync temp {} failed: {}",
                temp_path.display(),
                e
            ))
        })?;
    }

    fs::rename(&temp_path, path).map_err(|e| {
        TraceError::AtomicWriteFailed(format!(
            "rename {} -> {} failed: {}",
            temp_path.display(),
            path.display(),
            e
        ))
    })?;

    // Rename succeeded; the temp path no longer exists, so disarm the guard.
    guard.disarm();
    Ok(())
}

fn temp_sibling_path(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let filename = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "trace".to_string());
    let pid = std::process::id();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(".{filename}.trace-tmp-{pid}-{nanos}-{counter}"))
}

/// RAII helper that deletes the temp file if the caller does not call
/// `disarm()`. Prevents leaked temp files when a write fails mid-flight.
#[must_use = "TempFileGuard must be bound to a variable; dropping it immediately will remove the temp file before the atomic write completes"]
struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "trace-atomic-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_new_file() {
        let dir = unique_dir("new");
        let target = dir.join("note.md");
        write_atomic(&target, b"hello").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"hello");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn overwrites_existing_file_atomically() {
        let dir = unique_dir("overwrite");
        let target = dir.join("note.md");
        std::fs::write(&target, b"old contents").unwrap();
        write_atomic(&target, b"new contents").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new contents");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leaves_no_temp_files_after_success() {
        let dir = unique_dir("cleanup");
        let target = dir.join("note.md");
        write_atomic(&target, b"body").unwrap();
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec!["note.md".to_string()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fails_cleanly_when_parent_directory_missing() {
        let dir = unique_dir("missing-parent");
        let target = dir.join("nested/does/not/exist.md");
        let err = write_atomic(&target, b"body").unwrap_err();
        assert!(matches!(err, TraceError::AtomicWriteFailed(_)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn roundtrip_contents_are_byte_identical() {
        let dir = unique_dir("bytes");
        let target = dir.join("note.md");
        let bytes: &[u8] = b"# \xe6\x83\xb3\xe6\xb3\x95\n\n> one\n";
        write_atomic(&target, bytes).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

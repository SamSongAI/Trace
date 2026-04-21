//! Filesystem-backed vault path validation.
//!
//! Mac ports `Sources/Trace/Services/AppSettings.swift`'s
//! `vaultPathValidationIssue` helper (blank / !exists / !directory /
//! !writable). `trace-core` only exposes the blank probe because the
//! remaining three checks touch the filesystem; this module layers the
//! platform-level probes on top so higher layers can call a single
//! `validate_vault_path(path)` and get the full classification.
//!
//! The helper is cross-platform: it relies on `std::fs::metadata` plus a
//! write-probe that creates a uniquely-named temp file inside the target
//! directory. On Windows the probe maps to `CreateFileW`; on macOS/Linux
//! it maps to `open(O_CREAT | O_EXCL)`. In both cases a failure to create
//! the probe is reported as [`VaultPathValidationIssue::NotWritable`].
//!
//! Writability of a read-only directory behaves differently on Windows
//! (ACLs) vs Unix (mode bits). The unit tests only exercise the Empty /
//! DoesNotExist / NotDirectory / writable-OK paths because simulating
//! "not writable" portably inside a CI sandbox is fiddly — a TODO in the
//! test module notes the gap.

use std::fs;
use std::io::Write;
use std::path::Path;

use trace_core::VaultPathValidationIssue;
use uuid::Uuid;

/// Classifies `path` against the four [`VaultPathValidationIssue`]
/// variants, returning [`None`] when the path is a usable writable
/// directory. Mirrors Mac `AppSettings.vaultPathValidationIssue`.
///
/// Order of checks:
/// 1. [`VaultPathValidationIssue::is_blank`] — trim-empty ⇒ `Empty`.
/// 2. `metadata(path)` — error ⇒ `DoesNotExist`.
/// 3. `metadata.is_dir()` false ⇒ `NotDirectory`.
/// 4. Write-probe: create + write + delete a uniquely-named file inside
///    `path`. Any step failing ⇒ `NotWritable`.
pub fn validate_vault_path(path: &str) -> Option<VaultPathValidationIssue> {
    if let Some(issue) = VaultPathValidationIssue::is_blank(path) {
        return Some(issue);
    }

    let path_ref = Path::new(path);
    let metadata = match fs::metadata(path_ref) {
        Ok(meta) => meta,
        Err(_) => return Some(VaultPathValidationIssue::DoesNotExist),
    };

    if !metadata.is_dir() {
        return Some(VaultPathValidationIssue::NotDirectory);
    }

    if !is_writable_directory(path_ref) {
        return Some(VaultPathValidationIssue::NotWritable);
    }

    None
}

/// Probes whether `dir` accepts a freshly-created file. Returns false on
/// any failure along the create/write/delete path. The probe file name is
/// seeded with a `v4` UUID so concurrent probes on the same directory
/// cannot collide.
///
/// Deletion failure is treated as success because Windows sometimes
/// holds a brief handle on the just-closed file; the probe already
/// proved writability, and the leftover file is best-effort cleaned up
/// by the caller / OS temp reaper. On a successful delete the function
/// also reports success.
fn is_writable_directory(dir: &Path) -> bool {
    let probe_name = format!(".trace_write_probe_{}.tmp", Uuid::new_v4());
    let probe_path = dir.join(probe_name);

    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
    {
        Ok(f) => f,
        Err(_) => return false,
    };

    // Write a single byte so the probe exercises the underlying
    // filesystem, not just the directory entry. Volumes mounted as
    // read-only occasionally return success on `create_new` and then
    // fail on the first write — this guards against that.
    if file.write_all(b"0").is_err() {
        drop(file);
        let _ = fs::remove_file(&probe_path);
        return false;
    }
    drop(file);

    // Clean up; ignore errors (see doc comment).
    let _ = fs::remove_file(&probe_path);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn blank_path_returns_empty() {
        assert_eq!(
            validate_vault_path(""),
            Some(VaultPathValidationIssue::Empty)
        );
    }

    #[test]
    fn whitespace_only_path_returns_empty() {
        assert_eq!(
            validate_vault_path("   "),
            Some(VaultPathValidationIssue::Empty)
        );
        assert_eq!(
            validate_vault_path("\t\n"),
            Some(VaultPathValidationIssue::Empty)
        );
    }

    #[test]
    fn nonexistent_path_returns_does_not_exist() {
        // A deterministic missing path: under a fresh TempDir so it
        // cannot collide with a real directory on the CI host.
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("no-such-subdir");
        let as_str = missing.to_string_lossy().into_owned();
        assert_eq!(
            validate_vault_path(&as_str),
            Some(VaultPathValidationIssue::DoesNotExist)
        );
    }

    #[test]
    fn file_path_returns_not_directory() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("not-a-dir.txt");
        fs::write(&file_path, b"hello").unwrap();
        let as_str = file_path.to_string_lossy().into_owned();
        assert_eq!(
            validate_vault_path(&as_str),
            Some(VaultPathValidationIssue::NotDirectory)
        );
    }

    #[test]
    fn writable_directory_returns_none() {
        let dir = TempDir::new().unwrap();
        let as_str = dir.path().to_string_lossy().into_owned();
        assert_eq!(validate_vault_path(&as_str), None);
    }

    #[test]
    fn writable_directory_probe_leaves_no_probe_file_behind() {
        // After a successful probe the directory must be empty again.
        // A regression that leaked probe files would fill the user's
        // vault with `.trace_write_probe_*` cruft.
        let dir = TempDir::new().unwrap();
        let as_str = dir.path().to_string_lossy().into_owned();
        assert_eq!(validate_vault_path(&as_str), None);
        let leftover: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            leftover.is_empty(),
            "probe left files behind: {leftover:?}"
        );
    }

    // TODO: cross-platform `NotWritable` coverage is non-trivial — on
    // Unix `chmod 0o000` cannot be made durable inside TempDir without
    // root-level interference, and on Windows ACL-denying the temp dir
    // requires elevated privileges. Parked as a gap; the variant itself
    // is exercised in higher-layer settings-window tests.
}

//! Path-traversal safety for user-provided relative paths.
//!
//! Mac Trace relies on `URL.resolvingSymlinksInPath()` plus a manual prefix
//! check (see `ThreadWriter.threadFileURL`). We reproduce the same contract in
//! Rust: given a vault root and a user-supplied relative fragment, produce an
//! absolute path that is guaranteed to live under the vault. Any attempt to
//! escape — via `..`, absolute paths, Windows drive letters, UNC prefixes, or
//! a symlink that resolves outside — returns `TraceError::PathEscapesVault`.

use std::path::{Component, Path, PathBuf};

use crate::error::TraceError;

/// Resolves `relative` against `vault` and returns the absolute path if (and
/// only if) the result lies inside `vault`. The relative fragment may use
/// either `/` or `\\` as a separator; empty or whitespace-only fragments are
/// rejected.
///
/// The vault itself is **not** required to exist at call time, so this
/// function is safe to use from tests and from pre-flight checks. When a
/// resolved component happens to exist and is a symlink, the symlink is
/// canonicalized and must still resolve inside the vault.
pub fn resolve_within_vault(vault: &Path, relative: &str) -> Result<PathBuf, TraceError> {
    let trimmed = relative.trim();
    if trimmed.is_empty() {
        return Err(TraceError::PathEscapesVault(relative.to_string()));
    }

    let normalized_separators = trimmed.replace('\\', "/");

    if is_absolute_like(&normalized_separators) {
        return Err(TraceError::PathEscapesVault(relative.to_string()));
    }

    let mut segments: Vec<&str> = Vec::new();
    for segment in normalized_separators.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(TraceError::PathEscapesVault(relative.to_string()));
        }
        segments.push(segment);
    }

    if segments.is_empty() {
        return Err(TraceError::PathEscapesVault(relative.to_string()));
    }

    let mut joined = vault.to_path_buf();
    for segment in &segments {
        joined.push(segment);
    }

    // If any existing prefix of the joined path is a symlink we must make sure
    // the canonical target still lives inside the canonical vault.
    let canonical_vault = canonicalize_existing_prefix(vault);
    if let Some(canonical_target) = canonicalize_existing_prefix_of(&joined) {
        if !canonical_target.starts_with(&canonical_vault) {
            return Err(TraceError::PathEscapesVault(relative.to_string()));
        }
    }

    // Belt-and-suspenders: after resolving symlinks the joined path must still
    // start with the vault (string-level check guards the fully-virtual case
    // where nothing exists yet).
    let logical_vault = normalize_components(vault);
    let logical_joined = normalize_components(&joined);
    if !logical_joined.starts_with(&logical_vault) {
        return Err(TraceError::PathEscapesVault(relative.to_string()));
    }

    Ok(joined)
}

fn is_absolute_like(normalized: &str) -> bool {
    if normalized.starts_with('/') {
        return true;
    }
    // Windows drive letter: `C:` or `C:/...`
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return true;
    }
    false
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Canonicalizes the longest existing prefix of `path` so symlink escapes are
/// detected even when the leaf file has not yet been created.
fn canonicalize_existing_prefix(path: &Path) -> PathBuf {
    canonicalize_existing_prefix_of(path).unwrap_or_else(|| normalize_components(path))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> PathBuf {
        PathBuf::from("/tmp/trace-vault")
    }

    #[test]
    fn resolves_simple_relative_path() {
        let resolved = resolve_within_vault(&vault(), "Daily/2026.md").unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/trace-vault/Daily/2026.md"));
    }

    #[test]
    fn normalizes_backslash_separators() {
        let resolved = resolve_within_vault(&vault(), "Daily\\note.md").unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/trace-vault/Daily/note.md"));
    }

    #[test]
    fn collapses_current_directory_segments() {
        let resolved = resolve_within_vault(&vault(), "./Daily/./note.md").unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/trace-vault/Daily/note.md"));
    }

    #[test]
    fn rejects_parent_directory_traversal() {
        let err = resolve_within_vault(&vault(), "../etc/passwd").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_embedded_parent_directory_segment() {
        let err = resolve_within_vault(&vault(), "Daily/../../etc/passwd").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_unix_absolute_path() {
        let err = resolve_within_vault(&vault(), "/etc/passwd").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_windows_drive_letter() {
        let err = resolve_within_vault(&vault(), "C:/Windows/System32").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_windows_drive_letter_with_backslash() {
        let err = resolve_within_vault(&vault(), "D:\\secrets.md").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_empty_input() {
        let err = resolve_within_vault(&vault(), "").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_whitespace_only_input() {
        let err = resolve_within_vault(&vault(), "   ").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn rejects_slash_only_input() {
        // This is treated as absolute.
        let err = resolve_within_vault(&vault(), "/").unwrap_err();
        assert!(matches!(err, TraceError::PathEscapesVault(_)));
    }

    #[test]
    fn allows_unicode_folder_names() {
        let resolved = resolve_within_vault(&vault(), "想法/note.md").unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/trace-vault/想法/note.md"));
    }

    #[test]
    fn detects_symlink_escape_when_vault_exists() {
        let tmp = std::env::temp_dir().join(format!("trace-vault-safety-{}", std::process::id()));
        let outside =
            std::env::temp_dir().join(format!("trace-outside-safety-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, tmp.join("escape")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, tmp.join("escape")).unwrap();

        let result = resolve_within_vault(&tmp, "escape/secret.md");
        assert!(matches!(result, Err(TraceError::PathEscapesVault(_))));

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&outside);
    }
}

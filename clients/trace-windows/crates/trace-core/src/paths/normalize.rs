//! Cross-platform filename normalization.
//!
//! Vaults are commonly synced between macOS and Windows (iCloud, Dropbox,
//! OneDrive), so we sanitize to the stricter Windows rule set even though the
//! Rust client only runs on Windows today. This mirrors Mac's Trace behaviour
//! (see `DailyNoteWriter.normalizedFileNameSegment`) plus the additional
//! Windows-specific constraints: reserved device names, trailing dots, and
//! trailing spaces.

use crate::error::TraceError;

/// Characters that are illegal in Windows file names.
const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Reserved Windows device names (case-insensitive match against the file
/// stem). Files with these names fail to open on Windows regardless of
/// extension.
const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Normalizes a user-provided filename segment into one that is safe on both
/// Windows and macOS. Returns `TraceError::InvalidFilename` if the input is
/// empty or consists entirely of characters that would be stripped.
///
/// Rules (aligned with Mac Trace where possible):
/// - Replace illegal characters (`<>:"/\\|?*`) and ASCII control bytes with `-`
/// - Collapse runs of whitespace into a single `-`
/// - Collapse runs of `-` into a single `-`
/// - Trim leading/trailing `-`, `.`, and whitespace
/// - If the resulting stem matches a reserved device name, suffix `-file`
pub fn sanitize_filename(raw: &str) -> Result<String, TraceError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TraceError::InvalidFilename(raw.to_string()));
    }

    let replaced: String = trimmed
        .chars()
        .map(|c| {
            if ILLEGAL_CHARS.contains(&c) || c.is_control() {
                '-'
            } else if c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .collect();

    let collapsed = collapse_runs(&replaced);
    let trimmed_edges: String = collapsed
        .trim_matches(|c: char| c == '-' || c == '.' || c.is_whitespace())
        .to_string();

    if trimmed_edges.is_empty() {
        return Err(TraceError::InvalidFilename(raw.to_string()));
    }

    if is_reserved_device_name(&trimmed_edges) {
        return Ok(format!("{}-file", trimmed_edges));
    }

    Ok(trimmed_edges)
}

/// Normalizes a filename stem while preserving (and separately sanitizing) an
/// optional extension. Useful when callers want to keep `.md`, `.png`, etc.
/// even if the user entered the full name as part of the stem.
pub fn sanitize_filename_preserve_extension(
    stem: &str,
    extension: Option<&str>,
) -> Result<String, TraceError> {
    let clean_stem = sanitize_filename(stem)?;
    match extension {
        Some(ext) => {
            let clean_ext = sanitize_filename(ext)?;
            Ok(format!("{}.{}", clean_stem, clean_ext))
        }
        None => Ok(clean_stem),
    }
}

fn collapse_runs(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut last_was_dash = false;
    let mut last_was_space = false;

    for c in input.chars() {
        if c == '-' {
            if !last_was_dash {
                result.push('-');
            }
            last_was_dash = true;
            last_was_space = false;
        } else if c.is_whitespace() {
            if !last_was_space && !last_was_dash {
                result.push('-');
            }
            last_was_space = true;
            last_was_dash = true;
        } else {
            result.push(c);
            last_was_dash = false;
            last_was_space = false;
        }
    }

    result
}

fn is_reserved_device_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    RESERVED_DEVICE_NAMES.contains(&stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_all_illegal_characters() {
        let result = sanitize_filename("a<b>c:d\"e/f\\g|h?i*j").unwrap();
        assert_eq!(result, "a-b-c-d-e-f-g-h-i-j");
    }

    #[test]
    fn sanitize_collapses_whitespace_runs() {
        let result = sanitize_filename("hello   world\tfoo").unwrap();
        assert_eq!(result, "hello-world-foo");
    }

    #[test]
    fn sanitize_trims_leading_and_trailing_dots_and_dashes() {
        let result = sanitize_filename("...my note---").unwrap();
        assert_eq!(result, "my-note");
    }

    #[test]
    fn sanitize_rejects_empty_input() {
        assert!(matches!(
            sanitize_filename(""),
            Err(TraceError::InvalidFilename(_))
        ));
    }

    #[test]
    fn sanitize_rejects_whitespace_only_input() {
        assert!(matches!(
            sanitize_filename("   \t\n"),
            Err(TraceError::InvalidFilename(_))
        ));
    }

    #[test]
    fn sanitize_rejects_only_illegal_characters() {
        assert!(matches!(
            sanitize_filename("///"),
            Err(TraceError::InvalidFilename(_))
        ));
    }

    #[test]
    fn sanitize_suffixes_reserved_device_names() {
        assert_eq!(sanitize_filename("CON").unwrap(), "CON-file");
        assert_eq!(sanitize_filename("nul").unwrap(), "nul-file");
        assert_eq!(sanitize_filename("COM1").unwrap(), "COM1-file");
    }

    #[test]
    fn sanitize_does_not_touch_non_reserved_names_starting_with_device_prefix() {
        assert_eq!(sanitize_filename("CONfidence").unwrap(), "CONfidence");
    }

    #[test]
    fn sanitize_preserves_unicode_characters() {
        assert_eq!(sanitize_filename("想法").unwrap(), "想法");
        assert_eq!(sanitize_filename("読書ノート").unwrap(), "読書ノート");
    }

    #[test]
    fn sanitize_replaces_newlines_with_space_collapsed_to_dash() {
        let result = sanitize_filename("line1\nline2").unwrap();
        assert_eq!(result, "line1-line2");
    }

    #[test]
    fn sanitize_strips_trailing_dots_windows_requirement() {
        let result = sanitize_filename("name.").unwrap();
        assert_eq!(result, "name");
    }

    #[test]
    fn sanitize_strips_trailing_spaces_windows_requirement() {
        let result = sanitize_filename("name ").unwrap();
        assert_eq!(result, "name");
    }

    #[test]
    fn preserve_extension_combines_stem_and_extension() {
        let result = sanitize_filename_preserve_extension("my: note", Some("md")).unwrap();
        assert_eq!(result, "my-note.md");
    }

    #[test]
    fn preserve_extension_works_without_extension() {
        let result = sanitize_filename_preserve_extension("note", None).unwrap();
        assert_eq!(result, "note");
    }
}

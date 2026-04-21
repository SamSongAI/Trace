use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadConfig {
    pub id: Uuid,
    pub name: String,
    pub target_file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub order: i32,
}

impl ThreadConfig {
    /// Minimum number of thread configs that can exist at once. Matches Mac
    /// `AppSettings.canRemoveThread`'s `threadConfigs.count > 1` gate: users
    /// cannot delete the last remaining thread because thread mode requires at
    /// least one destination.
    pub const MINIMUM_COUNT: usize = 1;
    /// Maximum number of thread configs that can exist at once. Matches Mac
    /// `AppSettings.canAddThread`'s `threadConfigs.count < 9` gate: the thread
    /// chip row would overflow past this cap on both ports.
    pub const MAXIMUM_COUNT: usize = 9;

    pub fn new(
        name: impl Into<String>,
        target_file: impl Into<String>,
        icon: Option<String>,
        order: i32,
    ) -> Self {
        Self::with_id(Uuid::new_v4(), name, target_file, icon, order)
    }

    pub fn with_id(
        id: Uuid,
        name: impl Into<String>,
        target_file: impl Into<String>,
        icon: Option<String>,
        order: i32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            target_file: target_file.into(),
            icon,
            order,
        }
    }
}

/// Splits a `target_file` string into its `(folder, filename)` components.
///
/// Mirrors Mac `ThreadConfigRow.parseTargetFile` in
/// `Sources/Trace/UI/Settings/ThreadConfigRow.swift`:
///
/// * Backslashes are normalized to forward slashes before splitting so paths
///   copy-pasted from a Windows file manager still yield a sensible
///   `(folder, filename)` tuple.
/// * Absolute paths (leading `/`) keep their leading slash in the folder
///   component, matching the Mac reference's handling.
/// * Inputs without a slash return `("", input)` — the whole string is treated
///   as the filename.
///
/// Kept as a free function (not a `ThreadConfig` method) because both the UI
/// layer's per-keystroke recompute and future persistence normalizers need a
/// pure entry point that does not require an existing `ThreadConfig` instance.
pub fn split_target_file(target_file: &str) -> (String, String) {
    // Normalize backslashes up front so the `rfind` below applies the same
    // contract across separator styles.
    let normalized = target_file.replace('\\', "/");
    match normalized.rfind('/') {
        Some(idx) => {
            // `idx` is a byte index on a string that only contains ASCII `/`
            // as the splitter; slicing on either side is safe for every
            // Unicode input.
            let folder = normalized[..idx].to_string();
            let filename = normalized[idx + 1..].to_string();
            (folder, filename)
        }
        None => (String::new(), normalized),
    }
}

/// Joins a `(folder, filename)` pair back into a `target_file` string.
///
/// Mirrors Mac `ThreadConfigRow.buildTargetFile`:
///
/// * Backslashes in both inputs are normalized to forward slashes so the
///   joined output uses a single separator convention.
/// * An empty folder (or a folder that is empty after normalization) returns
///   just the filename, which preserves the Mac reference's "relative paths
///   with no folder render as bare filenames" shape.
/// * A filename that is empty after normalization returns just the folder,
///   which gives the UI a sensible intermediate state while the user is
///   retyping the filename without dropping the folder component.
///
/// Unlike the Mac reference this helper does **not** trim whitespace on
/// either input — the shadow fields in `SettingsApp` intentionally carry
/// the raw keystroke state (sub-task 8 will trim on write-back), so trimming
/// here would throw away in-progress edits.
pub fn join_folder_and_filename(folder: &str, filename: &str) -> String {
    let folder_norm = folder.replace('\\', "/");
    let filename_norm = filename.replace('\\', "/");

    if folder_norm.is_empty() {
        return filename_norm;
    }
    if filename_norm.is_empty() {
        return folder_norm;
    }
    format!("{folder_norm}/{filename_norm}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_sets_fields_and_generates_id() {
        let config = ThreadConfig::new("想法", "想法.md", Some("lightbulb".into()), 0);
        assert_eq!(config.name, "想法");
        assert_eq!(config.target_file, "想法.md");
        assert_eq!(config.icon.as_deref(), Some("lightbulb"));
        assert_eq!(config.order, 0);
        assert!(!config.id.is_nil());
    }

    #[test]
    fn fresh_configs_have_distinct_ids() {
        let a = ThreadConfig::new("A", "A.md", None, 0);
        let b = ThreadConfig::new("B", "B.md", None, 1);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn serialize_uses_camel_case_keys() {
        let id = Uuid::parse_str("3f2504e0-4f89-41d3-9a0c-0305e82c3301").unwrap();
        let config = ThreadConfig::with_id(id, "想法", "想法.md", Some("lightbulb".into()), 0);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"targetFile\":\"想法.md\""));
        assert!(json.contains("\"icon\":\"lightbulb\""));
        assert!(json.contains("\"order\":0"));
        assert!(json.contains("\"id\":\"3f2504e0-4f89-41d3-9a0c-0305e82c3301\""));
    }

    #[test]
    fn round_trip_through_json_preserves_all_fields() {
        let id = Uuid::new_v4();
        let config = ThreadConfig::with_id(id, "读书笔记", "读书笔记.md", Some("book".into()), 2);
        let json = serde_json::to_string(&config).unwrap();
        let decoded: ThreadConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn icon_is_optional_and_omitted_when_none() {
        let id = Uuid::new_v4();
        let config = ThreadConfig::with_id(id, "t", "t.md", None, 0);
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("\"icon\""));
        let decoded: ThreadConfig = serde_json::from_str(&json).unwrap();
        assert!(decoded.icon.is_none());
    }

    #[test]
    fn count_bounds_match_mac_reference() {
        // Mac `AppSettings.canAddThread` / `canRemoveThread` bracket the
        // thread-config list at [1, 9]. Lock the constants here so a drift
        // against the Mac reference is caught at test time.
        assert_eq!(ThreadConfig::MINIMUM_COUNT, 1);
        assert_eq!(ThreadConfig::MAXIMUM_COUNT, 9);
    }

    // --- split_target_file ----------------------------------------------

    #[test]
    fn split_target_file_handles_nested_relative_path() {
        let (folder, filename) = split_target_file("foo/bar/notes.md");
        assert_eq!(folder, "foo/bar");
        assert_eq!(filename, "notes.md");
    }

    #[test]
    fn split_target_file_handles_bare_filename() {
        let (folder, filename) = split_target_file("notes.md");
        assert_eq!(folder, "");
        assert_eq!(filename, "notes.md");
    }

    #[test]
    fn split_target_file_preserves_absolute_path_prefix() {
        let (folder, filename) = split_target_file("/Users/x/notes.md");
        assert_eq!(folder, "/Users/x");
        assert_eq!(filename, "notes.md");
    }

    #[test]
    fn split_target_file_handles_empty_string() {
        let (folder, filename) = split_target_file("");
        assert_eq!(folder, "");
        assert_eq!(filename, "");
    }

    #[test]
    fn split_target_file_normalizes_backslashes() {
        // Windows-style path pasted from Explorer: every `\` becomes `/`
        // before the split so downstream callers only ever see one separator
        // convention.
        let (folder, filename) = split_target_file("foo\\bar\\notes.md");
        assert_eq!(folder, "foo/bar");
        assert_eq!(filename, "notes.md");
    }

    #[test]
    fn split_target_file_handles_trailing_slash_as_empty_filename() {
        // A path that ends at a folder separator has no filename component.
        // Callers treat this as "folder with no filename yet" and supply a
        // default on write-back.
        let (folder, filename) = split_target_file("foo/bar/");
        assert_eq!(folder, "foo/bar");
        assert_eq!(filename, "");
    }

    // --- join_folder_and_filename --------------------------------------

    #[test]
    fn join_folder_and_filename_builds_relative_path() {
        assert_eq!(
            join_folder_and_filename("foo/bar", "notes.md"),
            "foo/bar/notes.md"
        );
    }

    #[test]
    fn join_folder_and_filename_returns_filename_only_for_empty_folder() {
        assert_eq!(join_folder_and_filename("", "notes.md"), "notes.md");
    }

    #[test]
    fn join_folder_and_filename_preserves_absolute_folder() {
        assert_eq!(
            join_folder_and_filename("/Users/x", "notes.md"),
            "/Users/x/notes.md"
        );
    }

    #[test]
    fn join_folder_and_filename_returns_folder_only_for_empty_filename() {
        // The UI renders the in-progress folder when the user has cleared
        // the filename. Mac's `buildTargetFile` would emit `"foo/"` after
        // trimming; the Windows port avoids the trailing slash so a later
        // `split` round-trip stays idempotent.
        assert_eq!(join_folder_and_filename("foo", ""), "foo");
    }

    #[test]
    fn join_folder_and_filename_returns_empty_for_both_empty() {
        assert_eq!(join_folder_and_filename("", ""), "");
    }

    #[test]
    fn join_folder_and_filename_normalizes_backslashes() {
        assert_eq!(
            join_folder_and_filename("foo\\bar", "notes.md"),
            "foo/bar/notes.md"
        );
        assert_eq!(
            join_folder_and_filename("foo", "sub\\notes.md"),
            "foo/sub/notes.md"
        );
    }

    #[test]
    fn join_then_split_is_idempotent_for_typical_inputs() {
        // Round-trip property: for inputs with no backslashes and non-empty
        // components, split(join(folder, filename)) == (folder, filename).
        let cases = [
            ("foo/bar", "notes.md"),
            ("/Users/x", "notes.md"),
            ("", "notes.md"),
        ];
        for (folder, filename) in cases {
            let joined = join_folder_and_filename(folder, filename);
            let (rebuilt_folder, rebuilt_filename) = split_target_file(&joined);
            assert_eq!(rebuilt_folder, folder);
            assert_eq!(rebuilt_filename, filename);
        }
    }
}

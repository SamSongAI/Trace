//! Clipboard-image interface and path planning.
//!
//! Phase 2.6 — *interface definition only*. This module provides:
//!
//! - The [`ClipboardImageWriterSettings`] trait (vault path + daily folder
//!   name).
//! - The [`ClipboardImageWriter`] struct and its constructor.
//! - [`ImageWritePlan`] — a pure-computation result describing where a PNG
//!   *would* be written and what Markdown link would be emitted.
//! - The [`ClipboardImageWriter::plan`] method, which performs all the
//!   path-building, filename-formatting, and vault-validation steps
//!   without any filesystem or image I/O.
//!
//! Phase 13 added [`ClipboardImageWriter::write_png`], which builds the
//! plan, creates the target directory, and writes the caller-provided PNG
//! bytes atomically. The Windows clipboard reading and PNG encoding live
//! in `trace-platform`; by the time bytes reach this module they are
//! already PNG-encoded.
//!
//! # Byte-level parity with Mac reference
//!
//! Mirrors `Sources/Trace/Services/ClipboardImageWriter.swift`. The
//! filename format is `trace-yyyyMMdd-HHmmss-SSS.png`, the date folder is
//! `yyyy-MM-dd`, and the Markdown link is `![image](assets/{date}/{file})`
//! — every byte of these strings is asserted in the tests below.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::error::TraceError;
use crate::writer::validated_vault_path;

/// Settings required to place a clipboard PNG under the daily-note vault.
///
/// Mirrors the `vaultPath` + `dailyFolderName` fields of Swift's
/// `ClipboardImageWritingSettingsProviding`. Whitespace-only `vault_path`
/// is rejected with [`TraceError::InvalidVaultPath`] by
/// [`ClipboardImageWriter::plan`].
pub trait ClipboardImageWriterSettings {
    /// Absolute filesystem path to the daily-note vault root.
    fn vault_path(&self) -> &Path;

    /// Name of the daily-note sub-folder inside the vault (e.g. `"daily"`).
    /// Images land under `{vault_path}/{daily_folder_name}/assets/{date}/`.
    fn daily_folder_name(&self) -> &str;
}

// Blanket forwarding for `Arc<T>` — parity with the other writer
// settings traits so hosts can share a single settings snapshot through
// an `Arc` without cloning.
impl<T: ClipboardImageWriterSettings + ?Sized> ClipboardImageWriterSettings for Arc<T> {
    fn vault_path(&self) -> &Path {
        (**self).vault_path()
    }

    fn daily_folder_name(&self) -> &str {
        (**self).daily_folder_name()
    }
}

/// Pure-computation result of [`ClipboardImageWriter::plan`]. Phase 13
/// consumes this to drive the actual PNG write; tests consume it to pin
/// byte-level parity with Swift without touching the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageWritePlan {
    /// Absolute filesystem path where the PNG should be written.
    pub target_path: PathBuf,
    /// Vault-relative forward-slash path, suitable for embedding in the
    /// Markdown link (e.g. `"assets/2026-04-20/trace-20260420-120500-123.png"`).
    pub relative_path: String,
    /// The full Markdown image link. Ready to be inserted into a daily
    /// note, thread, or inbox file.
    pub markdown_link: String,
}

/// Clipboard image writer. The struct carries only the settings; `now`
/// is injected per call so tests can pin the clock without a time source.
///
/// Phase 2.6 provides [`Self::plan`] (pure computation); Phase 13 adds
/// [`Self::write_png`] (disk-backed). See the module-level docs.
pub struct ClipboardImageWriter<S: ClipboardImageWriterSettings> {
    settings: S,
}

impl<S: ClipboardImageWriterSettings> ClipboardImageWriter<S> {
    pub fn new(settings: S) -> Self {
        Self { settings }
    }

    /// Builds the full write plan for a clipboard PNG. Pure computation:
    /// validates the vault path, constructs the target directory and
    /// filename, and formats the Markdown link.
    ///
    /// Returns [`TraceError::InvalidVaultPath`] if the configured vault
    /// path is blank. No filesystem or image I/O is performed.
    pub fn plan(&self, now: DateTime<Utc>) -> Result<ImageWritePlan, TraceError> {
        let vault = validated_vault_path(self.settings.vault_path(), "vault path")?;

        let day_folder = day_folder_string(now);
        let filename = png_filename(now);
        let relative_path = relative_asset_path(&day_folder, &filename);
        let markdown_link = image_markdown_link(&relative_path);

        let target_path = vault
            .join(self.settings.daily_folder_name())
            .join("assets")
            .join(&day_folder)
            .join(&filename);

        Ok(ImageWritePlan {
            target_path,
            relative_path,
            markdown_link,
        })
    }

    /// Writes `png_bytes` to disk under the configured vault's daily asset
    /// tree and returns the full [`ImageWritePlan`] that drove the write.
    ///
    /// This is a thin wrapper over [`Self::plan`] + `create_dir_all` +
    /// [`crate::writer::atomic::write_atomic`]. The byte-level path and
    /// Markdown-link format are locked by the `plan` tests — we deliberately
    /// do **not** duplicate path construction here.
    ///
    /// Mirrors Swift's `ClipboardImageWriter.saveFromPasteboardImage`
    /// (atomic temp + rename, recursive parent-dir creation, overwrites
    /// allowed at the target path) but returns the full plan instead of
    /// just the Markdown link so Rust callers can log the target path.
    ///
    /// `png_bytes` is trusted as-is — the platform layer (Task 13.2) owns
    /// clipboard reading and PNG encoding; this method performs no magic-
    /// number validation. Empty slices are accepted (Swift parity: Swift's
    /// `Data.write` does not reject empty data either).
    ///
    /// Errors:
    /// - [`TraceError::InvalidVaultPath`] if the configured vault path is
    ///   blank (propagated from [`Self::plan`] before any I/O).
    /// - [`TraceError::AtomicWriteFailed`] if the parent-directory
    ///   creation or the atomic write itself fails.
    pub fn write_png(
        &self,
        png_bytes: &[u8],
        now: DateTime<Utc>,
    ) -> Result<ImageWritePlan, TraceError> {
        let plan = self.plan(now)?;

        let parent = plan
            .target_path
            .parent()
            .expect("plan.target_path always has a parent (vault-rooted)");
        std::fs::create_dir_all(parent).map_err(|e| {
            TraceError::AtomicWriteFailed(format!(
                "create dir {} failed: {e}",
                parent.display()
            ))
        })?;

        crate::writer::atomic::write_atomic(&plan.target_path, png_bytes)?;

        Ok(plan)
    }
}

/// Formats `now` as `yyyy-MM-dd` for the per-day asset folder. Mirrors
/// Swift's `dayFolderString` (en_US_POSIX locale, so locale-independent).
fn day_folder_string(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%d").to_string()
}

/// Formats `now` as `yyyyMMdd-HHmmss-SSS` for the filename stem. Mirrors
/// Swift's `filenameTimestamp`. `%3f` yields zero-padded milliseconds.
fn filename_timestamp(now: DateTime<Utc>) -> String {
    now.format("%Y%m%d-%H%M%S-%3f").to_string()
}

/// Returns `trace-{filename_timestamp}.png`. The `trace-` prefix matches
/// Swift verbatim — the brand doubles as a collision-avoidance token.
fn png_filename(now: DateTime<Utc>) -> String {
    format!("trace-{}.png", filename_timestamp(now))
}

/// Joins the day folder and filename with a forward slash for use inside
/// the Markdown link. Always forward-slash regardless of platform — the
/// link is read by Markdown renderers, not by the OS path resolver.
fn relative_asset_path(day_folder: &str, filename: &str) -> String {
    format!("assets/{day_folder}/{filename}")
}

/// Wraps `relative_path` in the `![image]({…})` Markdown syntax. Byte
/// parity with Swift's `"![image](\(relativePath))"`.
fn image_markdown_link(relative_path: &str) -> String {
    format!("![image]({relative_path})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    struct StubSettings {
        vault: PathBuf,
        daily_folder: String,
    }

    impl ClipboardImageWriterSettings for StubSettings {
        fn vault_path(&self) -> &Path {
            &self.vault
        }
        fn daily_folder_name(&self) -> &str {
            &self.daily_folder
        }
    }

    fn fixed_time_ms(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32, ms: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_milli_opt(h, mi, s, ms)
            .unwrap()
            .and_utc()
    }

    #[test]
    fn day_folder_string_is_yyyy_mm_dd_zero_padded() {
        let t = Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap();
        assert_eq!(day_folder_string(t), "2026-01-05");
    }

    #[test]
    fn filename_timestamp_includes_three_digit_milliseconds() {
        let t = fixed_time_ms(2026, 4, 20, 12, 5, 0, 123);
        assert_eq!(filename_timestamp(t), "20260420-120500-123");
    }

    #[test]
    fn filename_timestamp_zero_pads_milliseconds() {
        // 0ms → "000", 7ms → "007" — regression test for a common
        // `%f`-family pitfall where nanoseconds leak into the output
        // unless the width specifier (`%3f`) is used.
        let zero = fixed_time_ms(2026, 4, 20, 12, 5, 0, 0);
        assert_eq!(filename_timestamp(zero), "20260420-120500-000");
        let seven = fixed_time_ms(2026, 4, 20, 12, 5, 0, 7);
        assert_eq!(filename_timestamp(seven), "20260420-120500-007");
    }

    #[test]
    fn png_filename_has_trace_prefix_and_png_extension() {
        let t = fixed_time_ms(2026, 4, 20, 12, 5, 0, 123);
        assert_eq!(png_filename(t), "trace-20260420-120500-123.png");
    }

    #[test]
    fn relative_asset_path_uses_forward_slashes() {
        assert_eq!(
            relative_asset_path("2026-04-20", "trace-20260420-120500-123.png"),
            "assets/2026-04-20/trace-20260420-120500-123.png",
        );
    }

    #[test]
    fn image_markdown_link_matches_swift_bytes() {
        let rel = "assets/2026-04-20/trace-20260420-120500-123.png";
        assert_eq!(
            image_markdown_link(rel),
            "![image](assets/2026-04-20/trace-20260420-120500-123.png)",
        );
    }

    #[test]
    fn plan_produces_absolute_target_and_markdown_link() {
        let settings = StubSettings {
            vault: PathBuf::from("/vault"),
            daily_folder: "daily".to_string(),
        };
        let writer = ClipboardImageWriter::new(settings);
        let now = fixed_time_ms(2026, 4, 20, 12, 5, 0, 123);

        let plan = writer.plan(now).unwrap();

        assert_eq!(
            plan.target_path,
            PathBuf::from("/vault/daily/assets/2026-04-20/trace-20260420-120500-123.png"),
        );
        assert_eq!(
            plan.relative_path,
            "assets/2026-04-20/trace-20260420-120500-123.png",
        );
        assert_eq!(
            plan.markdown_link,
            "![image](assets/2026-04-20/trace-20260420-120500-123.png)",
        );
    }

    #[test]
    fn plan_rejects_blank_vault_path() {
        let settings = StubSettings {
            vault: PathBuf::from("   "),
            daily_folder: "daily".to_string(),
        };
        let writer = ClipboardImageWriter::new(settings);
        let now = fixed_time_ms(2026, 4, 20, 12, 5, 0, 123);

        let err = writer.plan(now).unwrap_err();
        assert!(
            matches!(err, TraceError::InvalidVaultPath(_)),
            "expected InvalidVaultPath, got {err:?}",
        );
    }

    #[test]
    fn plan_rejects_empty_vault_path() {
        let settings = StubSettings {
            vault: PathBuf::from(""),
            daily_folder: "daily".to_string(),
        };
        let writer = ClipboardImageWriter::new(settings);
        let now = fixed_time_ms(2026, 4, 20, 12, 5, 0, 123);

        assert!(matches!(
            writer.plan(now).unwrap_err(),
            TraceError::InvalidVaultPath(_),
        ));
    }

    #[test]
    fn plan_respects_custom_daily_folder_name() {
        let settings = StubSettings {
            vault: PathBuf::from("/vault"),
            daily_folder: "journal".to_string(),
        };
        let writer = ClipboardImageWriter::new(settings);
        let now = fixed_time_ms(2026, 4, 20, 12, 5, 0, 0);

        let plan = writer.plan(now).unwrap();
        assert_eq!(
            plan.target_path,
            PathBuf::from("/vault/journal/assets/2026-04-20/trace-20260420-120500-000.png"),
        );
        // Relative path and markdown link are vault-relative — they do
        // NOT include the daily folder name (Swift parity: the image is
        // linked from inside the daily note, which already lives under
        // that folder, so the link starts at `assets/`).
        assert_eq!(
            plan.relative_path,
            "assets/2026-04-20/trace-20260420-120500-000.png",
        );
    }

    #[test]
    fn image_write_plan_is_equatable_and_cloneable() {
        let a = ImageWritePlan {
            target_path: PathBuf::from("/a.png"),
            relative_path: "a.png".into(),
            markdown_link: "![image](a.png)".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ------------------------------------------------------------------
    // Phase 13 — `write_png` (disk-backed) tests.
    //
    // The byte-level path and Markdown-link format stays pinned by the
    // `plan` tests above; these tests only verify the disk-write contract:
    // atomic write, recursive parent creation, overwrite-allowed, and the
    // invariance that `write_png` returns exactly what `plan` returns.
    // ------------------------------------------------------------------

    /// Fixed `now` reused by every `write_png` test so the produced paths
    /// remain byte-identical to the Swift reference asserted in `plan` tests.
    fn pinned_now() -> DateTime<Utc> {
        fixed_time_ms(2026, 4, 20, 12, 5, 0, 123)
    }

    fn stub_settings(vault: &Path) -> StubSettings {
        StubSettings {
            vault: vault.to_path_buf(),
            daily_folder: "daily".to_string(),
        }
    }

    #[test]
    fn write_png_creates_asset_tree_and_writes_bytes_atomically() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let writer = ClipboardImageWriter::new(stub_settings(tempdir.path()));
        let now = pinned_now();
        let payload: &[u8] = b"\x89PNG";

        let plan = writer.write_png(payload, now).unwrap();

        // File landed at the planned path with byte-exact contents.
        assert!(plan.target_path.exists(), "target path must exist on disk");
        assert_eq!(std::fs::read(&plan.target_path).unwrap(), payload);

        // Markdown link is byte-identical to Swift's reference format.
        assert_eq!(
            plan.markdown_link,
            "![image](assets/2026-04-20/trace-20260420-120500-123.png)",
        );

        // Atomic-write should leave no temp siblings behind on the success
        // path. The filter substring below mirrors the exact filename format
        // produced by `crate::writer::atomic::temp_sibling_path`:
        // `.{filename}.trace-tmp-{pid}-{nanos}-{counter}`. Matching on the
        // leading-dot prefix (`.trace-tmp-`) guards the assertion against
        // false negatives if `atomic.rs` ever renames its temp-file scheme.
        let parent = plan.target_path.parent().unwrap();
        let leftover: Vec<_> = std::fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".trace-tmp-"))
            .collect();
        assert!(
            leftover.is_empty(),
            "expected no .trace-tmp-* siblings, found {leftover:?}",
        );
    }

    #[test]
    fn write_png_creates_parent_dirs_recursively() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let writer = ClipboardImageWriter::new(stub_settings(tempdir.path()));
        let now = pinned_now();

        // Pre-condition: none of the intermediate dirs exist yet.
        assert!(!tempdir.path().join("daily").exists());

        writer.write_png(b"bytes", now).unwrap();

        // Each layer of the asset tree must be materialized.
        assert!(tempdir.path().join("daily").is_dir());
        assert!(tempdir.path().join("daily/assets").is_dir());
        assert!(tempdir.path().join("daily/assets/2026-04-20").is_dir());
    }

    #[test]
    fn write_png_overwrites_existing_file_at_same_timestamp() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let writer = ClipboardImageWriter::new(stub_settings(tempdir.path()));
        let now = pinned_now();

        let first = writer.write_png(b"first payload", now).unwrap();
        let second = writer.write_png(b"second payload", now).unwrap();

        // Same `now` ⇒ same target path, second write wins.
        assert_eq!(first.target_path, second.target_path);
        assert_eq!(
            std::fs::read(&second.target_path).unwrap(),
            b"second payload",
        );
    }

    #[test]
    fn write_png_rejects_invalid_vault_path() {
        let settings = StubSettings {
            vault: PathBuf::from("   "),
            daily_folder: "daily".to_string(),
        };
        let writer = ClipboardImageWriter::new(settings);

        let err = writer.write_png(b"x", pinned_now()).unwrap_err();

        // Error must come from the `plan()` stage before any I/O happens —
        // `InvalidVaultPath`, not `AtomicWriteFailed`.
        assert!(
            matches!(err, TraceError::InvalidVaultPath(_)),
            "expected InvalidVaultPath, got {err:?}",
        );
    }

    #[test]
    fn write_png_accepts_empty_byte_slice() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let writer = ClipboardImageWriter::new(stub_settings(tempdir.path()));

        let plan = writer.write_png(&[], pinned_now()).unwrap();

        assert!(plan.target_path.exists());
        assert_eq!(
            std::fs::metadata(&plan.target_path).unwrap().len(),
            0,
            "empty payload must yield a zero-byte file (Swift parity)",
        );
    }

    #[test]
    fn write_png_returns_same_plan_as_standalone_plan_call() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let writer = ClipboardImageWriter::new(stub_settings(tempdir.path()));
        let now = pinned_now();

        // Same `now` + same settings ⇒ `plan` and `write_png` must agree on
        // every field of `ImageWritePlan`. This guards against future
        // refactors that might duplicate path construction inside `write_png`.
        let planned = writer.plan(now).unwrap();
        let written = writer.write_png(b"payload", now).unwrap();

        assert_eq!(planned, written);
    }
}

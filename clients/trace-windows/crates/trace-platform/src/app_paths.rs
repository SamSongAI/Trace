//! Windows known-folder resolution for Trace application directories.
//!
//! Provides the canonical on-disk locations where Trace stores its data:
//!
//! | Directory | Win32 Known Folder | Purpose |
//! |-----------|-------------------|---------|
//! | `%APPDATA%\Trace` | `FOLDERID_RoamingAppData` | `settings.json` and other per-user config that should roam across machines |
//! | `%LOCALAPPDATA%\Trace` | `FOLDERID_LocalAppData` | Logs and caches that are machine-local |
//!
//! All public functions call `SHGetKnownFolderPath` with `KF_FLAG_CREATE` (so
//! the shell-managed parent directory is created if absent) and then create
//! the `Trace` sub-directory with [`std::fs::create_dir_all`] before returning
//! the path.
//!
//! Non-Windows targets expose only [`AppPathsError`] so the crate still
//! compiles on macOS/Linux developer machines.

use std::fmt;

/// Error returned when resolving or creating an application directory.
#[derive(Debug)]
pub enum AppPathsError {
    /// `SHGetKnownFolderPath` returned a failure HRESULT. In practice this
    /// is very rare — it usually means the user profile is corrupt or the
    /// process is running with unusual privilege restrictions. The
    /// `hresult` field carries the raw 32-bit HRESULT value.
    KnownFolderResolution { hresult: i32 },
    /// The path returned by `SHGetKnownFolderPath` could not be decoded as
    /// valid UTF-16. Windows in principle allows non-UTF-16 paths, but
    /// every known folder on a well-formed system is valid UTF-16; this
    /// variant exists as a forward-looking safety net.
    InvalidPathEncoding,
    /// `std::fs::create_dir_all` failed while creating the `Trace`
    /// sub-directory. The `io_kind` field reports the underlying I/O kind.
    CreateDirectory { io_kind: std::io::ErrorKind },
}

impl fmt::Display for AppPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppPathsError::KnownFolderResolution { hresult } => write!(
                f,
                "SHGetKnownFolderPath failed (HRESULT {hresult:#010x})"
            ),
            AppPathsError::InvalidPathEncoding => f.write_str(
                "the path returned by SHGetKnownFolderPath is not valid UTF-16",
            ),
            AppPathsError::CreateDirectory { io_kind } => write!(
                f,
                "failed to create the Trace application directory: {io_kind:?}"
            ),
        }
    }
}

impl std::error::Error for AppPathsError {}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::AppPathsError;
    use std::path::PathBuf;

    use windows::core::PWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_LocalAppData, FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_CREATE,
    };

    /// The sub-directory name appended to every known-folder base path.
    const APP_SUBDIR: &str = "Trace";

    /// RAII wrapper for the `PWSTR` buffer that `SHGetKnownFolderPath`
    /// returns. The buffer is allocated by the shell with `CoTaskMemAlloc`
    /// and must be released with `CoTaskMemFree`. Holding the pointer in
    /// a `Drop` type ensures the free happens even on early-return paths
    /// — mirrors the `MenuGuard` pattern in [`crate::system_tray`].
    struct CoTaskMemPwstr(PWSTR);

    impl Drop for CoTaskMemPwstr {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: the pointer was handed to us by
                // `SHGetKnownFolderPath` on success and no other copy of
                // it has been freed elsewhere.
                unsafe { CoTaskMemFree(Some(self.0 .0 as *const _)) };
            }
        }
    }

    /// Calls `SHGetKnownFolderPath`, converts the UTF-16 result to a
    /// [`PathBuf`], appends `"Trace"`, creates the sub-directory, and
    /// returns it.
    fn resolve_and_create(folder_id: *const windows::core::GUID) -> Result<PathBuf, AppPathsError> {
        // SAFETY: `folder_id` is a valid pointer to a static GUID
        // constant; `KF_FLAG_CREATE` is a well-defined flag value; passing
        // `HANDLE::default()` (NULL) selects the current user's token,
        // which is what we want for a user-scope app.
        let raw = unsafe {
            SHGetKnownFolderPath(folder_id, KF_FLAG_CREATE, HANDLE::default()).map_err(|e| {
                AppPathsError::KnownFolderResolution {
                    hresult: e.code().0,
                }
            })?
        };

        // Wrap immediately so the allocation is freed on every path below.
        let guard = CoTaskMemPwstr(raw);

        // SAFETY: `guard.0` is a valid NUL-terminated UTF-16 pointer
        // returned by a successful `SHGetKnownFolderPath` call.
        let path_string = unsafe { guard.0.to_string() }
            .map_err(|_| AppPathsError::InvalidPathEncoding)?;

        let mut path = PathBuf::from(path_string);
        path.push(APP_SUBDIR);

        std::fs::create_dir_all(&path).map_err(|e| AppPathsError::CreateDirectory {
            io_kind: e.kind(),
        })?;

        Ok(path)
    }

    /// Returns `%APPDATA%\Trace` (Roaming AppData).
    ///
    /// The directory is created if it does not already exist. Use this
    /// for `settings.json` and other per-user configuration that should
    /// roam across machines signed in with the same Microsoft account.
    pub fn roaming_app_data_dir() -> Result<PathBuf, AppPathsError> {
        resolve_and_create(&FOLDERID_RoamingAppData)
    }

    /// Returns `%LOCALAPPDATA%\Trace` (Local AppData).
    ///
    /// The directory is created if it does not already exist. Use this
    /// for logs, caches, and other machine-local data that should not
    /// roam.
    pub fn local_app_data_dir() -> Result<PathBuf, AppPathsError> {
        resolve_and_create(&FOLDERID_LocalAppData)
    }

    /// Returns `%APPDATA%\Trace\settings.json`.
    ///
    /// The parent directory is created; the file itself is not. This
    /// function only answers "where should settings live".
    pub fn settings_file_path() -> Result<PathBuf, AppPathsError> {
        let mut p = roaming_app_data_dir()?;
        p.push("settings.json");
        Ok(p)
    }

    /// Returns `%LOCALAPPDATA%\Trace\logs`, creating it if absent.
    pub fn log_dir() -> Result<PathBuf, AppPathsError> {
        let mut p = local_app_data_dir()?;
        p.push("logs");
        std::fs::create_dir_all(&p).map_err(|e| AppPathsError::CreateDirectory {
            io_kind: e.kind(),
        })?;
        Ok(p)
    }
}

#[cfg(windows)]
pub use imp::{local_app_data_dir, log_dir, roaming_app_data_dir, settings_file_path};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_paths_error_display_includes_error_code_or_kind() {
        // -2147024893 == 0x80070003 (ERROR_PATH_NOT_FOUND wrapped as HRESULT).
        let msg =
            AppPathsError::KnownFolderResolution { hresult: -2147024893_i32 }.to_string();
        assert!(
            msg.contains("0x80070003"),
            "Display should include hex HRESULT, got: {msg:?}"
        );

        let msg = AppPathsError::InvalidPathEncoding.to_string();
        assert!(!msg.is_empty(), "Display should be non-empty");

        let msg = AppPathsError::CreateDirectory {
            io_kind: std::io::ErrorKind::PermissionDenied,
        }
        .to_string();
        assert!(
            msg.contains("PermissionDenied"),
            "Display should include the io kind, got: {msg:?}"
        );
    }

    #[test]
    fn app_paths_error_debug_includes_variant_name() {
        let r = format!("{:?}", AppPathsError::InvalidPathEncoding);
        assert!(r.contains("InvalidPathEncoding"), "got: {r:?}");

        let r = format!(
            "{:?}",
            AppPathsError::KnownFolderResolution { hresult: 0 }
        );
        assert!(r.contains("KnownFolderResolution"), "got: {r:?}");

        let r = format!(
            "{:?}",
            AppPathsError::CreateDirectory {
                io_kind: std::io::ErrorKind::NotFound,
            }
        );
        assert!(r.contains("CreateDirectory"), "got: {r:?}");
    }

    #[test]
    fn app_paths_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<AppPathsError>();
    }

    // ---- Windows-only integration tests ------------------------------------
    //
    // These need an interactive Windows session to hit the real file system.
    // Run on Windows with: `cargo test -p trace-platform -- --ignored`.

    #[cfg(windows)]
    mod windows_only {
        use super::super::{
            local_app_data_dir, log_dir, roaming_app_data_dir, settings_file_path,
        };

        #[test]
        #[ignore = "requires a Windows interactive session; run manually on Windows"]
        fn roaming_app_data_dir_ends_with_trace_and_exists() {
            let dir = roaming_app_data_dir().expect("roaming_app_data_dir should succeed");
            assert_eq!(
                dir.file_name().and_then(|n| n.to_str()),
                Some("Trace"),
                "directory should end with 'Trace', got: {}",
                dir.display()
            );
            assert!(dir.exists(), "directory should exist: {}", dir.display());
        }

        #[test]
        #[ignore = "requires a Windows interactive session; run manually on Windows"]
        fn local_app_data_dir_ends_with_trace_and_exists() {
            let dir = local_app_data_dir().expect("local_app_data_dir should succeed");
            assert_eq!(
                dir.file_name().and_then(|n| n.to_str()),
                Some("Trace"),
                "directory should end with 'Trace', got: {}",
                dir.display()
            );
            assert!(dir.exists(), "directory should exist: {}", dir.display());
        }

        #[test]
        #[ignore = "requires a Windows interactive session; run manually on Windows"]
        fn settings_file_path_ends_with_settings_json_and_parent_exists() {
            let path = settings_file_path().expect("settings_file_path should succeed");
            assert_eq!(
                path.file_name().and_then(|n| n.to_str()),
                Some("settings.json"),
                "path should end with 'settings.json', got: {}",
                path.display()
            );
            let parent = path.parent().expect("path should have a parent");
            assert!(
                parent.exists(),
                "parent directory should exist: {}",
                parent.display()
            );
        }

        #[test]
        #[ignore = "requires a Windows interactive session; run manually on Windows"]
        fn log_dir_ends_with_logs_and_exists() {
            let dir = log_dir().expect("log_dir should succeed");
            assert_eq!(
                dir.file_name().and_then(|n| n.to_str()),
                Some("logs"),
                "directory should end with 'logs', got: {}",
                dir.display()
            );
            assert!(dir.exists(), "log dir should exist: {}", dir.display());
        }
    }
}

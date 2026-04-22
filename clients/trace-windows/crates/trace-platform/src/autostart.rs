//! Launch-at-login integration via the Windows Run registry key.
//!
//! The canonical place to register a user-scope auto-start entry on Windows
//! is:
//!
//! ```text
//! HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
//! ```
//!
//! Each value under that key is a "name → command-line" mapping. At user
//! logon, the shell launches every entry in sequence. Trace uses the app
//! name (default `"Trace"`) as the value name and the executable path as
//! the value data. Because the key is user-scope (`HKCU`, not `HKLM`), no
//! elevation is required to toggle the entry.
//!
//! ## Path quoting
//!
//! The executable path is quoted with surrounding double quotes so that
//! paths containing spaces (`C:\Program Files\Trace\trace.exe`) parse
//! correctly. The shell tolerates either quoted or unquoted paths when the
//! path has no spaces, but always quoting is safer.
//!
//! ## Non-Windows targets
//!
//! The error enum [`AutostartError`] is cross-platform; the read/write
//! functions are Windows-only. On macOS/Linux the crate compiles but this
//! module exposes no callable functions (launch-at-login on macOS is
//! handled elsewhere through `SMAppService`).

use std::fmt;

/// Subkey under `HKEY_CURRENT_USER` that holds the per-user Run entries.
const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Error returned by the registry manipulation functions.
#[derive(Debug)]
pub enum AutostartError {
    /// `RegOpenKeyExW` failed. The field is the raw Win32 error code from
    /// the call. Typical causes: registry hive is corrupt (very rare) or
    /// a group policy has locked down write access to the Run key.
    OpenKey { code: u32 },
    /// `RegQueryValueExW` failed with an error other than
    /// `ERROR_FILE_NOT_FOUND` (which is treated as "entry does not exist").
    QueryValue { code: u32 },
    /// `RegSetValueExW` failed.
    SetValue { code: u32 },
    /// `RegDeleteValueW` failed with an error other than
    /// `ERROR_FILE_NOT_FOUND` (deleting an absent entry is a no-op).
    DeleteValue { code: u32 },
    /// The executable path could not be converted to a valid UTF-16 string
    /// (it contained an unpaired surrogate or interior NUL). This should
    /// be impossible for a path handed out by [`std::env::current_exe`],
    /// but is reported defensively.
    InvalidPathEncoding,
}

impl fmt::Display for AutostartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AutostartError::OpenKey { code } => write!(
                f,
                "RegOpenKeyExW failed for the Run key (Win32 error {code})"
            ),
            AutostartError::QueryValue { code } => {
                write!(f, "RegQueryValueExW failed (Win32 error {code})")
            }
            AutostartError::SetValue { code } => {
                write!(f, "RegSetValueExW failed (Win32 error {code})")
            }
            AutostartError::DeleteValue { code } => {
                write!(f, "RegDeleteValueW failed (Win32 error {code})")
            }
            AutostartError::InvalidPathEncoding => {
                f.write_str("executable path could not be encoded as UTF-16")
            }
        }
    }
}

impl std::error::Error for AutostartError {}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
pub use imp::{disable, enable, is_enabled};

#[cfg(windows)]
mod imp {
    use super::{AutostartError, RUN_KEY_PATH};
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
    };

    /// RAII guard that closes an open HKEY on drop.
    struct KeyGuard(HKEY);

    impl Drop for KeyGuard {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: `self.0` was returned by a successful
                // `RegOpenKeyExW` and has not been closed elsewhere.
                let _ = unsafe { RegCloseKey(self.0) };
            }
        }
    }

    /// Encodes `s` as a NUL-terminated UTF-16 buffer.
    fn utf16_nul(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Opens the Run key under HKCU with the requested access mask.
    fn open_run_key(access: u32) -> Result<KeyGuard, AutostartError> {
        let path_utf16 = utf16_nul(RUN_KEY_PATH);
        let mut hkey = HKEY::default();
        // SAFETY: `path_utf16` outlives the call; `&mut hkey` is a valid
        // out-param for the returned key handle.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path_utf16.as_ptr()),
                0,
                windows::Win32::System::Registry::REG_SAM_FLAGS(access),
                &mut hkey,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(AutostartError::OpenKey { code: status.0 });
        }
        Ok(KeyGuard(hkey))
    }

    /// Returns whether an autostart entry named `app_name` exists under the
    /// Run key. Does not validate the value's contents.
    pub fn is_enabled(app_name: &str) -> Result<bool, AutostartError> {
        let key = open_run_key(KEY_QUERY_VALUE.0)?;
        let value_name = utf16_nul(app_name);
        // We only need presence, not contents — pass null pointers for the
        // output buffers and rely on the status code.
        // SAFETY: `value_name` outlives the call; all null out-params are
        // explicitly allowed by RegQueryValueExW when caller only wants
        // existence / size information.
        let status =
            unsafe { RegQueryValueExW(key.0, PCWSTR(value_name.as_ptr()), None, None, None, None) };
        match status {
            s if s == ERROR_SUCCESS => Ok(true),
            s if s == ERROR_FILE_NOT_FOUND => Ok(false),
            WIN32_ERROR(code) => Err(AutostartError::QueryValue { code }),
        }
    }

    /// Sets the autostart entry `app_name` to launch `exe_path` (quoted)
    /// at user logon. Overwrites any existing entry with the same name.
    pub fn enable(app_name: &str, exe_path: &Path) -> Result<(), AutostartError> {
        let exe_str = exe_path
            .to_str()
            .ok_or(AutostartError::InvalidPathEncoding)?;
        // Quote the path so spaces inside `exe_str` don't get split by
        // the shell's argv parser at launch.
        let quoted = format!("\"{exe_str}\"");
        let data_utf16: Vec<u16> = quoted.encode_utf16().chain(std::iter::once(0)).collect();

        let key = open_run_key(KEY_SET_VALUE.0)?;
        let value_name = utf16_nul(app_name);

        // Byte count includes the trailing NUL (RegSetValueExW expects
        // `cbData` in bytes, not wide characters).
        let byte_count = (data_utf16.len() * std::mem::size_of::<u16>()) as u32;
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                data_utf16.as_ptr() as *const u8,
                data_utf16.len() * std::mem::size_of::<u16>(),
            )
        };
        let _ = byte_count; // retained for clarity — windows-rs infers len from slice

        // SAFETY: `value_name` and `data_utf16` (backing `byte_slice`)
        // both outlive the call.
        let status = unsafe {
            RegSetValueExW(
                key.0,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(byte_slice),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(AutostartError::SetValue { code: status.0 });
        }
        Ok(())
    }

    /// Removes the autostart entry `app_name` if present. Deleting an
    /// absent entry is a silent no-op (not an error).
    pub fn disable(app_name: &str) -> Result<(), AutostartError> {
        let key = open_run_key(KEY_SET_VALUE.0)?;
        let value_name = utf16_nul(app_name);
        // SAFETY: `value_name` outlives the call.
        let status = unsafe { RegDeleteValueW(key.0, PCWSTR(value_name.as_ptr())) };
        match status {
            s if s == ERROR_SUCCESS => Ok(()),
            s if s == ERROR_FILE_NOT_FOUND => Ok(()),
            WIN32_ERROR(code) => Err(AutostartError::DeleteValue { code }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_error_display_includes_code_or_hint() {
        let msg = AutostartError::OpenKey { code: 5 }.to_string();
        assert!(msg.contains("5"), "got: {msg:?}");

        let msg = AutostartError::QueryValue { code: 2 }.to_string();
        assert!(msg.contains("2"), "got: {msg:?}");

        let msg = AutostartError::SetValue { code: 5 }.to_string();
        assert!(msg.contains("5"), "got: {msg:?}");

        let msg = AutostartError::DeleteValue { code: 2 }.to_string();
        assert!(msg.contains("2"), "got: {msg:?}");

        let msg = AutostartError::InvalidPathEncoding.to_string();
        assert!(
            !msg.is_empty() && !msg.contains("InvalidPathEncoding"),
            "Display should be human-readable, got: {msg:?}"
        );
    }

    #[test]
    fn autostart_error_debug_includes_variant_name() {
        let r = format!("{:?}", AutostartError::OpenKey { code: 0 });
        assert!(r.contains("OpenKey"), "got: {r:?}");

        let r = format!("{:?}", AutostartError::QueryValue { code: 0 });
        assert!(r.contains("QueryValue"), "got: {r:?}");

        let r = format!("{:?}", AutostartError::SetValue { code: 0 });
        assert!(r.contains("SetValue"), "got: {r:?}");

        let r = format!("{:?}", AutostartError::DeleteValue { code: 0 });
        assert!(r.contains("DeleteValue"), "got: {r:?}");

        let r = format!("{:?}", AutostartError::InvalidPathEncoding);
        assert!(r.contains("InvalidPathEncoding"), "got: {r:?}");
    }

    #[test]
    fn autostart_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<AutostartError>();
    }

    // ---- Windows-only integration tests ------------------------------------
    //
    // These tests write to the real HKCU Run key under a test-specific app
    // name, then clean up. They are `#[ignore]` because they require a
    // Windows session and mutate user state.

    #[cfg(windows)]
    mod windows_only {
        use super::super::{disable, enable, is_enabled};
        use std::path::PathBuf;

        /// Use a unique app name so we don't collide with a real Trace
        /// install if one happens to exist on the test box.
        const TEST_APP_NAME: &str = "TracePlatformTest_DoNotUseInProduction";

        #[test]
        #[ignore = "requires a Windows interactive session; mutates HKCU Run key"]
        fn enable_then_is_enabled_then_disable_round_trip() {
            // Pre-condition: the test entry should not exist yet. If a
            // previous aborted run left it behind, clean up.
            let _ = disable(TEST_APP_NAME);
            assert!(
                !is_enabled(TEST_APP_NAME).expect("is_enabled should succeed"),
                "test entry should not exist before enable"
            );

            let fake_exe = PathBuf::from(r"C:\Program Files\Trace\trace.exe");
            enable(TEST_APP_NAME, &fake_exe).expect("enable should succeed");
            assert!(
                is_enabled(TEST_APP_NAME).expect("is_enabled should succeed"),
                "test entry should exist after enable"
            );

            disable(TEST_APP_NAME).expect("disable should succeed");
            assert!(
                !is_enabled(TEST_APP_NAME).expect("is_enabled should succeed"),
                "test entry should not exist after disable"
            );

            // Disabling an absent entry should be a no-op, not an error.
            disable(TEST_APP_NAME).expect("disabling an absent entry should be a no-op");
        }
    }
}

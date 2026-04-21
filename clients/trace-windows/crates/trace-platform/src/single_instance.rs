//! Single-instance enforcement via a named Win32 mutex.
//!
//! Windows does not provide a built-in "only one copy of this app" guard the
//! way macOS does through the bundle identifier. The standard approach is to
//! create a named mutex in the user's local namespace at startup. If the
//! mutex already exists, another instance is running and the current process
//! should exit (ideally after posting a "please show yourself" signal to the
//! existing instance — that second step is a future enhancement).
//!
//! ## Name scoping
//!
//! The mutex name uses the `Local\` prefix, which scopes it to the current
//! user's logon session. This is the correct level of isolation for a user
//! app: two different users on the same machine can each run their own copy
//! of Trace, but a single user cannot launch the app twice.
//!
//! ## Usage
//!
//! ```no_run
//! # #[cfg(windows)]
//! # fn main() {
//! use trace_platform::single_instance::{acquire, SingleInstance};
//!
//! match acquire().expect("mutex creation should not fail") {
//!     SingleInstance::Acquired(_guard) => {
//!         // First instance — proceed with normal startup. Hold the guard
//!         // for the entire lifetime of the process so the mutex is only
//!         // released when we truly exit.
//!     }
//!     SingleInstance::AlreadyRunning => {
//!         // Another copy is already running. Exit quietly.
//!         std::process::exit(0);
//!     }
//! }
//! # }
//! # #[cfg(not(windows))]
//! # fn main() {}
//! ```

use std::fmt;

/// The mutex name. `Local\` scopes it to the current user's logon session.
/// The suffix is a bundle-ID-style constant to avoid colliding with other
/// apps that happen to pick the word "Trace".
const MUTEX_NAME: &str = "Local\\com.samsong.Trace.Singleton";

/// Error returned by [`acquire`] when the underlying Win32 call fails.
#[derive(Debug)]
pub enum SingleInstanceError {
    /// `CreateMutexW` returned a null handle. The `code` field is the raw
    /// Win32 error code from `GetLastError`. This is extremely unusual —
    /// the only documented failure modes are out-of-memory and an
    /// excessively long name.
    CreateMutexFailed { code: u32 },
}

impl fmt::Display for SingleInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SingleInstanceError::CreateMutexFailed { code } => {
                write!(f, "CreateMutexW failed (Win32 error {code})")
            }
        }
    }
}

impl std::error::Error for SingleInstanceError {}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
pub use imp::{acquire, SingleInstance, SingleInstanceGuard};

#[cfg(windows)]
mod imp {
    use super::{SingleInstanceError, MUTEX_NAME};

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
    use windows::Win32::System::Threading::CreateMutexW;

    /// Outcome of attempting to acquire the single-instance guard.
    ///
    /// [`SingleInstance::Acquired`] means this process is the first
    /// instance and now owns the mutex — hold the contained
    /// [`SingleInstanceGuard`] for the entire process lifetime.
    /// [`SingleInstance::AlreadyRunning`] means another copy is already
    /// running; the caller should exit.
    pub enum SingleInstance {
        Acquired(SingleInstanceGuard),
        AlreadyRunning,
    }

    /// RAII owner of the named mutex handle. Closes the handle on drop,
    /// which releases the single-instance claim.
    pub struct SingleInstanceGuard {
        handle: HANDLE,
    }

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            if !self.handle.is_invalid() {
                // SAFETY: `handle` was returned by a successful `CreateMutexW`
                // and has not been closed elsewhere.
                let _ = unsafe { CloseHandle(self.handle) };
            }
        }
    }

    // `HANDLE` is not `Send`/`Sync` by default in the `windows` crate, but a
    // mutex handle is safe to transfer across threads once created — we only
    // close it on drop. Holding the guard on a different thread than the one
    // that created it is fine for our use case (typically it lives on the
    // main thread for the whole process).
    //
    // We don't actually need `Send`/`Sync` for the intended usage (hold on
    // main thread), so we deliberately do NOT add unsafe impls here. If a
    // future caller needs cross-thread transfer, that can be added with
    // explicit justification.

    /// Attempts to become the single instance.
    ///
    /// Creates a named mutex. If another process already owns a mutex with
    /// the same name, returns [`SingleInstance::AlreadyRunning`] and closes
    /// the redundant handle. Otherwise returns [`SingleInstance::Acquired`]
    /// wrapping a guard that must be held for the process lifetime.
    pub fn acquire() -> Result<SingleInstance, SingleInstanceError> {
        // Convert the UTF-8 name to a null-terminated UTF-16 buffer.
        let name_utf16: Vec<u16> = MUTEX_NAME.encode_utf16().chain(std::iter::once(0)).collect();

        // SAFETY: `name_utf16` lives until the end of this function, longer
        // than the `CreateMutexW` call. The pointer we pass is valid for
        // the duration of the call. `None` for `lpMutexAttributes` means
        // "default security descriptor (not inheritable)".
        let create_result =
            unsafe { CreateMutexW(None, false, PCWSTR(name_utf16.as_ptr())) };

        let handle = match create_result {
            Ok(h) => h,
            Err(e) => {
                return Err(SingleInstanceError::CreateMutexFailed {
                    code: e.code().0 as u32,
                });
            }
        };

        // Per MSDN: if the mutex already existed, `CreateMutexW` still
        // returns a valid handle but `GetLastError` is `ERROR_ALREADY_EXISTS`.
        // SAFETY: called immediately after `CreateMutexW` with no
        // intervening Win32 calls, so the last-error slot is fresh.
        let already_existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;

        if already_existed {
            // Close our redundant handle — the other process still owns
            // the underlying named object.
            // SAFETY: `handle` was returned by the successful CreateMutex above.
            let _ = unsafe { CloseHandle(handle) };
            Ok(SingleInstance::AlreadyRunning)
        } else {
            Ok(SingleInstance::Acquired(SingleInstanceGuard { handle }))
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
    fn single_instance_error_display_includes_code() {
        let msg = SingleInstanceError::CreateMutexFailed { code: 8 }.to_string();
        assert!(
            msg.contains("8"),
            "Display should include the Win32 error code, got: {msg:?}"
        );
    }

    #[test]
    fn single_instance_error_debug_includes_variant_name() {
        let r = format!("{:?}", SingleInstanceError::CreateMutexFailed { code: 0 });
        assert!(r.contains("CreateMutexFailed"), "got: {r:?}");
    }

    #[test]
    fn single_instance_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<SingleInstanceError>();
    }

    // ---- Windows-only integration tests ------------------------------------

    #[cfg(windows)]
    mod windows_only {
        use super::super::{acquire, SingleInstance};

        #[test]
        #[ignore = "requires a Windows interactive session; run manually on Windows"]
        fn first_acquire_returns_acquired_then_second_returns_already_running() {
            let first = acquire().expect("first acquire should succeed");
            assert!(
                matches!(first, SingleInstance::Acquired(_)),
                "first acquire should succeed"
            );

            // While the first guard is still alive, a second acquire from
            // the same process should also see the mutex as existing (the
            // handle is process-local but the *named* object is shared
            // across all openers).
            let second = acquire().expect("second acquire should succeed");
            assert!(
                matches!(second, SingleInstance::AlreadyRunning),
                "second acquire should report AlreadyRunning"
            );

            // After the first guard drops, a fresh acquire should succeed.
            drop(first);
            let third = acquire().expect("third acquire should succeed");
            assert!(
                matches!(third, SingleInstance::Acquired(_)),
                "third acquire should succeed after the first guard is dropped"
            );
        }
    }
}

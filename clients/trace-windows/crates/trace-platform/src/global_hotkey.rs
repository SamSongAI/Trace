//! Windows global hotkey integration.
//!
//! On Windows this module registers a single system-wide hotkey using
//! [`RegisterHotKey`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-registerhotkey)
//! attached to a message-only window that runs on a dedicated worker thread.
//! Each press is forwarded to the caller through a
//! [`std::sync::mpsc::Receiver`]. Dropping the returned [`GlobalHotkey`]
//! unregisters the hotkey and joins the worker thread.
//!
//! This is the Rust port of `Sources/Trace/Services/GlobalHotKeyManager.swift`
//! from the macOS client. It covers only the one app-wide hotkey that wakes
//! the capture panel — the in-panel hotkeys (send-note, append-note, mode
//! toggle) live in the iced UI layer (Phase 11) and are not handled here.
//!
//! Non-Windows targets expose only [`GlobalHotkeyError`] so the crate still
//! compiles on macOS/Linux developer machines.
//!
//! # Example (Windows-only)
//!
//! ```no_run
//! # #[cfg(windows)]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use trace_platform::global_hotkey::GlobalHotkey;
//! use trace_core::{DEFAULT_GLOBAL_HOTKEY_MODIFIERS, DEFAULT_GLOBAL_HOTKEY_VKEY};
//!
//! let hotkey = GlobalHotkey::register(
//!     DEFAULT_GLOBAL_HOTKEY_VKEY,
//!     DEFAULT_GLOBAL_HOTKEY_MODIFIERS,
//! )?;
//!
//! // Blocks until the user presses the hotkey.
//! hotkey.events().recv()?;
//! # Ok(())
//! # }
//! # #[cfg(not(windows))]
//! # fn main() {}
//! ```

use std::fmt;

/// Error returned when registering or running a global hotkey.
#[derive(Debug)]
pub enum GlobalHotkeyError {
    /// `RegisterHotKey` failed. Inner value is the Win32 error code from
    /// `GetLastError` at the moment of the failure. A common cause is that
    /// the requested `(modifiers, vkey)` pair is already registered by
    /// another process.
    RegistrationFailed(u32),
    /// `CreateWindowExW` for the message-only window failed.
    WindowCreationFailed(u32),
    /// `RegisterClassExW` failed.
    WindowClassFailed(u32),
    /// The worker thread could not be spawned.
    ThreadSpawnFailed(String),
    /// The worker thread panicked or exited without sending a registration
    /// result back to the caller.
    ThreadDied,
}

impl fmt::Display for GlobalHotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlobalHotkeyError::RegistrationFailed(code) => write!(
                f,
                "RegisterHotKey failed (Win32 error {code}); another process may own this shortcut"
            ),
            GlobalHotkeyError::WindowCreationFailed(code) => {
                write!(f, "CreateWindowExW failed (Win32 error {code})")
            }
            GlobalHotkeyError::WindowClassFailed(code) => {
                write!(f, "RegisterClassExW failed (Win32 error {code})")
            }
            GlobalHotkeyError::ThreadSpawnFailed(reason) => {
                write!(f, "failed to spawn global-hotkey worker thread: {reason}")
            }
            GlobalHotkeyError::ThreadDied => f.write_str(
                "global-hotkey worker thread exited before reporting a registration result",
            ),
        }
    }
}

impl std::error::Error for GlobalHotkeyError {}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::GlobalHotkeyError;

    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::OnceLock;
    use std::thread::JoinHandle;
    use std::time::Duration;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        PostMessageW, RegisterClassExW, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE,
        WINDOW_STYLE, WM_CLOSE, WM_DESTROY, WM_HOTKEY, WM_USER, WNDCLASSEXW,
    };

    /// Fixed identifier passed to `RegisterHotKey`. We only own one hotkey
    /// per process, so a constant is fine.
    const HOTKEY_ID: i32 = 1;

    /// Custom `WM_USER` message the handle posts to tell the worker thread to
    /// tear everything down. `WM_USER + 1` avoids colliding with system
    /// messages and leaves `WM_USER` itself free for future use.
    const WM_TRACE_STOP: u32 = WM_USER + 1;

    /// Timeout for the bootstrap handshake. Creating a message-only window
    /// and registering a hotkey should take well under a second; two seconds
    /// guards against deadlocks without being user-visible in the fast path.
    const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(2);

    /// Window class name, NUL-terminated UTF-16. Registered once per process
    /// via [`WND_CLASS_REGISTRATION`]; subsequent `GlobalHotkey::register`
    /// calls reuse the same class.
    const WND_CLASS_NAME: &[u16] = &[
        'T' as u16, 'r' as u16, 'a' as u16, 'c' as u16, 'e' as u16, 'G' as u16, 'l' as u16,
        'o' as u16, 'b' as u16, 'a' as u16, 'l' as u16, 'H' as u16, 'o' as u16, 't' as u16,
        'k' as u16, 'e' as u16, 'y' as u16, 'W' as u16, 'n' as u16, 'd' as u16, 'C' as u16,
        'l' as u16, 'a' as u16, 's' as u16, 's' as u16, 0,
    ];

    /// One-shot class registration. Calling `RegisterClassExW` twice with the
    /// same name fails with `ERROR_CLASS_ALREADY_EXISTS` — we gate it behind
    /// a `OnceLock` and cache the result so all subsequent hotkey registrations
    /// reuse the same class atom (or the same failure).
    static WND_CLASS_REGISTRATION: OnceLock<Result<(), u32>> = OnceLock::new();

    /// A registered Windows global hotkey.
    ///
    /// Drop unregisters the hotkey and joins the worker thread. Callers pump
    /// press events through the channel returned by [`Self::events`].
    pub struct GlobalHotkey {
        /// Raw HWND of the message-only window owned by the worker thread.
        /// We store `isize` because `HWND` contains a raw pointer and is not
        /// `Send`; `PostMessageW` is thread-safe so posting from the owning
        /// thread into the worker is sound.
        hwnd_raw: isize,
        thread_handle: Option<JoinHandle<()>>,
        events_rx: Receiver<()>,
    }

    impl GlobalHotkey {
        /// Register the hotkey.
        ///
        /// `vkey` is a Windows virtual-key code (e.g. `VK_N = 0x4E`);
        /// `modifiers` is a bitmask of `MOD_ALT` (`0x0001`) / `MOD_CONTROL`
        /// (`0x0002`) / `MOD_SHIFT` (`0x0004`) / `MOD_WIN` (`0x0008`). These
        /// are the same values that [`trace_core::settings`] already stores.
        ///
        /// Returns an error if the underlying Win32 calls fail — the most
        /// common cause is another process already owning the requested
        /// shortcut.
        pub fn register(vkey: u32, modifiers: u32) -> Result<Self, GlobalHotkeyError> {
            let (events_tx, events_rx) = mpsc::channel::<()>();
            let (boot_tx, boot_rx) = mpsc::channel::<BootstrapMessage>();

            let thread_handle = std::thread::Builder::new()
                .name("trace-global-hotkey".into())
                .spawn(move || worker_main(vkey, modifiers, events_tx, boot_tx))
                .map_err(|e| GlobalHotkeyError::ThreadSpawnFailed(e.to_string()))?;

            match boot_rx.recv_timeout(BOOTSTRAP_TIMEOUT) {
                Ok(BootstrapMessage::Ready { hwnd_raw }) => Ok(GlobalHotkey {
                    hwnd_raw,
                    thread_handle: Some(thread_handle),
                    events_rx,
                }),
                Ok(BootstrapMessage::Failed(err)) => {
                    // Worker handled cleanup; joining here just drains the
                    // thread so we don't leave it dangling.
                    let _ = thread_handle.join();
                    Err(err)
                }
                Err(_) => {
                    // The worker died before or during bootstrap (panic
                    // inside `worker_main`, or both halves of `boot_tx`
                    // dropped before send). Best-effort join so we don't
                    // leave a detached thread; ignore any panic payload.
                    let _ = thread_handle.join();
                    Err(GlobalHotkeyError::ThreadDied)
                }
            }
        }

        /// Returns the receiver for hotkey-press events. Each received `()`
        /// corresponds to one fire of the global hotkey. The receiver stays
        /// live for as long as this handle exists; after [`Drop`] the
        /// sender half is gone and the receiver will yield
        /// [`mpsc::RecvError`] on subsequent calls (not that you can call
        /// it — the handle is gone too).
        pub fn events(&self) -> &Receiver<()> {
            &self.events_rx
        }

        /// Test-only accessor for the raw HWND so integration tests can
        /// post synthetic messages without actually pressing the key. Never
        /// exposed in release builds.
        #[cfg(test)]
        pub(crate) fn hwnd_raw_for_tests(&self) -> isize {
            self.hwnd_raw
        }
    }

    impl Drop for GlobalHotkey {
        fn drop(&mut self) {
            unsafe {
                // Wake the worker's `GetMessageW` so it can leave the loop
                // and run its cleanup path (UnregisterHotKey +
                // DestroyWindow). PostMessageW is documented as thread-safe.
                let hwnd = HWND(self.hwnd_raw as *mut _);
                // Best-effort: if PostMessageW fails (e.g. the worker
                // already tore the window down because of a panic) there
                // is nothing sensible we can do at drop time.
                let _ = PostMessageW(hwnd, WM_TRACE_STOP, WPARAM(0), LPARAM(0));
            }

            if let Some(handle) = self.thread_handle.take() {
                // Ignore panics — `Drop` can't surface errors.
                let _ = handle.join();
            }
        }
    }

    /// Message sent from the worker thread back to `register` during
    /// bootstrap. Exactly one is produced per worker lifetime.
    enum BootstrapMessage {
        Ready { hwnd_raw: isize },
        Failed(GlobalHotkeyError),
    }

    /// Entry point for the worker thread. Creates the window class (once
    /// per process), the message-only window, registers the hotkey, reports
    /// success, then runs the message pump until a stop message arrives.
    fn worker_main(
        vkey: u32,
        modifiers: u32,
        events_tx: Sender<()>,
        boot_tx: Sender<BootstrapMessage>,
    ) {
        // 1. Ensure the window class exists. Cache the result so that the
        //    second hotkey registration in the same process doesn't pay the
        //    RegisterClassExW cost again.
        let class_result = WND_CLASS_REGISTRATION.get_or_init(register_class);
        if let Err(code) = class_result {
            let _ = boot_tx.send(BootstrapMessage::Failed(
                GlobalHotkeyError::WindowClassFailed(*code),
            ));
            return;
        }

        // 2. Create the message-only window. `HWND_MESSAGE` parents the
        //    window into the message-only window tree so it never appears
        //    on screen, in Alt+Tab, etc.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(WND_CLASS_NAME.as_ptr()),
                PCWSTR(std::ptr::null()),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                None,
                None,
            )
        };

        let hwnd = match hwnd {
            Ok(hwnd) if !hwnd.0.is_null() => hwnd,
            Ok(_) => {
                let _ = boot_tx.send(BootstrapMessage::Failed(
                    GlobalHotkeyError::WindowCreationFailed(last_error_code()),
                ));
                return;
            }
            Err(_) => {
                let _ = boot_tx.send(BootstrapMessage::Failed(
                    GlobalHotkeyError::WindowCreationFailed(last_error_code()),
                ));
                return;
            }
        };

        // 3. Register the hotkey against the message-only window.
        let hotkey_result =
            unsafe { RegisterHotKey(hwnd, HOTKEY_ID, HOT_KEY_MODIFIERS(modifiers), vkey) };
        if hotkey_result.is_err() {
            let err_code = last_error_code();
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            let _ = boot_tx.send(BootstrapMessage::Failed(
                GlobalHotkeyError::RegistrationFailed(err_code),
            ));
            return;
        }

        // 4. Signal success. HWND isn't Send, so we smuggle it as isize.
        let hwnd_raw = hwnd.0 as isize;
        if boot_tx.send(BootstrapMessage::Ready { hwnd_raw }).is_err() {
            // Caller hung up before we could report success; clean up.
            unsafe {
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
                let _ = DestroyWindow(hwnd);
            }
            return;
        }

        // 5. Pump messages until we get a stop signal.
        run_message_loop(hwnd, &events_tx);

        // 6. Tear everything down.
        unsafe {
            let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
            let _ = DestroyWindow(hwnd);
        }
    }

    fn register_class() -> Result<(), u32> {
        let hinstance: HINSTANCE = unsafe {
            match GetModuleHandleW(PCWSTR(std::ptr::null())) {
                Ok(module) => HINSTANCE(module.0),
                Err(_) => HINSTANCE(std::ptr::null_mut()),
            }
        };

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            lpszClassName: PCWSTR(WND_CLASS_NAME.as_ptr()),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassExW(&wnd_class) };
        if atom == 0 {
            Err(last_error_code())
        } else {
            Ok(())
        }
    }

    fn run_message_loop(hwnd: HWND, events_tx: &Sender<()>) {
        let mut msg = MSG::default();
        loop {
            // Filter on `hwnd` so we only pick up messages for our
            // message-only window — both `WM_HOTKEY` (because we registered
            // the hotkey against this HWND) and `WM_TRACE_STOP` (posted via
            // `PostMessageW(hwnd, ...)` from Drop) arrive here.
            let got = unsafe { GetMessageW(&mut msg, hwnd, 0, 0) };
            // GetMessageW returns 0 on WM_QUIT, -1 on error. We never
            // PostQuitMessage, so a 0 here means something upstream nuked
            // our queue — bail out.
            if got.0 <= 0 {
                break;
            }

            match msg.message {
                WM_HOTKEY => {
                    // The receiver being dropped means the handle is gone
                    // (usually impossible because Drop posts WM_TRACE_STOP
                    // first), so just exit the pump.
                    if events_tx.send(()).is_err() {
                        break;
                    }
                }
                WM_TRACE_STOP | WM_CLOSE | WM_DESTROY => break,
                _ => unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
            }
        }
    }

    /// WndProc for the message-only window. We route most messages to
    /// `DefWindowProcW`; `WM_HOTKEY` is delivered through `GetMessageW` to
    /// the worker pump, not to WndProc, so this is mostly a formality.
    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn last_error_code() -> u32 {
        unsafe { GetLastError().0 }
    }
}

#[cfg(windows)]
pub use imp::GlobalHotkey;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Cross-platform tests (run on Mac and Windows) ---------------------

    #[test]
    fn global_hotkey_error_display_is_human_readable() {
        let cases: [GlobalHotkeyError; 5] = [
            GlobalHotkeyError::RegistrationFailed(1409),
            GlobalHotkeyError::WindowCreationFailed(5),
            GlobalHotkeyError::WindowClassFailed(1410),
            GlobalHotkeyError::ThreadSpawnFailed("nope".into()),
            GlobalHotkeyError::ThreadDied,
        ];

        for err in &cases {
            let rendered = err.to_string();
            assert!(
                !rendered.is_empty(),
                "Display impl produced empty string for {err:?}"
            );
            // A useful message is more than the variant name.
            assert!(
                rendered.len() >= 10,
                "Display impl too terse for {err:?}: {rendered:?}"
            );
        }
    }

    #[test]
    fn global_hotkey_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<GlobalHotkeyError>();
    }

    #[test]
    fn global_hotkey_error_debug_includes_variant_name() {
        let rendered = format!("{:?}", GlobalHotkeyError::RegistrationFailed(5));
        assert!(
            rendered.contains("RegistrationFailed"),
            "Debug output should name the variant, got {rendered:?}"
        );
    }

    // ---- Windows-only integration tests ------------------------------------
    //
    // These need a real desktop session (RegisterHotKey fails without one)
    // and so are marked `#[ignore]`. Run locally on Windows with:
    //
    //     cargo test -p trace-platform -- --ignored
    //
    // They're compile-checked on every platform via
    // `cargo check --target x86_64-pc-windows-*` in CI.

    #[cfg(windows)]
    mod windows_only {
        use super::super::*;
        use std::time::Duration;
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_HOTKEY};

        const VK_F24: u32 = 0x87;
        const MOD_CONTROL: u32 = 0x0002;

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn register_and_drop_is_clean() {
            let hotkey = GlobalHotkey::register(VK_F24, MOD_CONTROL)
                .expect("Ctrl+F24 should be registerable");
            drop(hotkey);
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn synthetic_wm_hotkey_sends_event_to_channel() {
            let hotkey = GlobalHotkey::register(VK_F24, MOD_CONTROL)
                .expect("Ctrl+F24 should be registerable");

            // Reach into the handle's stored HWND to simulate a hotkey
            // press without actually hitting the keyboard.
            let hwnd_raw = hotkey.hwnd_raw_for_tests();
            unsafe {
                PostMessageW(HWND(hwnd_raw as *mut _), WM_HOTKEY, WPARAM(0), LPARAM(0))
                    .expect("PostMessageW should succeed on a live message-only window");
            }

            hotkey
                .events()
                .recv_timeout(Duration::from_secs(1))
                .expect("synthetic WM_HOTKEY should produce a channel event");
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn duplicate_registration_returns_error() {
            let _first = GlobalHotkey::register(VK_F24, MOD_CONTROL)
                .expect("first registration should succeed");

            let second = GlobalHotkey::register(VK_F24, MOD_CONTROL);
            match second {
                Err(GlobalHotkeyError::RegistrationFailed(_)) => {}
                Err(other) => {
                    panic!("second registration should fail with RegistrationFailed, got {other:?}")
                }
                Ok(_) => {
                    panic!("second registration should fail with RegistrationFailed, got Ok(...)")
                }
            }
        }
    }
}

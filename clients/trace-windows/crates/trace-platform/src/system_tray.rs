//! Windows system tray (notification area) integration.
//!
//! On Windows this module registers a single `Shell_NotifyIconW` tray icon
//! attached to a hidden worker window that runs on a dedicated worker thread.
//! Tray interactions (menu commands, double-clicks) are forwarded to the
//! caller through a [`std::sync::mpsc::Receiver<TrayEvent>`]. Dropping the
//! returned [`SystemTray`] removes the icon and joins the worker thread.
//!
//! This is the Rust port of the `setupStatusItem()` path in
//! `Sources/Trace/App/AppDelegate.swift`. The macOS reference ships a
//! hard-coded-English menu with three items — "New Note", "Settings…" and
//! "Quit Trace" — and we mirror that structure, adding proper
//! localization via [`trace_core::L10n`].
//!
//! ## Why a hidden regular window, not `HWND_MESSAGE`?
//!
//! The sibling [`crate::global_hotkey`] module uses a message-only window
//! (`HWND_MESSAGE`) because nothing broadcasts interesting messages to it.
//! Tray icons are different: when Explorer restarts (after a crash, a shell
//! extension upgrade, or `taskkill /f /im explorer.exe` followed by a relaunch)
//! it re-announces itself by broadcasting the registered `TaskbarCreated`
//! message to all top-level windows — but **message-only windows do not
//! receive shell broadcasts**. If we used `HWND_MESSAGE` the tray icon would
//! silently vanish after every Explorer restart. We therefore create a
//! hidden [`WS_POPUP`] window parented to the desktop so it's still a
//! top-level window for broadcast purposes, never shown on screen, and
//! invisible to Alt-Tab.
//!
//! Non-Windows targets expose only [`SystemTrayError`] and [`TrayEvent`] so
//! the crate still compiles on macOS/Linux developer machines.
//!
//! # Example (Windows-only)
//!
//! ```no_run
//! # #[cfg(windows)]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use trace_core::models::Language;
//! use trace_platform::system_tray::{SystemTray, TrayEvent};
//!
//! let tray = SystemTray::install("Trace".to_string(), Language::Zh)?;
//!
//! // Blocks until the user picks something from the tray menu.
//! match tray.events().recv()? {
//!     TrayEvent::NewNote => { /* show capture panel */ }
//!     TrayEvent::OpenSettings => { /* show settings */ }
//!     TrayEvent::Quit => { /* quit the app */ }
//! }
//! # Ok(())
//! # }
//! # #[cfg(not(windows))]
//! # fn main() {}
//! ```

use std::fmt;

/// Error returned when installing or running the system tray.
#[derive(Debug)]
pub enum SystemTrayError {
    /// `Shell_NotifyIconW(NIM_ADD, ...)` returned `FALSE`. Inner value is the
    /// Win32 error code from `GetLastError` at the moment of the failure.
    /// Typical cause: the shell isn't ready (very early boot) or Explorer has
    /// crashed and hasn't restarted yet.
    ShellNotifyFailed(u32),
    /// `CreateWindowExW` for the hidden tray worker window failed.
    WindowCreationFailed(u32),
    /// `RegisterClassExW` failed.
    WindowClassFailed(u32),
    /// Loading the tray icon from the embedded PNG bytes failed
    /// (`CreateIconFromResourceEx` / `LoadImageW` returned an error).
    IconLoadFailed(u32),
    /// The worker thread could not be spawned.
    ThreadSpawnFailed(String),
    /// The worker thread panicked or exited without sending an install
    /// result back to the caller.
    ThreadDied,
}

impl fmt::Display for SystemTrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemTrayError::ShellNotifyFailed(code) => write!(
                f,
                "Shell_NotifyIconW failed (Win32 error {code}); the shell may not be ready"
            ),
            SystemTrayError::WindowCreationFailed(code) => {
                write!(f, "CreateWindowExW failed (Win32 error {code})")
            }
            SystemTrayError::WindowClassFailed(code) => {
                write!(f, "RegisterClassExW failed (Win32 error {code})")
            }
            SystemTrayError::IconLoadFailed(code) => {
                write!(
                    f,
                    "failed to load tray icon from embedded PNG (Win32 error {code})"
                )
            }
            SystemTrayError::ThreadSpawnFailed(reason) => {
                write!(f, "failed to spawn system-tray worker thread: {reason}")
            }
            SystemTrayError::ThreadDied => {
                f.write_str("system-tray worker thread exited before reporting an install result")
            }
        }
    }
}

impl std::error::Error for SystemTrayError {}

/// Events emitted by the tray icon.
///
/// This is intentionally a small, menu-focused enum — it tracks the three
/// actions the macOS reference menu exposes. Left-click and right-click both
/// open the context menu (standard Windows tray UX, no dedicated variant
/// needed); double-click is a fast path to [`TrayEvent::NewNote`] so users
/// don't have to go through the menu for the most common action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// User chose "New Note" from the context menu, or double-clicked the
    /// tray icon (Windows convention: double-click runs the default action).
    NewNote,
    /// User chose "Open Settings" from the context menu.
    OpenSettings,
    /// User chose "Quit <AppName>" from the context menu.
    Quit,
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::{SystemTrayError, TrayEvent};

    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::OnceLock;
    use std::thread::JoinHandle;
    use std::time::Duration;

    use trace_core::models::Language;
    use trace_core::L10n;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
        DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
        PostMessageW, RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow,
        TrackPopupMenu, TranslateMessage, HICON, HMENU, LR_DEFAULTCOLOR, MF_SEPARATOR, MF_STRING,
        MSG, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_CLOSE, WM_COMMAND, WM_DESTROY,
        WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_USER, WNDCLASSEXW, WS_POPUP,
    };

    /// Constant ID of our single tray icon. The value doesn't matter as long
    /// as it's stable for the lifetime of the process — `Shell_NotifyIconW`
    /// uses `(hWnd, uID)` as the identity tuple.
    const TRAY_ICON_ID: u32 = 1;

    /// Custom callback message Shell posts to our WndProc for mouse events on
    /// the tray icon. Must sit in the `WM_USER` range. `WM_USER + 1` is
    /// reserved by [`crate::global_hotkey`] for its stop signal, so we claim
    /// the next slot.
    const WM_TRACE_TRAY: u32 = WM_USER + 2;

    /// Custom message the handle posts to tell the worker thread to tear
    /// everything down.
    const WM_TRACE_STOP: u32 = WM_USER + 3;

    /// Command IDs for the context-menu entries. Stable across refreshes
    /// because we rebuild the menu from scratch each time.
    const CMD_NEW_NOTE: u32 = 1;
    const CMD_SETTINGS: u32 = 2;
    const CMD_QUIT: u32 = 3;

    /// Version passed to `CreateIconFromResourceEx`. `0x00030000` selects
    /// version-3 icon resources, which covers every PNG/ICO format shipped
    /// since Vista — in particular, high-color PNG-in-ICO icons. Older
    /// `0x00020000` would work too but is unnecessarily conservative.
    const ICON_VERSION: u32 = 0x00030000;

    /// Timeout for the bootstrap handshake. See [`crate::global_hotkey`] for
    /// the same rationale.
    const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(2);

    /// Embedded 32x32 PNG used as the tray icon. `include_bytes!` walks up
    /// from `crates/trace-platform/src/` → `crates/trace-platform/` →
    /// `crates/` → `trace-windows/` → `assets/`.
    const TRAY_ICON_PNG: &[u8] = include_bytes!("../../../assets/trace-32.png");

    /// Window-class name, NUL-terminated UTF-16. Kept distinct from the
    /// global-hotkey class so the two modules can coexist in the same
    /// process without stepping on each other's `RegisterClassExW`.
    const WND_CLASS_NAME: &[u16] = &[
        'T' as u16, 'r' as u16, 'a' as u16, 'c' as u16, 'e' as u16, 'S' as u16, 'y' as u16,
        's' as u16, 't' as u16, 'e' as u16, 'm' as u16, 'T' as u16, 'r' as u16, 'a' as u16,
        'y' as u16, 'W' as u16, 'n' as u16, 'd' as u16, 'C' as u16, 'l' as u16, 'a' as u16,
        's' as u16, 's' as u16, 0,
    ];

    /// UTF-16 NUL-terminated name of the `TaskbarCreated` broadcast, passed
    /// to `RegisterWindowMessageW`. Kept as a const so lookups are cheap.
    const TASKBAR_CREATED_NAME: &[u16] = &[
        'T' as u16, 'a' as u16, 's' as u16, 'k' as u16, 'b' as u16, 'a' as u16, 'r' as u16,
        'C' as u16, 'r' as u16, 'e' as u16, 'a' as u16, 't' as u16, 'e' as u16, 'd' as u16, 0,
    ];

    /// One-shot class registration. Calling `RegisterClassExW` twice with the
    /// same name fails with `ERROR_CLASS_ALREADY_EXISTS` — we gate it behind
    /// a `OnceLock` and cache the result so all subsequent tray installations
    /// reuse the same class atom (or the same failure).
    ///
    /// The failure branch is intentionally permanent for the lifetime of the
    /// process. `RegisterClassExW` failures are nearly always configuration-
    /// or resource-limit errors that don't self-heal; caching the error code
    /// lets subsequent `install()` calls fail fast with the same diagnostic
    /// instead of retrying and potentially making things worse.
    static WND_CLASS_REGISTRATION: OnceLock<Result<(), u32>> = OnceLock::new();

    /// RAII wrapper for `HMENU` that calls `DestroyMenu` on drop. Guarantees
    /// the menu is released even if a future edit introduces an early return
    /// or panic between `TrackPopupMenu` and the explicit `DestroyMenu` call.
    struct MenuGuard(HMENU);

    impl Drop for MenuGuard {
        fn drop(&mut self) {
            if !self.0.0.is_null() {
                unsafe {
                    let _ = DestroyMenu(self.0);
                }
            }
        }
    }

    /// An installed Windows tray icon.
    ///
    /// Drop removes the icon and joins the worker thread. Callers consume
    /// events through the channel returned by [`Self::events`].
    pub struct SystemTray {
        /// Raw HWND of the worker-owned hidden window. Stored as `isize`
        /// because `HWND` holds a raw pointer and is not `Send`;
        /// `PostMessageW` is documented as thread-safe.
        hwnd_raw: isize,
        thread_handle: Option<JoinHandle<()>>,
        events_rx: Receiver<TrayEvent>,
    }

    impl SystemTray {
        /// Install the tray icon.
        ///
        /// `app_name` is interpolated into the "Quit <name>" menu entry;
        /// `language` is captured at install time and controls the static
        /// menu strings. (The Mac reference rebuilds its menu when the
        /// language changes — a later phase can add dynamic refresh here;
        /// Phase 7 accepts install-time fixing.)
        pub fn install(app_name: String, language: Language) -> Result<Self, SystemTrayError> {
            let (events_tx, events_rx) = mpsc::channel::<TrayEvent>();
            let (boot_tx, boot_rx) = mpsc::channel::<BootstrapMessage>();

            let thread_handle = std::thread::Builder::new()
                .name("trace-system-tray".into())
                .spawn(move || worker_main(app_name, language, events_tx, boot_tx))
                .map_err(|e| SystemTrayError::ThreadSpawnFailed(e.to_string()))?;

            match boot_rx.recv_timeout(BOOTSTRAP_TIMEOUT) {
                Ok(BootstrapMessage::Ready { hwnd_raw }) => Ok(SystemTray {
                    hwnd_raw,
                    thread_handle: Some(thread_handle),
                    events_rx,
                }),
                Ok(BootstrapMessage::Failed(err)) => {
                    // Worker handled cleanup; join just drains the thread.
                    let _ = thread_handle.join();
                    Err(err)
                }
                Err(_) => {
                    // Detach rather than join — see global_hotkey.rs for
                    // the identical rationale.
                    drop(thread_handle);
                    Err(SystemTrayError::ThreadDied)
                }
            }
        }

        /// Returns the receiver for tray events. Each received
        /// [`TrayEvent`] corresponds to one user interaction (menu command
        /// or double-click). The receiver stays live for the handle's
        /// lifetime.
        pub fn events(&self) -> &Receiver<TrayEvent> {
            &self.events_rx
        }

        /// Test-only accessor for the raw HWND so integration tests can
        /// post synthetic messages without a real tray interaction.
        #[cfg(test)]
        pub(crate) fn hwnd_raw_for_tests(&self) -> isize {
            self.hwnd_raw
        }
    }

    impl Drop for SystemTray {
        fn drop(&mut self) {
            // Idempotent: if the worker already tore things down (e.g. via
            // a panic earlier), PostMessageW will fail silently and
            // thread_handle.take() returns None on a second drop.
            unsafe {
                let hwnd = HWND(self.hwnd_raw as *mut _);
                // Best effort — a failure here just means the worker is
                // already gone, which is exactly what we want at drop time.
                let _ = PostMessageW(hwnd, WM_TRACE_STOP, WPARAM(0), LPARAM(0));
            }

            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }
        }
    }

    /// Message sent from the worker thread back to `install` during
    /// bootstrap. Exactly one is produced per worker lifetime.
    enum BootstrapMessage {
        Ready { hwnd_raw: isize },
        Failed(SystemTrayError),
    }

    // Thread-local state for the worker. Storing state in the window's
    // user-data slot would be more idiomatic, but the sibling
    // `global_hotkey` module keeps things simple by passing a `Sender`
    // through closures and we follow suit — the WndProc here is more
    // complex so we stash the state in a thread-local cell.
    thread_local! {
        static WORKER_STATE: std::cell::RefCell<Option<WorkerState>> =
            const { std::cell::RefCell::new(None) };
    }

    struct WorkerState {
        events_tx: Sender<TrayEvent>,
        /// Cached app-name-interpolated "Quit X" string so we don't
        /// re-format on every menu open.
        quit_label_utf16: Vec<u16>,
        /// Cached "New Note" / "Open Settings" strings.
        new_note_utf16: Vec<u16>,
        open_settings_utf16: Vec<u16>,
        /// Cached NOTIFYICONDATAW so `WM_TASKBARCREATED` can re-add the
        /// icon with the same payload.
        nid: NOTIFYICONDATAW,
    }

    /// Entry point for the worker thread.
    fn worker_main(
        app_name: String,
        language: Language,
        events_tx: Sender<TrayEvent>,
        boot_tx: Sender<BootstrapMessage>,
    ) {
        // 1. Class registration (once per process).
        let class_result = WND_CLASS_REGISTRATION.get_or_init(register_class);
        if let Err(code) = class_result {
            let _ = boot_tx.send(BootstrapMessage::Failed(
                SystemTrayError::WindowClassFailed(*code),
            ));
            return;
        }

        // 2. Create the hidden popup window. Must NOT use HWND_MESSAGE
        //    because message-only windows don't receive the
        //    `TaskbarCreated` broadcast. A zero-sized invisible popup
        //    parented to the desktop works as a top-level window for
        //    broadcast purposes while staying off-screen.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(WND_CLASS_NAME.as_ptr()),
                PCWSTR(std::ptr::null()),
                WS_POPUP, // no WS_VISIBLE — window never appears
                0,
                0,
                0,
                0,
                HWND(std::ptr::null_mut()), // desktop parent
                None,
                None,
                None,
            )
        };

        let hwnd = match hwnd {
            Ok(hwnd) if !hwnd.0.is_null() => hwnd,
            Ok(_) => {
                let _ = boot_tx.send(BootstrapMessage::Failed(
                    SystemTrayError::WindowCreationFailed(last_error_code()),
                ));
                return;
            }
            Err(_) => {
                let _ = boot_tx.send(BootstrapMessage::Failed(
                    SystemTrayError::WindowCreationFailed(last_error_code()),
                ));
                return;
            }
        };

        // 3. Load the tray icon from the embedded PNG bytes.
        let hicon = match unsafe {
            CreateIconFromResourceEx(TRAY_ICON_PNG, true, ICON_VERSION, 32, 32, LR_DEFAULTCOLOR)
        } {
            Ok(icon) if !icon.0.is_null() => icon,
            Ok(_) => {
                let err_code = last_error_code();
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                let _ = boot_tx.send(BootstrapMessage::Failed(SystemTrayError::IconLoadFailed(
                    err_code,
                )));
                return;
            }
            Err(_) => {
                let err_code = last_error_code();
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                let _ = boot_tx.send(BootstrapMessage::Failed(SystemTrayError::IconLoadFailed(
                    err_code,
                )));
                return;
            }
        };

        // 4. Register the shell-broadcast message ID so we can recreate the
        //    icon if Explorer restarts. `RegisterWindowMessageW` returns 0 on
        //    failure (documented); the message loop guards against this value
        //    to avoid spuriously matching `WM_NULL` (also 0) and re-adding
        //    the icon after every menu close.
        let taskbar_created_msg =
            unsafe { RegisterWindowMessageW(PCWSTR(TASKBAR_CREATED_NAME.as_ptr())) };

        // 5. Build NOTIFYICONDATAW and register the icon.
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRACE_TRAY,
            hIcon: hicon,
            ..Default::default()
        };
        write_tooltip(&mut nid.szTip, &app_name);

        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
        if !added.as_bool() {
            let err_code = last_error_code();
            unsafe {
                let _ = DestroyIcon(hicon);
                let _ = DestroyWindow(hwnd);
            }
            let _ = boot_tx.send(BootstrapMessage::Failed(
                SystemTrayError::ShellNotifyFailed(err_code),
            ));
            return;
        }

        // 6. Store the worker state so WndProc can reach it.
        let state = WorkerState {
            events_tx,
            quit_label_utf16: widen_nul_terminated(&L10n::quit(language, &app_name)),
            new_note_utf16: widen_nul_terminated(L10n::new_note(language)),
            open_settings_utf16: widen_nul_terminated(L10n::open_settings(language)),
            nid,
        };
        WORKER_STATE.with(|cell| *cell.borrow_mut() = Some(state));

        // 7. Signal success.
        let hwnd_raw = hwnd.0 as isize;
        if boot_tx.send(BootstrapMessage::Ready { hwnd_raw }).is_err() {
            // Caller hung up before we could report success; clean up.
            cleanup(hwnd, hicon);
            WORKER_STATE.with(|cell| *cell.borrow_mut() = None);
            return;
        }

        // 8. Pump messages until stop arrives.
        run_message_loop(taskbar_created_msg);

        // 9. Tear down.
        cleanup(hwnd, hicon);
        WORKER_STATE.with(|cell| *cell.borrow_mut() = None);
    }

    fn cleanup(hwnd: HWND, hicon: HICON) {
        // Rebuild a minimal NOTIFYICONDATAW for NIM_DELETE — only (hWnd, uID)
        // are consulted.
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            let _ = DestroyIcon(hicon);
            let _ = DestroyWindow(hwnd);
        }
    }

    fn register_class() -> Result<(), u32> {
        let hinstance: HINSTANCE = unsafe {
            GetModuleHandleW(PCWSTR(std::ptr::null()))
                .map(|module| HINSTANCE(module.0))
                .map_err(|_| last_error_code())?
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

    fn run_message_loop(taskbar_created_msg: u32) {
        let mut msg = MSG::default();
        loop {
            // NULL HWND filter — WM_TASKBARCREATED arrives as a broadcast
            // and might not always carry our hwnd in msg.hwnd, so we accept
            // everything on this thread and dispatch based on message id.
            let got = unsafe { GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0) };
            if got.0 <= 0 {
                break;
            }

            // Short-circuit our custom stop message here so we never race
            // with TranslateMessage/DispatchMessageW doing something
            // unexpected with it.
            if msg.message == WM_TRACE_STOP || msg.message == WM_CLOSE || msg.message == WM_DESTROY
            {
                break;
            }

            // WM_TASKBARCREATED is a dynamically-registered message (not a
            // compile-time const) so it needs the value we learned at
            // startup. Guard against `taskbar_created_msg == 0` because
            // `RegisterWindowMessageW` returns 0 on failure and `WM_NULL`
            // is also 0 — we post `WM_NULL` to ourselves after every
            // `TrackPopupMenu` call, so an unchecked match would re-add
            // the tray icon on every menu close.
            if taskbar_created_msg != 0 && msg.message == taskbar_created_msg {
                re_add_tray_icon();
                continue;
            }

            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Re-announce the tray icon to a freshly-restarted Explorer.
    /// Called from the worker thread only.
    fn re_add_tray_icon() {
        WORKER_STATE.with(|cell| {
            if let Some(state) = cell.borrow().as_ref() {
                unsafe {
                    let _ = Shell_NotifyIconW(NIM_ADD, &state.nid);
                }
            }
        });
    }

    /// WndProc for the hidden tray window. Runs on the worker thread.
    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            // Callback from Shell when the user mouses over the tray icon.
            // The specific mouse event lives in LPARAM's low word.
            WM_TRACE_TRAY => {
                let mouse_event = (lparam.0 as u32) & 0xFFFF;
                handle_tray_mouse(hwnd, mouse_event);
                LRESULT(0)
            }
            // Context-menu command dispatch (fires both from TrackPopupMenu
            // with TPM_RETURNCMD returning the id AND from synthetic
            // PostMessageW calls in tests). TPM_RETURNCMD means
            // TrackPopupMenu returns the id directly rather than posting
            // WM_COMMAND, but we still handle the posted form so tests can
            // exercise the dispatch path without mousing through a menu.
            WM_COMMAND => {
                let cmd_id = (wparam.0 as u32) & 0xFFFF;
                dispatch_command(cmd_id);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    fn handle_tray_mouse(hwnd: HWND, mouse_event: u32) {
        match mouse_event {
            WM_LBUTTONUP | WM_RBUTTONUP => show_context_menu(hwnd),
            WM_LBUTTONDBLCLK => emit(TrayEvent::NewNote),
            _ => {}
        }
    }

    fn show_context_menu(hwnd: HWND) {
        // Build a fresh popup menu each time so language/app-name changes
        // between install and invocation (should any later phase add live
        // refresh) just work. The `MenuGuard` ensures the menu is destroyed
        // on every return path, including any future panic between
        // `TrackPopupMenu` and the end of the function.
        let menu = match unsafe { CreatePopupMenu() } {
            Ok(m) => MenuGuard(m),
            Err(_) => return, // Menu couldn't be created; nothing sensible to do.
        };

        WORKER_STATE.with(|cell| {
            let state_ref = cell.borrow();
            let state = match state_ref.as_ref() {
                Some(s) => s,
                None => return,
            };

            unsafe {
                let _ = AppendMenuW(
                    menu.0,
                    MF_STRING,
                    CMD_NEW_NOTE as usize,
                    PCWSTR(state.new_note_utf16.as_ptr()),
                );
                let _ = AppendMenuW(menu.0, MF_SEPARATOR, 0, PCWSTR(std::ptr::null()));
                let _ = AppendMenuW(
                    menu.0,
                    MF_STRING,
                    CMD_SETTINGS as usize,
                    PCWSTR(state.open_settings_utf16.as_ptr()),
                );
                let _ = AppendMenuW(menu.0, MF_SEPARATOR, 0, PCWSTR(std::ptr::null()));
                let _ = AppendMenuW(
                    menu.0,
                    MF_STRING,
                    CMD_QUIT as usize,
                    PCWSTR(state.quit_label_utf16.as_ptr()),
                );
            }
        });

        // Documented Shell workarounds: SetForegroundWindow before the
        // menu so clicking outside it dismisses it; PostMessageW(WM_NULL)
        // after so Windows doesn't leave stale state behind.
        let mut pt = POINT::default();
        unsafe {
            let _ = SetForegroundWindow(hwnd);
            let _ = GetCursorPos(&mut pt);
        }

        let cmd = unsafe {
            TrackPopupMenu(
                menu.0,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                pt.x,
                pt.y,
                0,
                hwnd,
                None,
            )
        };

        unsafe {
            let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        }
        // `menu` drops at end of scope, which calls `DestroyMenu` — no
        // explicit cleanup needed here.

        // TPM_RETURNCMD returns the selected command id, or 0 if the menu
        // was dismissed without a selection.
        let cmd_id = cmd.0 as u32;
        if cmd_id != 0 {
            dispatch_command(cmd_id);
        }
    }

    fn dispatch_command(cmd_id: u32) {
        let event = match cmd_id {
            CMD_NEW_NOTE => TrayEvent::NewNote,
            CMD_SETTINGS => TrayEvent::OpenSettings,
            CMD_QUIT => TrayEvent::Quit,
            _ => return,
        };
        emit(event);
    }

    fn emit(event: TrayEvent) {
        WORKER_STATE.with(|cell| {
            if let Some(state) = cell.borrow().as_ref() {
                // If the receiver is gone the handle was dropped; the
                // next `GetMessageW` iteration will see our stop message
                // or the message-loop will naturally bail out.
                let _ = state.events_tx.send(event);
            }
        });
    }

    /// Writes a UTF-16 NUL-terminated copy of `s` into `dest`, truncating
    /// to `dest.len() - 1` u16 code units to leave room for the NUL.
    /// Safe for empty strings (writes only the NUL).
    fn write_tooltip(dest: &mut [u16; 128], s: &str) {
        let wide: Vec<u16> = s.encode_utf16().collect();
        let max = dest.len() - 1;
        let copy_len = wide.len().min(max);
        dest[..copy_len].copy_from_slice(&wide[..copy_len]);
        dest[copy_len] = 0;
    }

    /// UTF-16 NUL-terminated widening helper for menu strings. The returned
    /// `Vec<u16>` must outlive any `PCWSTR` pointing into it; we keep the
    /// vectors in [`WorkerState`] exactly for that reason.
    fn widen_nul_terminated(s: &str) -> Vec<u16> {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        v
    }

    fn last_error_code() -> u32 {
        unsafe { GetLastError().0 }
    }
}

#[cfg(windows)]
pub use imp::SystemTray;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Cross-platform tests (run on Mac and Windows) ---------------------

    #[test]
    fn system_tray_error_display_is_human_readable() {
        use SystemTrayError::*;
        let cases: [(SystemTrayError, &str); 6] = [
            (ShellNotifyFailed(5), "ShellNotifyFailed"),
            (WindowCreationFailed(6), "WindowCreationFailed"),
            (WindowClassFailed(87), "WindowClassFailed"),
            (IconLoadFailed(8), "IconLoadFailed"),
            (ThreadSpawnFailed("nope".into()), "ThreadSpawnFailed"),
            (ThreadDied, "ThreadDied"),
        ];
        for (err, variant_name) in cases {
            let rendered = err.to_string();
            assert!(
                !rendered.is_empty(),
                "Display should not be empty for {variant_name}: {rendered:?}"
            );
            assert!(
                !rendered.contains(variant_name),
                "Display should be human-readable, not leak the debug variant name ({variant_name}): {rendered:?}"
            );
        }
    }

    #[test]
    fn system_tray_error_display_includes_error_codes() {
        assert!(SystemTrayError::ShellNotifyFailed(1409)
            .to_string()
            .contains("1409"));
        assert!(SystemTrayError::WindowCreationFailed(6)
            .to_string()
            .contains('6'));
        assert!(SystemTrayError::IconLoadFailed(42)
            .to_string()
            .contains("42"));
        assert!(SystemTrayError::ThreadSpawnFailed("pool exhausted".into())
            .to_string()
            .contains("pool exhausted"));
    }

    #[test]
    fn system_tray_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<SystemTrayError>();
    }

    #[test]
    fn system_tray_error_debug_includes_variant_name() {
        let rendered = format!("{:?}", SystemTrayError::ShellNotifyFailed(5));
        assert!(
            rendered.contains("ShellNotifyFailed"),
            "Debug output should name the variant, got {rendered:?}"
        );
    }

    #[test]
    fn tray_event_is_copy_clone_eq() {
        fn assert_copy<T: Copy>() {}
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}
        fn assert_eq<T: Eq + PartialEq>() {}

        assert_copy::<TrayEvent>();
        assert_clone::<TrayEvent>();
        assert_debug::<TrayEvent>();
        assert_eq::<TrayEvent>();

        // Runtime sanity: distinct variants compare unequal, same variants
        // compare equal — guards against accidental `PartialEq` derives
        // that always return true.
        let a = TrayEvent::NewNote;
        let b = TrayEvent::NewNote;
        let c = TrayEvent::OpenSettings;
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ---- Windows-only integration tests ------------------------------------
    //
    // Installing a real tray icon needs an interactive session (Shell must
    // be running and the user must have a desktop), so we mark them
    // `#[ignore]`. Run locally on Windows with:
    //
    //     cargo test -p trace-platform -- --ignored
    //
    // They're compile-checked on every platform via
    // `cargo check --target x86_64-pc-windows-*` in CI.

    #[cfg(windows)]
    mod windows_only {
        use super::super::*;
        use std::time::Duration;
        use trace_core::models::Language;
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            PostMessageW, WM_COMMAND, WM_LBUTTONDBLCLK, WM_USER,
        };

        // Mirror the internal constants — keeping them in sync is fine
        // because the tests live next to the module that defines them.
        const WM_TRACE_TRAY: u32 = WM_USER + 2;
        const CMD_NEW_NOTE: u32 = 1;

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn install_and_drop_is_clean() {
            let tray = SystemTray::install("Trace".to_string(), Language::En)
                .expect("tray should install on an interactive desktop");
            drop(tray);
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn synthetic_menu_command_sends_event() {
            let tray = SystemTray::install("Trace".to_string(), Language::En)
                .expect("tray should install on an interactive desktop");

            let hwnd_raw = tray.hwnd_raw_for_tests();
            unsafe {
                // MAKEWPARAM(CMD_NEW_NOTE, 0) — low word = id, high word = 0.
                let wparam = WPARAM(CMD_NEW_NOTE as usize);
                PostMessageW(HWND(hwnd_raw as *mut _), WM_COMMAND, wparam, LPARAM(0))
                    .expect("PostMessageW should succeed on a live tray window");
            }

            let event = tray
                .events()
                .recv_timeout(Duration::from_secs(1))
                .expect("synthetic WM_COMMAND should produce a channel event");
            assert_eq!(event, TrayEvent::NewNote);
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn double_click_emits_new_note() {
            let tray = SystemTray::install("Trace".to_string(), Language::En)
                .expect("tray should install on an interactive desktop");

            let hwnd_raw = tray.hwnd_raw_for_tests();
            unsafe {
                // LPARAM low word carries the mouse event for tray callback.
                let lparam = LPARAM(WM_LBUTTONDBLCLK as isize);
                PostMessageW(HWND(hwnd_raw as *mut _), WM_TRACE_TRAY, WPARAM(0), lparam)
                    .expect("PostMessageW should succeed on a live tray window");
            }

            let event = tray
                .events()
                .recv_timeout(Duration::from_secs(1))
                .expect("synthetic double-click should produce a channel event");
            assert_eq!(event, TrayEvent::NewNote);
        }
    }
}

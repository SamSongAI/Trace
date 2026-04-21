//! Win32 window primitives for the capture panel.
//!
//! The capture panel is a floating, always-on-top window that appears on
//! every Space, restores focus to the previously foregrounded app on
//! dismiss, and clamps itself into a sensible monitor when the saved frame
//! is off-screen. This module exposes the minimum set of Win32 wrappers
//! the iced UI layer needs to implement that behaviour without pulling the
//! full `windows` crate surface into the UI module.
//!
//! This is the Rust port of the window-management side of
//! `Sources/Trace/UI/Capture/CapturePanelController.swift` and
//! `Sources/Trace/Services/CapturePanelController.swift`. The auto-hide-on-
//! unpin behaviour is intentionally NOT handled here — that's a UI-layer
//! concern (iced listens for focus-loss events and decides whether to hide).
//!
//! ## Design
//!
//! - **Pure monitor math** ([`ScreenRect`] and [`place_on_best_monitor`])
//!   is cross-platform and testable on any host. The Mac reference picks
//!   the monitor with the largest overlap with the saved frame and falls
//!   back to centering on primary if the frame is fully off-screen; we
//!   reproduce that exactly.
//! - **HWND operations** are gated behind `#[cfg(windows)]` and take raw
//!   HWND values as `isize`, matching the convention used by
//!   [`crate::global_hotkey`] and [`crate::system_tray`].
//! - **Error enum** ([`WindowError`]) is exposed on all platforms so higher
//!   layers can compile on macOS/Linux developer machines.
//!
//! Unlike the two sibling modules in this crate, `window` holds no worker
//! thread and no message loop — every operation is a synchronous Win32
//! call on the caller's thread. The caller is expected to invoke these
//! from the UI thread after a global-hotkey press or user interaction,
//! where the process has the foreground-claim rights Windows requires.

use std::fmt;

/// Screen-coordinate rectangle. Follows the Win32 `RECT` convention where
/// `right` and `bottom` are exclusive, so `width = right - left`.
///
/// Coordinates are in virtual-screen pixels (the coordinate space used by
/// `GetWindowRect`, `MonitorFromRect`, and friends). Negative values are
/// legal — a monitor to the left of the primary can have a negative `left`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    /// Build from raw edge coordinates. No validation — a rect with
    /// `right < left` is legal input (treated as empty by [`Self::width`]
    /// and [`Self::is_empty`]).
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Build from origin + size. Useful when converting from
    /// persisted-frame storage that uses `(x, y, w, h)` tuples.
    pub const fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }

    /// Width in pixels, clamped to zero for inverted rects.
    pub fn width(&self) -> i32 {
        (self.right - self.left).max(0)
    }

    /// Height in pixels, clamped to zero for inverted rects.
    pub fn height(&self) -> i32 {
        (self.bottom - self.top).max(0)
    }

    /// A rect with zero width or height is empty. Inverted rects are also
    /// considered empty since [`Self::width`] / [`Self::height`] clamp.
    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    /// Area of intersection with `other`, as `i64` so a full-overlap of two
    /// 4K monitors (~8M pixels) can never overflow. Returns 0 if the rects
    /// don't intersect.
    pub fn intersection_area(&self, other: &ScreenRect) -> i64 {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);
        let w = (right - left).max(0) as i64;
        let h = (bottom - top).max(0) as i64;
        w * h
    }

    /// True iff the two rects share any positive area.
    pub fn intersects(&self, other: &ScreenRect) -> bool {
        self.intersection_area(other) > 0
    }

    /// Returns a rect with the same size as `self` slid to fit inside
    /// `bounds`. If `self` is larger than `bounds` on an axis, it shrinks
    /// to exactly `bounds` on that axis instead of preserving the original
    /// size.
    pub fn clamped_to(&self, bounds: &ScreenRect) -> ScreenRect {
        let w = self.width().min(bounds.width());
        let h = self.height().min(bounds.height());

        // Slide the origin into `bounds` while preserving the (possibly
        // shrunk) size. `max(bounds.left)` pushes a too-far-left rect in,
        // and `min(bounds.right - w)` pulls a too-far-right rect back.
        let left = self.left.max(bounds.left).min(bounds.right - w);
        let top = self.top.max(bounds.top).min(bounds.bottom - h);
        ScreenRect::new(left, top, left + w, top + h)
    }

    /// Build a rect of `size` centered inside `bounds`. If the requested
    /// size is larger than `bounds`, the result is clipped to `bounds`
    /// on that axis (the origin is pinned to `bounds.left` / `bounds.top`).
    pub fn centered_in(size: (i32, i32), bounds: &ScreenRect) -> ScreenRect {
        let (w, h) = size;
        let w = w.max(0).min(bounds.width());
        let h = h.max(0).min(bounds.height());
        let left = bounds.left + (bounds.width() - w) / 2;
        let top = bounds.top + (bounds.height() - h) / 2;
        ScreenRect::new(left, top, left + w, top + h)
    }
}

/// Picks the best placement for `desired` given the enumerated `monitors`.
/// Returns the rect to apply.
///
/// Selection rule (matches
/// `Sources/Trace/UI/Capture/CapturePanelController.swift:188-287`):
/// 1. Find the monitor with the largest intersection area with `desired`.
///    If that intersection is non-trivially positive (≥ 1 pixel on both
///    axes), clamp `desired` to that monitor's work area and return it.
/// 2. Otherwise (desired is fully off-screen), return a rect of
///    `desired_size` centered in the primary monitor's work area.
///
/// `monitors[0]` is assumed to be the primary monitor — the Windows-only
/// [`enumerate_monitor_work_areas`] helper sorts to preserve that ordering.
/// If `monitors` is empty the function returns `desired` unchanged (there
/// is nothing to clamp against); in practice Windows always has at least
/// one monitor.
///
/// `desired_size` is passed separately because a saved rect may be smaller
/// than a monitor's work area (we preserve it), but the centering fallback
/// needs to know the size independently — the `desired` rect's dimensions
/// may be bogus if its left/top were serialised incorrectly.
pub fn place_on_best_monitor(
    desired: ScreenRect,
    desired_size: (i32, i32),
    monitors: &[ScreenRect],
) -> ScreenRect {
    if monitors.is_empty() {
        return desired;
    }

    // Pick the monitor with the largest overlap with `desired`. Ties are
    // broken by first-seen (stable) — `max_by_key` returns the last max,
    // but for our purposes any tie-break is acceptable because two
    // monitors with the exact same overlap area produce visually identical
    // results after clamping.
    let best = monitors
        .iter()
        .map(|m| (m, desired.intersection_area(m)))
        .max_by_key(|(_, area)| *area);

    match best {
        Some((monitor, area)) if area > 0 => desired.clamped_to(monitor),
        // No monitor has positive overlap — desired is fully off-screen.
        // Fall back to centering at the caller-provided size on the
        // primary monitor.
        _ => ScreenRect::centered_in(desired_size, &monitors[0]),
    }
}

/// Error returned by window-management operations.
///
/// Exposed on all platforms so higher layers can `use trace_platform::window::WindowError`
/// without `#[cfg(windows)]` plumbing. The variants that wrap a `u32` carry
/// the Win32 `GetLastError` code captured at the moment of failure.
#[derive(Debug)]
pub enum WindowError {
    /// `SetWindowPos` returned `FALSE`. Inner value is `GetLastError`.
    SetWindowPosFailed(u32),
    /// `SetWindowLongPtrW` signalled failure — it returned 0 *and*
    /// `GetLastError` was non-zero (a return of 0 with last-error == 0
    /// just means the previous extended-style value was zero, which is
    /// legal and not an error).
    SetWindowLongFailed(u32),
    /// Both `SetForegroundWindow` attempts (plain, then via the
    /// `AttachThreadInput` workaround) returned `FALSE`. Inner value is
    /// `GetLastError` from the last attempt.
    SetForegroundFailed(u32),
    /// `EnumDisplayMonitors` returned `FALSE`. Extremely rare — usually
    /// indicates a graphics-driver problem or a session in transition.
    MonitorEnumFailed(u32),
    /// The HWND passed in is no longer alive (`IsWindow` returned
    /// `FALSE`). Callers treating this as a no-op is the typical
    /// response.
    InvalidWindow,
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::SetWindowPosFailed(code) => {
                write!(f, "SetWindowPos failed (Win32 error {code})")
            }
            WindowError::SetWindowLongFailed(code) => {
                write!(f, "SetWindowLongPtrW failed (Win32 error {code})")
            }
            WindowError::SetForegroundFailed(code) => write!(
                f,
                "SetForegroundWindow failed even with the AttachThreadInput workaround (Win32 error {code})"
            ),
            WindowError::MonitorEnumFailed(code) => {
                write!(f, "EnumDisplayMonitors failed (Win32 error {code})")
            }
            WindowError::InvalidWindow => {
                f.write_str("the target window handle is no longer valid")
            }
        }
    }
}

impl std::error::Error for WindowError {}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::{ScreenRect, WindowError};

    use std::mem::size_of;

    use windows::Win32::Foundation::{
        GetLastError, SetLastError, BOOL, HWND, LPARAM, RECT, TRUE, WIN32_ERROR,
    };
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MonitorFromRect, HDC, HMONITOR, MONITORINFO,
        MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindow,
        SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST,
        HWND_TOPMOST, MONITORINFOF_PRIMARY, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    };

    /// Enumerate the work areas (the portion of each monitor not occupied
    /// by the taskbar) of every attached display. The primary monitor is
    /// always first in the returned vector — callers rely on this for the
    /// fallback branch of [`super::place_on_best_monitor`].
    ///
    /// Returns [`WindowError::MonitorEnumFailed`] only if
    /// `EnumDisplayMonitors` itself fails, which almost never happens on a
    /// normal interactive session.
    pub fn enumerate_monitor_work_areas() -> Result<Vec<ScreenRect>, WindowError> {
        // Pair of (work area, is-primary) accumulated by the callback. We
        // carry the primary flag alongside the rect so we can promote the
        // primary to slot 0 after enumeration without re-querying
        // `GetMonitorInfoW`.
        let mut acc: Vec<(ScreenRect, bool)> = Vec::new();
        let acc_ptr = &mut acc as *mut Vec<(ScreenRect, bool)>;

        // SAFETY: we pass a live `*mut Vec<…>` through `lparam`; the
        // callback (below) casts it back and pushes into it. The lifetime
        // of `acc` covers the entire `EnumDisplayMonitors` call because we
        // borrow it mutably here and `EnumDisplayMonitors` is synchronous.
        let ok = unsafe {
            EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_enum_proc),
                LPARAM(acc_ptr as isize),
            )
        };

        if !ok.as_bool() {
            return Err(WindowError::MonitorEnumFailed(last_error_code()));
        }

        // Promote the primary monitor to index 0 so
        // `place_on_best_monitor` can use `monitors[0]` as its fallback
        // target without re-checking the flag.
        acc.sort_by_key(|(_, is_primary)| if *is_primary { 0 } else { 1 });

        Ok(acc.into_iter().map(|(rect, _)| rect).collect())
    }

    /// Returns the work area of the primary monitor. Never fails on a
    /// live Windows session (there is always a primary monitor).
    pub fn primary_monitor_work_area() -> Result<ScreenRect, WindowError> {
        // A zero-sized rect at (0, 0) is a perfectly valid input to
        // `MonitorFromRect`; with `MONITOR_DEFAULTTOPRIMARY` it always
        // returns the primary monitor.
        let origin = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let hmon = unsafe { MonitorFromRect(&origin, MONITOR_DEFAULTTOPRIMARY) };
        if hmon.is_invalid() {
            return Err(WindowError::MonitorEnumFailed(last_error_code()));
        }

        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let got = unsafe { GetMonitorInfoW(hmon, &mut info) };
        if !got.as_bool() {
            return Err(WindowError::MonitorEnumFailed(last_error_code()));
        }

        Ok(rect_to_screen(&info.rcWork))
    }

    /// Apply the tool-window + topmost extended styles to `hwnd`:
    ///
    /// * `WS_EX_TOOLWINDOW` hides the window from Alt+Tab and the taskbar
    ///   (the capture panel is transient UI, not a long-lived window).
    /// * `WS_EX_TOPMOST` keeps it above ordinary windows. Actual Z-order
    ///   application still requires [`set_topmost`] (setting the style
    ///   alone is not enough — see MSDN `SetWindowPos` remarks).
    ///
    /// Idempotent: the function OR-merges the bits, so calling it twice is
    /// a no-op on the second call.
    pub fn apply_panel_window_styles(hwnd_raw: isize) -> Result<(), WindowError> {
        let hwnd = HWND(hwnd_raw as *mut _);
        if !unsafe { IsWindow(hwnd) }.as_bool() {
            return Err(WindowError::InvalidWindow);
        }

        // SetWindowLongPtrW returns the previous value, or 0 on error.
        // A previous value of 0 is legal, so we have to disambiguate
        // with GetLastError — hence the SetLastError(0) priming.
        let existing = unsafe {
            SetLastError(WIN32_ERROR(0));
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
        };
        let err = last_error_code();
        if existing == 0 && err != 0 {
            return Err(WindowError::SetWindowLongFailed(err));
        }

        let new = existing | (WS_EX_TOPMOST.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
        if new == existing {
            // Already set — avoid the SetWindowLongPtrW round-trip.
            return Ok(());
        }

        let prev = unsafe {
            SetLastError(WIN32_ERROR(0));
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new)
        };
        let err = last_error_code();
        if prev == 0 && err != 0 {
            return Err(WindowError::SetWindowLongFailed(err));
        }

        Ok(())
    }

    /// Apply or clear the topmost Z-order state on `hwnd`. Must be called
    /// after [`apply_panel_window_styles`] for the style change to take
    /// visual effect (see MSDN `SetWindowPos` remarks on `HWND_TOPMOST`).
    ///
    /// Uses `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE` so the call only
    /// touches Z-order — position, size, and activation are preserved.
    pub fn set_topmost(hwnd_raw: isize, on: bool) -> Result<(), WindowError> {
        let hwnd = HWND(hwnd_raw as *mut _);
        if !unsafe { IsWindow(hwnd) }.as_bool() {
            return Err(WindowError::InvalidWindow);
        }

        let insert_after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
        let ok = unsafe {
            SetWindowPos(
                hwnd,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
        if ok.is_err() {
            return Err(WindowError::SetWindowPosFailed(last_error_code()));
        }
        Ok(())
    }

    /// Bring `hwnd` to the foreground.
    ///
    /// Attempts plain `SetForegroundWindow` first. Windows restricts this
    /// call to processes that have recently earned foreground-claim
    /// rights (e.g. just received a global hotkey press) — when those
    /// rights are absent, falls back to the documented
    /// `AttachThreadInput` workaround: briefly attaches our input queue
    /// to the current foreground window's thread, retries
    /// `SetForegroundWindow`, then detaches.
    ///
    /// Returns [`WindowError::SetForegroundFailed`] only if both attempts
    /// fail. Most callers will see the plain call succeed.
    pub fn bring_to_foreground(hwnd_raw: isize) -> Result<(), WindowError> {
        let hwnd = HWND(hwnd_raw as *mut _);
        if !unsafe { IsWindow(hwnd) }.as_bool() {
            return Err(WindowError::InvalidWindow);
        }

        // Fast path: this works whenever the process has foreground-claim
        // rights (immediately after a global-hotkey press, a click, etc.).
        if unsafe { SetForegroundWindow(hwnd) }.as_bool() {
            return Ok(());
        }

        // Slow path: attach our input queue to the foreground window's
        // thread, retry, detach. Skip the attach if there's no foreground
        // window or if it's already our thread.
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.0.is_null() {
            // Nothing is foreground — the retry would do the same thing
            // as the plain call. Report failure rather than spin.
            return Err(WindowError::SetForegroundFailed(last_error_code()));
        }

        let our_thread = unsafe { GetCurrentThreadId() };
        let foreground_thread = unsafe { GetWindowThreadProcessId(foreground, None) };
        if foreground_thread == 0 || foreground_thread == our_thread {
            // Nothing to attach to, or we're already the foreground
            // thread — return the original failure.
            return Err(WindowError::SetForegroundFailed(last_error_code()));
        }

        // Attach, retry, detach. We detach on both success and failure to
        // avoid leaving thread input attached, which would affect focus
        // behaviour across the rest of the session.
        let attached = unsafe { AttachThreadInput(our_thread, foreground_thread, TRUE) };
        let retry_ok = if attached.as_bool() {
            unsafe { SetForegroundWindow(hwnd) }.as_bool()
        } else {
            false
        };
        let last_err_before_detach = if retry_ok { 0 } else { last_error_code() };
        if attached.as_bool() {
            unsafe {
                // Best-effort detach — if this fails the session's
                // focus behaviour may be slightly off until the next
                // foreground change, but we can't recover further.
                let _ = AttachThreadInput(our_thread, foreground_thread, BOOL(0));
            }
        }

        if retry_ok {
            Ok(())
        } else {
            Err(WindowError::SetForegroundFailed(last_err_before_detach))
        }
    }

    /// Snapshot the currently-foreground HWND so it can later be restored
    /// via [`restore_foreground`]. Returns `None` if there is no foreground
    /// window or if the foreground window is our own `self_hwnd_raw`.
    ///
    /// Port of `CapturePanelController.swift:201-228`, which does the
    /// same check against `NSWorkspace.shared.frontmostApplication` and
    /// skips if it's Trace itself.
    pub fn capture_previous_foreground(self_hwnd_raw: isize) -> Option<isize> {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.0.is_null() {
            return None;
        }
        let raw = foreground.0 as isize;
        if raw == self_hwnd_raw {
            return None;
        }
        Some(raw)
    }

    /// Best-effort foreground restore. If `previous_hwnd_raw` no longer
    /// refers to a live window (`IsWindow` returns FALSE), returns
    /// [`WindowError::InvalidWindow`] without attempting the call — the
    /// caller is expected to treat this as a silent no-op (the user
    /// closed the previous window while our panel was open).
    pub fn restore_foreground(previous_hwnd_raw: isize) -> Result<(), WindowError> {
        let hwnd = HWND(previous_hwnd_raw as *mut _);
        if !unsafe { IsWindow(hwnd) }.as_bool() {
            return Err(WindowError::InvalidWindow);
        }

        // Restoring focus to a window that still exists is a foreground
        // transition just like any other — reuse the same fallback path.
        if unsafe { SetForegroundWindow(hwnd) }.as_bool() {
            return Ok(());
        }
        Err(WindowError::SetForegroundFailed(last_error_code()))
    }

    // ---- Internals --------------------------------------------------------

    /// `EnumDisplayMonitors` callback. Casts `lparam` back to our
    /// `Vec<(ScreenRect, bool)>` accumulator and appends the work area +
    /// primary flag for the current monitor.
    ///
    /// SAFETY: `lparam` must have been produced by
    /// [`enumerate_monitor_work_areas`] as a `*mut Vec<(ScreenRect, bool)>`
    /// pointing to an accumulator that is still alive (the caller
    /// guarantees this by holding `&mut acc` for the duration of
    /// `EnumDisplayMonitors`).
    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        if lparam.0 == 0 {
            return BOOL(0); // defensive — shouldn't happen
        }
        let acc = &mut *(lparam.0 as *mut Vec<(ScreenRect, bool)>);

        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(hmonitor, &mut info).as_bool() {
            // Skip this monitor and continue enumeration — returning
            // BOOL(0) would abort the whole enumeration, which is too
            // aggressive for a per-monitor failure.
            return BOOL(1);
        }

        let is_primary = info.dwFlags & MONITORINFOF_PRIMARY != 0;
        acc.push((rect_to_screen(&info.rcWork), is_primary));
        BOOL(1)
    }

    fn rect_to_screen(r: &RECT) -> ScreenRect {
        ScreenRect {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }

    fn last_error_code() -> u32 {
        unsafe { GetLastError().0 }
    }
}

#[cfg(windows)]
pub use imp::{
    apply_panel_window_styles, bring_to_foreground, capture_previous_foreground,
    enumerate_monitor_work_areas, primary_monitor_work_area, restore_foreground, set_topmost,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Pure-function tests (run on Mac and Windows) ---------------------

    #[test]
    fn screen_rect_basic_ops() {
        let r = ScreenRect::from_xywh(10, 20, 100, 50);
        assert_eq!(r.width(), 100);
        assert_eq!(r.height(), 50);
        assert!(!r.is_empty());

        let empty = ScreenRect::new(5, 5, 5, 42);
        assert_eq!(empty.width(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn screen_rect_intersection_area_disjoint_is_zero() {
        let a = ScreenRect::new(0, 0, 100, 100);
        let b = ScreenRect::new(200, 200, 300, 300);
        assert_eq!(a.intersection_area(&b), 0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn screen_rect_intersection_area_full_containment() {
        let outer = ScreenRect::new(0, 0, 1000, 1000);
        let inner = ScreenRect::from_xywh(100, 200, 50, 60);
        assert_eq!(inner.intersection_area(&outer), 50 * 60);
        assert_eq!(outer.intersection_area(&inner), 50 * 60);
    }

    #[test]
    fn screen_rect_intersection_area_partial_overlap() {
        // A covers x=[0,100], y=[0,100]; B covers x=[90,200], y=[95,300].
        // Overlap is x=[90,100] y=[95,100] → 10 × 5 = 50.
        let a = ScreenRect::new(0, 0, 100, 100);
        let b = ScreenRect::new(90, 95, 200, 300);
        assert_eq!(a.intersection_area(&b), 50);
        assert!(a.intersects(&b));
    }

    #[test]
    fn screen_rect_intersection_area_is_i64_and_does_not_overflow_on_4k() {
        // Two identical 4K rects. Full overlap = 3840 * 2160 = 8_294_400,
        // which is larger than i16 but comfortably inside i64.
        let a = ScreenRect::new(0, 0, 3840, 2160);
        let b = ScreenRect::new(0, 0, 3840, 2160);
        let overlap = a.intersection_area(&b);
        assert_eq!(overlap, 3840_i64 * 2160_i64);
        assert_eq!(overlap, 8_294_400);
        // Sanity: the return type is i64 so we can assign without cast.
        let _: i64 = overlap;
    }

    #[test]
    fn screen_rect_clamped_to_slides_inside_bounds() {
        // 100x100 rect at (-50, -50) should slide to (0, 0, 100, 100)
        // inside a 1920x1080 bounds.
        let bounds = ScreenRect::new(0, 0, 1920, 1080);
        let r = ScreenRect::from_xywh(-50, -50, 100, 100);
        let clamped = r.clamped_to(&bounds);
        assert_eq!(clamped, ScreenRect::new(0, 0, 100, 100));
    }

    #[test]
    fn screen_rect_clamped_to_shrinks_oversized() {
        // 3000x3000 rect into 1920x1080 bounds → shrinks to exactly
        // the bounds.
        let bounds = ScreenRect::new(0, 0, 1920, 1080);
        let r = ScreenRect::from_xywh(0, 0, 3000, 3000);
        let clamped = r.clamped_to(&bounds);
        assert_eq!(clamped, ScreenRect::new(0, 0, 1920, 1080));
    }

    #[test]
    fn screen_rect_centered_in_basic() {
        // 100x50 centered in 1920x1080 should land at
        // ((1920-100)/2, (1080-50)/2) = (910, 515).
        let bounds = ScreenRect::new(0, 0, 1920, 1080);
        let centered = ScreenRect::centered_in((100, 50), &bounds);
        assert_eq!(centered, ScreenRect::new(910, 515, 1010, 565));
    }

    #[test]
    fn place_on_best_monitor_picks_largest_overlap() {
        // Two monitors side-by-side. Desired rect overlaps monitor 2 by
        // 60x100 = 6000, monitor 1 by 40x100 = 4000 — should clamp into
        // monitor 2.
        let primary = ScreenRect::new(0, 0, 1920, 1080);
        let secondary = ScreenRect::new(1920, 0, 3840, 1080);
        let desired = ScreenRect::from_xywh(1880, 100, 100, 100);
        let placed = place_on_best_monitor(desired, (100, 100), &[primary, secondary]);
        // Clamped into secondary: slides right edge inward.
        assert!(placed.left >= secondary.left);
        assert!(placed.right <= secondary.right);
        assert_eq!(placed.width(), 100);
        assert_eq!(placed.height(), 100);
    }

    #[test]
    fn place_on_best_monitor_falls_back_to_primary_when_off_screen() {
        // Desired is way off to the right of both monitors; no overlap.
        let primary = ScreenRect::new(0, 0, 1920, 1080);
        let secondary = ScreenRect::new(1920, 0, 3840, 1080);
        let desired = ScreenRect::from_xywh(10_000, 10_000, 100, 100);
        let placed = place_on_best_monitor(desired, (100, 100), &[primary, secondary]);
        // Must be centered on primary: (1920-100)/2 = 910, (1080-100)/2 = 490.
        assert_eq!(placed, ScreenRect::new(910, 490, 1010, 590));
    }

    #[test]
    fn place_on_best_monitor_preserves_size_on_fallback() {
        // The desired rect's stored size is garbage (0x0), but the
        // caller passes a sane size. The fallback should center at the
        // sane size, not at the bogus (0,0) → (0,0) rect's "size".
        let primary = ScreenRect::new(0, 0, 1920, 1080);
        let bogus_desired = ScreenRect::new(-99999, -99999, -99999, -99999);
        let placed = place_on_best_monitor(bogus_desired, (400, 300), &[primary]);
        assert_eq!(placed.width(), 400);
        assert_eq!(placed.height(), 300);
    }

    #[test]
    fn place_on_best_monitor_single_monitor() {
        // Single monitor; desired overlaps partially, should clamp in.
        let primary = ScreenRect::new(0, 0, 1920, 1080);
        let desired = ScreenRect::from_xywh(1800, 900, 300, 300);
        let placed = place_on_best_monitor(desired, (300, 300), &[primary]);
        assert!(placed.right <= primary.right);
        assert!(placed.bottom <= primary.bottom);
        assert_eq!(placed.width(), 300);
        assert_eq!(placed.height(), 300);
    }

    // ---- Error Display / std::error::Error tests --------------------------

    #[test]
    fn window_error_display_is_human_readable_and_has_codes() {
        use WindowError::*;
        let cases: [(WindowError, &str); 5] = [
            (SetWindowPosFailed(1), "SetWindowPosFailed"),
            (SetWindowLongFailed(5), "SetWindowLongFailed"),
            (SetForegroundFailed(7), "SetForegroundFailed"),
            (MonitorEnumFailed(9), "MonitorEnumFailed"),
            (InvalidWindow, "InvalidWindow"),
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

        // Codes are surfaced.
        assert!(WindowError::SetWindowPosFailed(1409)
            .to_string()
            .contains("1409"));
        assert!(WindowError::SetWindowLongFailed(87)
            .to_string()
            .contains("87"));
        assert!(WindowError::SetForegroundFailed(5)
            .to_string()
            .contains('5'));
        assert!(WindowError::MonitorEnumFailed(6).to_string().contains('6'));
    }

    #[test]
    fn window_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<WindowError>();
    }

    // ---- Windows-only integration tests -----------------------------------
    //
    // These touch real HWND/monitor state so they need a live desktop
    // session. Marked `#[ignore]` — run manually on Windows with:
    //
    //     cargo test -p trace-platform -- --ignored

    #[cfg(windows)]
    mod windows_only {
        use super::super::*;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetWindowLongPtrW, GWL_EXSTYLE, WINDOW_EX_STYLE,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
        };

        /// Static window class is always registered — no RegisterClassExW
        /// needed.
        const STATIC_CLASS: &[u16] = &[
            'S' as u16, 'T' as u16, 'A' as u16, 'T' as u16, 'I' as u16, 'C' as u16, 0,
        ];

        fn create_dummy_hwnd() -> HWND {
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    PCWSTR(STATIC_CLASS.as_ptr()),
                    PCWSTR(std::ptr::null()),
                    WS_POPUP,
                    0,
                    0,
                    10,
                    10,
                    HWND(std::ptr::null_mut()),
                    None,
                    None,
                    None,
                )
                .expect("CreateWindowExW with STATIC class should succeed")
            }
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn primary_monitor_work_area_is_non_empty() {
            let r = primary_monitor_work_area()
                .expect("primary monitor lookup should succeed on a live session");
            assert!(r.width() > 0);
            assert!(r.height() > 0);
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn enumerate_monitor_work_areas_returns_at_least_one_and_primary_is_first() {
            let monitors = enumerate_monitor_work_areas()
                .expect("EnumDisplayMonitors should succeed on a live session");
            assert!(!monitors.is_empty());
            // The first entry must equal the primary monitor's work area.
            let primary = primary_monitor_work_area().expect("primary lookup should succeed");
            assert_eq!(monitors[0], primary);
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn apply_panel_window_styles_on_dummy_hwnd() {
            let hwnd = create_dummy_hwnd();
            let raw = hwnd.0 as isize;
            apply_panel_window_styles(raw)
                .expect("applying panel styles to a live HWND should succeed");

            let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            assert_ne!(ex & (WS_EX_TOPMOST.0 as isize), 0);
            assert_ne!(ex & (WS_EX_TOOLWINDOW.0 as isize), 0);

            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn set_topmost_toggle_roundtrip() {
            let hwnd = create_dummy_hwnd();
            let raw = hwnd.0 as isize;

            set_topmost(raw, true).expect("set_topmost(on) should succeed");
            let ex_on = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            assert_ne!(ex_on & (WS_EX_TOPMOST.0 as isize), 0);

            set_topmost(raw, false).expect("set_topmost(off) should succeed");
            let ex_off = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            assert_eq!(ex_off & (WS_EX_TOPMOST.0 as isize), 0);

            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }

        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn capture_previous_foreground_returns_none_for_self() {
            let hwnd = create_dummy_hwnd();
            let raw = hwnd.0 as isize;
            // Passing `raw` as self should suppress it even if it happens
            // to be the foreground window (which is unlikely for a hidden
            // popup but still a valid invariant to test).
            let captured = capture_previous_foreground(raw);
            assert_ne!(captured, Some(raw));
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }

        /// DOCUMENTED BEHAVIOUR: `restore_foreground` returns
        /// `Err(WindowError::InvalidWindow)` for a dead handle rather
        /// than Ok. Callers treat both as "don't panic, move on".
        #[test]
        #[ignore = "requires an interactive desktop session; run manually on Windows"]
        fn restore_foreground_tolerates_dead_handle() {
            let fake: isize = 0xDEAD_BEEF;
            match restore_foreground(fake) {
                Err(WindowError::InvalidWindow) => {}
                other => panic!("expected Err(InvalidWindow) for a dead handle, got {other:?}"),
            }
        }
    }
}

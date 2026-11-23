use std::ffi::c_void;
use std::mem::size_of;

use klip_core::{PlatformError, Rect, WindowId};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DWMWA_CLOAK, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWA_TRANSITIONS_FORCEDISABLED,
    DWMWINDOWATTRIBUTE, DwmGetWindowAttribute, DwmSetWindowAttribute,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, HWND_TOPMOST, IsIconic, IsWindowVisible,
    SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};
use windows::core::BOOL;

use crate::error::os_error;

struct Search {
    x: i32,
    y: i32,
    exclude: HWND,
    found: Option<Rect>,
}

/// Walks the top-level windows from the top of the z-order. Hit-testing with
/// `WindowFromPoint` cannot skip the overlay, which covers every point.
pub(crate) fn window_rect_at(x: i32, y: i32, exclude: WindowId) -> Option<Rect> {
    let mut search = Search {
        x,
        y,
        exclude: HWND(exclude.0 as *mut c_void),
        found: None,
    };
    // EnumWindows reports an error when the callback stops the walk early, which is
    // how a match is signalled, so the result carries no information.
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    search.found
}

unsafe extern "system" fn visit(window: HWND, context: LPARAM) -> BOOL {
    // SAFETY: `context` is the `Search` that `window_rect_at` keeps alive for the walk.
    let search = unsafe { &mut *(context.0 as *mut Search) };
    if window == search.exclude || !is_candidate(window) {
        return true.into();
    }
    match visible_bounds(window) {
        Some(bounds) if bounds.contains(search.x, search.y) => {
            search.found = Some(bounds);
            false.into()
        }
        _ => true.into(),
    }
}

fn is_candidate(window: HWND) -> bool {
    if !unsafe { IsWindowVisible(window) }.as_bool() || unsafe { IsIconic(window) }.as_bool() {
        return false;
    }
    let mut cloaked = 0u32;
    let queried = unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        )
    };
    if queried.is_ok() && cloaked != 0 {
        return false;
    }
    let style = unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) } as u32;
    let click_through = WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0;
    style & click_through != click_through
}

fn visible_bounds(window: HWND) -> Option<Rect> {
    let mut frame = RECT::default();
    unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut RECT as *mut c_void,
            size_of::<RECT>() as u32,
        )
    }
    .ok()?;
    let bounds = Rect {
        x: frame.left,
        y: frame.top,
        w: frame.right.abs_diff(frame.left),
        h: frame.bottom.abs_diff(frame.top),
    };
    (bounds.w > 0 && bounds.h > 0).then_some(bounds)
}

/// Shows the overlay cloaked and without the DWM open animation, so the first frame
/// can be drawn before anything appears on screen.
pub(crate) fn place_overlay(window: WindowId, area: Rect) -> Result<(), PlatformError> {
    let window = HWND(window.0 as *mut c_void);
    set_flag(
        window,
        DWMWA_TRANSITIONS_FORCEDISABLED,
        true,
        "disable overlay animation",
    )?;
    set_flag(window, DWMWA_CLOAK, true, "cloak overlay window")?;
    unsafe {
        SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            area.x,
            area.y,
            area.w as i32,
            area.h as i32,
            SWP_SHOWWINDOW,
        )
    }
    .map_err(os_error("place overlay window"))
}

pub(crate) fn reveal_overlay(window: WindowId) -> Result<(), PlatformError> {
    let window = HWND(window.0 as *mut c_void);
    set_flag(window, DWMWA_CLOAK, false, "uncloak overlay window")?;
    if !unsafe { SetForegroundWindow(window) }.as_bool() {
        log::debug!("overlay window did not take the foreground");
    }
    Ok(())
}

fn set_flag(
    window: HWND,
    attribute: DWMWINDOWATTRIBUTE,
    enabled: bool,
    operation: &'static str,
) -> Result<(), PlatformError> {
    let value = BOOL::from(enabled);
    unsafe {
        DwmSetWindowAttribute(
            window,
            attribute,
            (&raw const value).cast(),
            size_of::<BOOL>() as u32,
        )
    }
    .map_err(os_error(operation))
}

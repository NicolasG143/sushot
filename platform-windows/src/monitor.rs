use klip_core::{PlatformError, Rect};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::error::{last_os_error, os_error};

pub(crate) fn monitor_at_cursor() -> Result<Rect, PlatformError> {
    let mut cursor = POINT::default();
    unsafe { GetCursorPos(&mut cursor) }.map_err(os_error("read cursor position"))?;

    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return Err(last_os_error("read monitor bounds"));
    }

    Ok(rect_from(info.rcMonitor))
}

fn rect_from(area: RECT) -> Rect {
    Rect {
        x: area.left,
        y: area.top,
        w: area.right.abs_diff(area.left),
        h: area.bottom.abs_diff(area.top),
    }
}

#![cfg(windows)]

use klip_core::{Platform, Rect, WindowId};
use klip_platform_windows::WindowsPlatform;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::w;

fn probe_area(offset: i32) -> Rect {
    Rect {
        x: 140 + offset,
        y: 140 + offset,
        w: 300,
        h: 200,
    }
}

struct ProbeWindow(HWND);

impl ProbeWindow {
    fn open(area: Rect) -> Self {
        let module = unsafe { GetModuleHandleW(None) }.unwrap();
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("klip probe"),
                WS_POPUP | WS_VISIBLE,
                area.x,
                area.y,
                area.w as i32,
                area.h as i32,
                None,
                None,
                Some(module.into()),
                None,
            )
        }
        .unwrap();
        Self(window)
    }

    fn id(&self) -> WindowId {
        WindowId(self.0.0 as isize)
    }
}

impl Drop for ProbeWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

#[test]
fn window_rect_at_returns_the_topmost_window_bounds_under_the_point() {
    let area = probe_area(0);
    let _probe = ProbeWindow::open(area);

    let found = WindowsPlatform.window_rect_at(area.x + 10, area.y + 10, WindowId(0));

    assert_eq!(found, Some(area));
}

#[test]
fn window_rect_at_skips_the_excluded_window() {
    let area = probe_area(500);
    let probe = ProbeWindow::open(area);

    let found = WindowsPlatform.window_rect_at(area.x + 10, area.y + 10, probe.id());

    assert_ne!(found, Some(area));
}

#[test]
fn window_rect_at_finds_nothing_far_outside_the_desktop() {
    let found = WindowsPlatform.window_rect_at(-100_000, -100_000, WindowId(0));

    assert_eq!(found, None);
}

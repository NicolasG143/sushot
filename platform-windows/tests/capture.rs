#![cfg(windows)]

use klip_core::{Platform, Rect};
use klip_platform_windows::WindowsPlatform;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

fn virtual_desktop() -> Rect {
    unsafe {
        Rect {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
        }
    }
}

#[test]
fn grab_virtual_desktop_returns_an_opaque_frame_covering_every_monitor() {
    let frame = WindowsPlatform.grab_virtual_desktop().unwrap();

    assert_eq!(frame.bounds(), virtual_desktop());
    assert!(
        frame
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == 255)
    );
}

#[test]
fn monitor_at_cursor_lies_inside_the_virtual_desktop() {
    let monitor = WindowsPlatform.monitor_at_cursor().unwrap();

    assert!(monitor.w > 0 && monitor.h > 0);
    assert_eq!(virtual_desktop().intersection(monitor), Some(monitor));
}

#![cfg(windows)]

mod common;

use std::ptr;

use klip_core::Platform;
use klip_platform_windows::WindowsPlatform;
use windows::Win32::Graphics::Gdi::BITMAPV5HEADER;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;

use common::{
    RunningKlip, become_dpi_aware, empty_clipboard, png_size, press_print_screen_with,
    wait_for_copied_image,
};

#[test]
#[ignore = "sends real keystrokes and overwrites the clipboard; needs an interactive desktop"]
fn ctrl_print_screen_copies_the_monitor_under_the_cursor() {
    become_dpi_aware();
    empty_clipboard();
    let _klip = RunningKlip::start();

    press_print_screen_with(&[VK_CONTROL]);
    let copied = wait_for_copied_image();

    let monitor = WindowsPlatform
        .monitor_at_cursor()
        .expect("the cursor is on a monitor");
    let expected_size = (monitor.w, monitor.h);
    assert_eq!(png_size(copied.png), expected_size, "PNG size");

    assert!(
        copied.dib.len() >= size_of::<BITMAPV5HEADER>(),
        "CF_DIBV5 holds a V5 header"
    );
    let header = unsafe { ptr::read_unaligned(copied.dib.as_ptr().cast::<BITMAPV5HEADER>()) };
    assert_eq!(
        (header.bV5Width as u32, header.bV5Height as u32),
        expected_size,
        "CF_DIBV5 size"
    );
}

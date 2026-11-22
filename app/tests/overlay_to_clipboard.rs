#![cfg(windows)]

mod common;

use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_SHIFT};

use common::{
    PollerClipboard, RunningKlip, become_dpi_aware, empty_clipboard, left_button, move_cursor,
    png_size, press_print_screen_with, tap, wait_for_copied_image, wait_for_overlay,
};

const ORIGIN: (i32, i32) = (200, 200);

#[test]
#[ignore = "sends real input and overwrites the clipboard; needs an interactive desktop"]
fn dragging_a_region_copies_exactly_that_area() {
    become_dpi_aware();
    empty_clipboard();
    let _klip = RunningKlip::start();

    let started = Instant::now();
    press_print_screen_with(&[]);
    wait_for_overlay(true);
    eprintln!("overlay visible after {:?}", started.elapsed());
    move_cursor(ORIGIN.0, ORIGIN.1);
    left_button(true);
    move_cursor(ORIGIN.0 + 320, ORIGIN.1 + 180);
    left_button(false);
    wait_for_overlay(false);

    assert_eq!(png_size(wait_for_copied_image().png), (320, 180));
}

#[test]
#[ignore = "sends real input and overwrites the clipboard; needs an interactive desktop"]
fn one_press_reopens_the_overlay_after_a_region_capture() {
    become_dpi_aware();
    empty_clipboard();
    let _klip = RunningKlip::start();

    press_print_screen_with(&[]);
    wait_for_overlay(true);
    move_cursor(ORIGIN.0, ORIGIN.1);
    left_button(true);
    move_cursor(ORIGIN.0 + 120, ORIGIN.1 + 80);
    left_button(false);
    wait_for_overlay(false);
    wait_for_copied_image();

    press_print_screen_with(&[]);
    wait_for_overlay(true);
    tap(VK_ESCAPE);
    wait_for_overlay(false);
}

#[test]
#[ignore = "sends real input and overwrites the clipboard; needs an interactive desktop"]
fn escape_closes_the_overlay_and_copies_nothing() {
    become_dpi_aware();
    empty_clipboard();
    let _klip = RunningKlip::start();

    press_print_screen_with(&[]);
    wait_for_overlay(true);
    tap(VK_ESCAPE);
    wait_for_overlay(false);
    thread::sleep(Duration::from_millis(300));

    let clipboard = PollerClipboard::open().expect("the clipboard opens");
    assert!(clipboard.bytes_of(0x0008).is_none(), "nothing was copied");
}

#[test]
#[ignore = "sends real input and overwrites the clipboard; needs an interactive desktop"]
fn a_click_without_dragging_cancels_the_region() {
    become_dpi_aware();
    empty_clipboard();
    let _klip = RunningKlip::start();

    press_print_screen_with(&[]);
    wait_for_overlay(true);
    move_cursor(ORIGIN.0, ORIGIN.1);
    left_button(true);
    left_button(false);
    wait_for_overlay(false);
    thread::sleep(Duration::from_millis(300));

    let clipboard = PollerClipboard::open().expect("the clipboard opens");
    assert!(clipboard.bytes_of(0x0008).is_none(), "nothing was copied");
}

#[test]
#[ignore = "sends real input and overwrites the clipboard; needs an interactive desktop"]
fn clicking_in_window_mode_copies_the_window_under_the_cursor() {
    become_dpi_aware();
    empty_clipboard();
    let _klip = RunningKlip::start();

    press_print_screen_with(&[VK_SHIFT]);
    wait_for_overlay(true);
    move_cursor(ORIGIN.0, ORIGIN.1);
    move_cursor(ORIGIN.0 + 2, ORIGIN.1 + 2);
    left_button(true);
    left_button(false);
    wait_for_overlay(false);

    let (width, height) = png_size(wait_for_copied_image().png);
    assert!(width > 0 && height > 0);
}

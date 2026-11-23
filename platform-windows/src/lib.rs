#![cfg(windows)]

mod capture;
mod clipboard;
mod dib_v5;
mod error;
mod hotkeys;
mod monitor;
mod snapshot_key;
mod windows_at;

use std::sync::mpsc::Sender;

use klip_core::{Action, Frame, Platform, PlatformError, Rect, WindowId};

/// Coordinates are physical pixels only when the process is per-monitor DPI aware.
pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn grab_virtual_desktop(&self) -> Result<Frame, PlatformError> {
        capture::grab_virtual_desktop()
    }

    fn monitor_at_cursor(&self) -> Result<Rect, PlatformError> {
        monitor::monitor_at_cursor()
    }

    fn window_rect_at(&self, x: i32, y: i32, exclude: WindowId) -> Option<Rect> {
        windows_at::window_rect_at(x, y, exclude)
    }

    fn place_overlay(&self, window: WindowId, area: Rect) -> Result<(), PlatformError> {
        windows_at::place_overlay(window, area)
    }

    fn copy_image(&self, image: &Frame) -> Result<(), PlatformError> {
        clipboard::write_image(image)
    }

    fn register_hotkeys(&self, actions: Sender<Action>) -> Result<(), PlatformError> {
        hotkeys::register(actions)
    }
}

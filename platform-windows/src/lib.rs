#![cfg(windows)]

mod autostart;
mod balloon;
mod capture;
mod clipboard;
mod dib_v5;
mod environment;
mod error;
mod foreground;
mod hotkeys;
mod instance;
mod monitor;
mod snapshot_key;
mod windows_at;

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use klip_core::{
    Action, Frame, HotkeyKey, InstanceGuard, LocalTime, Platform, PlatformError, Rect, WindowId,
};

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

    fn reveal_overlay(&self, window: WindowId) -> Result<(), PlatformError> {
        windows_at::reveal_overlay(window)
    }

    fn grant_foreground(&self) {
        foreground::grant();
    }

    fn local_time(&self) -> LocalTime {
        environment::local_time()
    }

    fn config_path(&self) -> Option<PathBuf> {
        environment::config_path()
    }

    fn pictures_dir(&self) -> Option<PathBuf> {
        environment::pictures_dir()
    }

    fn set_autostart(&self, enabled: bool) -> Result<(), PlatformError> {
        autostart::set(enabled)
    }

    fn claim_instance(&self) -> Result<Option<InstanceGuard>, PlatformError> {
        Ok(instance::claim(instance::SINGLE_INSTANCE_NAME)?.map(InstanceGuard::new))
    }

    fn unregister_hotkeys(&self) {
        hotkeys::unregister();
    }

    fn copy_image(&self, image: &Frame) -> Result<(), PlatformError> {
        clipboard::write_image(image)
    }

    fn register_hotkeys(
        &self,
        key: HotkeyKey,
        actions: Sender<Action>,
    ) -> Result<(), PlatformError> {
        hotkeys::register(key, actions)
    }
}

/// Shows a notification from the tray icon identified by its window and icon id.
pub fn show_tray_notification(
    window: isize,
    icon_id: u32,
    title: &str,
    text: &str,
) -> Result<(), PlatformError> {
    balloon::show(window, icon_id, title, text)
}

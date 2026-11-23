// The path, autostart, lock and image modules hold no Linux calls so their tests run on
// any host; only the Linux backend uses them outside of tests.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

mod autostart;
mod image;
mod instance;
mod paths;
mod session;

#[cfg(target_os = "linux")]
mod hotkeys;
#[cfg(target_os = "linux")]
mod portal;
#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "linux")]
pub use backend::LinuxPlatform;

#[cfg(target_os = "linux")]
mod backend {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;
    use std::sync::{Mutex, PoisonError};

    use sushot_core::{
        Action, Frame, HotkeyKey, InstanceGuard, LocalTime, Mode, Platform, PlatformError, Rect,
        WindowId,
    };

    use crate::x11::fail;
    use crate::{autostart, hotkeys, instance, paths, portal, session, x11};

    /// X11 session: full backend. Wayland session: screenshots and shortcuts through
    /// the desktop portals, without sushot's own overlay.
    pub struct LinuxPlatform;

    fn var(name: &str) -> Option<String> {
        env::var(name).ok()
    }

    fn home() -> Option<PathBuf> {
        var("HOME")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
    }

    fn on_wayland() -> bool {
        session::is_wayland(
            var("XDG_SESSION_TYPE").as_deref(),
            var("WAYLAND_DISPLAY").as_deref(),
            var("DISPLAY").as_deref(),
        )
    }

    impl Platform for LinuxPlatform {
        fn grab_virtual_desktop(&self) -> Result<Frame, PlatformError> {
            x11::grab_virtual_desktop()
        }

        fn monitor_at_cursor(&self) -> Result<Rect, PlatformError> {
            x11::monitor_at_cursor()
        }

        fn window_rect_at(&self, x: i32, y: i32, exclude: WindowId) -> Option<Rect> {
            x11::window_rect_at(x, y, exclude)
        }

        fn place_overlay(&self, window: WindowId, area: Rect) -> Result<(), PlatformError> {
            x11::place_overlay(window, area)
        }

        fn has_selection_overlay(&self) -> bool {
            !on_wayland()
        }

        fn capture_with_system_ui(&self, mode: Mode) -> Result<Frame, PlatformError> {
            portal::screenshot(mode != Mode::FullScreen)
        }

        fn local_time(&self) -> LocalTime {
            local_time()
        }

        fn config_path(&self) -> Option<PathBuf> {
            paths::config_path(var("XDG_CONFIG_HOME").as_deref(), home().as_deref())
        }

        fn pictures_dir(&self) -> Option<PathBuf> {
            let user_dirs = home()
                .map(|home| home.join(".config").join("user-dirs.dirs"))
                .and_then(|path| fs::read_to_string(path).ok());
            paths::pictures_dir(home().as_deref(), user_dirs.as_deref())
        }

        fn ipc_path(&self) -> Option<PathBuf> {
            paths::socket_path(var("XDG_RUNTIME_DIR").as_deref())
        }

        fn set_autostart(&self, enabled: bool) -> Result<(), PlatformError> {
            let operation = "update autostart entry";
            let entry =
                paths::autostart_entry_path(var("XDG_CONFIG_HOME").as_deref(), home().as_deref())
                    .ok_or_else(|| PlatformError::new(operation, "no home directory"))?;
            let exe = env::current_exe().map_err(fail(operation))?;
            autostart::set(&entry, &exe, enabled).map_err(fail(operation))
        }

        fn claim_instance(&self) -> Result<Option<InstanceGuard>, PlatformError> {
            let operation = "claim the single instance lock";
            let path = paths::lock_path(var("XDG_RUNTIME_DIR").as_deref())
                .ok_or_else(|| PlatformError::new(operation, "XDG_RUNTIME_DIR is not set"))?;
            Ok(instance::claim(&path)
                .map_err(fail(operation))?
                .map(InstanceGuard::new))
        }

        fn register_hotkeys(
            &self,
            key: HotkeyKey,
            actions: Sender<Action>,
        ) -> Result<(), PlatformError> {
            if on_wayland() {
                portal::register_shortcuts(key, actions)
            } else {
                hotkeys::register(key, actions)
            }
        }

        fn unregister_hotkeys(&self) {
            hotkeys::unregister();
        }

        fn copy_image(&self, image: &Frame) -> Result<(), PlatformError> {
            let operation = "write the clipboard";
            let bounds = image.bounds();
            let data = arboard::ImageData {
                width: bounds.w as usize,
                height: bounds.h as usize,
                bytes: image.rgba().into(),
            };
            let mut slot = CLIPBOARD.lock().unwrap_or_else(PoisonError::into_inner);
            let mut clipboard = match slot.take() {
                Some(clipboard) => clipboard,
                None => arboard::Clipboard::new().map_err(fail(operation))?,
            };
            clipboard.set_image(data).map_err(fail(operation))?;
            *slot = Some(clipboard);
            Ok(())
        }
    }

    /// X11 serves clipboard contents from the process that owns the selection, so the
    /// clipboard must outlive each copy; dropping it loses the image unless a clipboard
    /// manager happens to take it over.
    static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

    fn local_time() -> LocalTime {
        // SAFETY: `time` accepts a null pointer and `localtime_r` writes only into `parts`.
        let parts = unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut parts: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut parts);
            parts
        };
        LocalTime {
            year: (parts.tm_year + 1900) as u16,
            month: (parts.tm_mon + 1) as u8,
            day: parts.tm_mday as u8,
            hour: parts.tm_hour as u8,
            minute: parts.tm_min as u8,
            second: parts.tm_sec as u8,
        }
    }
}

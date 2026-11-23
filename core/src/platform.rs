use std::any::Any;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::{Action, Frame, HotkeyKey, LocalTime, Mode, Rect};

/// Native handle of a top-level window, as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub isize);

/// Proof that this process is the only running instance; dropping it releases the claim.
pub struct InstanceGuard {
    _holder: Box<dyn Any>,
}

impl InstanceGuard {
    pub fn new(holder: impl Any) -> Self {
        Self {
            _holder: Box::new(holder),
        }
    }
}

#[derive(Debug)]
pub struct PlatformError {
    operation: &'static str,
    detail: String,
}

impl PlatformError {
    pub fn new(operation: &'static str, detail: impl Into<String>) -> Self {
        Self {
            operation,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} failed: {}", self.operation, self.detail)
    }
}

impl std::error::Error for PlatformError {}

pub trait Platform {
    /// Captures every monitor into one frame whose bounds are the virtual desktop.
    fn grab_virtual_desktop(&self) -> Result<Frame, PlatformError>;

    /// Bounds of the monitor under the cursor, in virtual desktop coordinates.
    fn monitor_at_cursor(&self) -> Result<Rect, PlatformError>;

    /// Visible bounds of the topmost window under the point, ignoring `exclude`.
    fn window_rect_at(&self, x: i32, y: i32, exclude: WindowId) -> Option<Rect>;

    /// Covers `area` with `window`, raised above every other window, in physical pixels.
    fn place_overlay(&self, window: WindowId, area: Rect) -> Result<(), PlatformError>;

    /// Makes the overlay placed by `place_overlay` appear once its first frame is drawn.
    fn reveal_overlay(&self, _window: WindowId) -> Result<(), PlatformError> {
        Ok(())
    }

    /// Whether sushot can draw its own selection overlay. When it cannot, selection happens
    /// in the system's UI through `capture_with_system_ui`.
    fn has_selection_overlay(&self) -> bool {
        true
    }

    /// Captures `mode` through the system's own capture UI and returns the result.
    fn capture_with_system_ui(&self, _mode: Mode) -> Result<Frame, PlatformError> {
        Err(PlatformError::new(
            "capture with the system UI",
            "not supported on this platform",
        ))
    }

    /// Socket through which `sushot --region` and friends reach the running instance.
    fn ipc_path(&self) -> Option<PathBuf> {
        None
    }

    fn local_time(&self) -> LocalTime;

    /// Where `config.toml` lives for the current user.
    fn config_path(&self) -> Option<PathBuf>;

    /// The user's pictures folder.
    fn pictures_dir(&self) -> Option<PathBuf>;

    /// Registers or removes the entry that starts sushot with the system.
    fn set_autostart(&self, enabled: bool) -> Result<(), PlatformError>;

    /// `None` means another sushot instance is already running.
    fn claim_instance(&self) -> Result<Option<InstanceGuard>, PlatformError>;

    /// Removes the hotkey hooks and waits until they are gone.
    fn unregister_hotkeys(&self);

    /// Replaces the clipboard contents with `image`.
    fn copy_image(&self, image: &Frame) -> Result<(), PlatformError>;

    /// Starts delivering an `Action` for every recognized Print Screen combination.
    /// Delivery stops once the receiving end of `actions` is dropped.
    fn register_hotkeys(
        &self,
        key: HotkeyKey,
        actions: Sender<Action>,
    ) -> Result<(), PlatformError>;
}

use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::{Action, Frame, LocalTime, Rect};

/// Native handle of a top-level window, as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub isize);

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

    /// Lets the next window this process opens take the foreground instead of
    /// appearing behind the active application.
    fn grant_foreground(&self);

    fn local_time(&self) -> LocalTime;

    /// Where `config.toml` lives for the current user.
    fn config_path(&self) -> Option<PathBuf>;

    /// The user's pictures folder.
    fn pictures_dir(&self) -> Option<PathBuf>;

    /// Replaces the clipboard contents with `image`.
    fn copy_image(&self, image: &Frame) -> Result<(), PlatformError>;

    /// Starts delivering an `Action` for every recognized Print Screen combination.
    /// Delivery stops once the receiving end of `actions` is dropped.
    fn register_hotkeys(&self, actions: Sender<Action>) -> Result<(), PlatformError>;
}

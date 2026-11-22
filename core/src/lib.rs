mod action;
mod canvas;
mod config;
mod encode;
mod filename;
mod frame;
mod geometry;
mod glyphs;
mod hotkey;
mod platform;

pub use action::{Action, Mode, Output, resolve};
pub use canvas::Canvas;
pub use config::{
    Config, ConfigError, load as load_config, save as save_config, update as update_config,
};
pub use encode::{encode_png, write_png};
pub use filename::{LocalTime, default_file_name};
pub use frame::{Frame, FrameError};
pub use geometry::Rect;
pub use hotkey::HotkeyKey;
pub use platform::{InstanceGuard, Platform, PlatformError, WindowId};

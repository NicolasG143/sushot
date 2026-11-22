mod action;
mod frame;
mod geometry;
mod platform;

pub use action::{Action, Mode, Output, resolve};
pub use frame::{Frame, FrameError};
pub use geometry::Rect;
pub use platform::{Platform, PlatformError};

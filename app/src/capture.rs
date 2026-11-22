use std::fmt;

use klip_core::{Action, Mode, Output, Platform, PlatformError, Rect};

#[derive(Debug)]
pub(crate) enum CaptureError {
    Platform(PlatformError),
    MonitorOutsideDesktop(Rect),
    NotAvailable(Action),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::Platform(error) => error.fmt(f),
            CaptureError::MonitorOutsideDesktop(monitor) => {
                write!(
                    f,
                    "monitor {monitor:?} does not overlap the captured desktop"
                )
            }
            CaptureError::NotAvailable(action) => {
                write!(
                    f,
                    "{:?} to {:?} is not available in this build",
                    action.mode, action.output
                )
            }
        }
    }
}

impl std::error::Error for CaptureError {}

impl From<PlatformError> for CaptureError {
    fn from(error: PlatformError) -> Self {
        CaptureError::Platform(error)
    }
}

pub(crate) fn perform(platform: &impl Platform, action: Action) -> Result<(), CaptureError> {
    match (action.mode, action.output) {
        (Mode::FullScreen, Output::Clipboard) => {
            let desktop = platform.grab_virtual_desktop()?;
            let monitor = platform.monitor_at_cursor()?;
            let capture = desktop
                .crop(monitor)
                .ok_or(CaptureError::MonitorOutsideDesktop(monitor))?;
            platform.copy_image(&capture)?;

            let copied = capture.bounds();
            log::info!(
                "copied a {}x{} capture to the clipboard",
                copied.w,
                copied.h
            );
            Ok(())
        }
        _ => Err(CaptureError::NotAvailable(action)),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::mpsc::Sender;

    use klip_core::Frame;

    use super::*;

    const DESKTOP: Rect = Rect {
        x: -2,
        y: 0,
        w: 4,
        h: 1,
    };
    const LEFT_MONITOR: Rect = Rect {
        x: -2,
        y: 0,
        w: 2,
        h: 1,
    };
    const LEFT_PIXELS: [u8; 8] = [1, 1, 1, 255, 2, 2, 2, 255];
    const RIGHT_PIXELS: [u8; 8] = [3, 3, 3, 255, 4, 4, 4, 255];

    #[derive(Default)]
    struct TwoMonitors {
        copied: RefCell<Option<(Rect, Vec<u8>)>>,
    }

    impl Platform for TwoMonitors {
        fn grab_virtual_desktop(&self) -> Result<Frame, PlatformError> {
            Ok(Frame::new(DESKTOP, [LEFT_PIXELS, RIGHT_PIXELS].concat()).unwrap())
        }

        fn monitor_at_cursor(&self) -> Result<Rect, PlatformError> {
            Ok(LEFT_MONITOR)
        }

        fn copy_image(&self, image: &Frame) -> Result<(), PlatformError> {
            self.copied
                .replace(Some((image.bounds(), image.rgba().to_vec())));
            Ok(())
        }

        fn register_hotkeys(&self, _actions: Sender<Action>) -> Result<(), PlatformError> {
            Ok(())
        }
    }

    #[test]
    fn full_screen_to_clipboard_copies_only_the_monitor_under_the_cursor() {
        let platform = TwoMonitors::default();
        let action = Action {
            mode: Mode::FullScreen,
            output: Output::Clipboard,
        };

        perform(&platform, action).unwrap();

        assert_eq!(
            platform.copied.take(),
            Some((LEFT_MONITOR, LEFT_PIXELS.to_vec()))
        );
    }

    #[test]
    fn action_without_an_implementation_is_reported_and_copies_nothing() {
        let platform = TwoMonitors::default();
        let action = Action {
            mode: Mode::Region,
            output: Output::Clipboard,
        };

        let result = perform(&platform, action);

        assert!(matches!(result, Err(CaptureError::NotAvailable(reported)) if reported == action));
        assert_eq!(platform.copied.take(), None);
    }
}

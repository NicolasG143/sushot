use std::fmt;

use klip_core::{Frame, Output, Platform, PlatformError, Rect};

#[derive(Debug)]
pub(crate) enum CaptureError {
    Platform(PlatformError),
    AreaOutsideDesktop(Rect),
    OutputNotAvailable(Output),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::Platform(error) => error.fmt(f),
            CaptureError::AreaOutsideDesktop(area) => {
                write!(f, "area {area:?} does not overlap the captured desktop")
            }
            CaptureError::OutputNotAvailable(output) => {
                write!(f, "output {output:?} is not available in this build")
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

pub(crate) fn full_screen(platform: &impl Platform, output: Output) -> Result<(), CaptureError> {
    let desktop = platform.grab_virtual_desktop()?;
    let monitor = platform.monitor_at_cursor()?;
    deliver(platform, &desktop, monitor, output)
}

/// Crops `area` out of the frozen `desktop` and sends it to `output`.
pub(crate) fn deliver(
    platform: &impl Platform,
    desktop: &Frame,
    area: Rect,
    output: Output,
) -> Result<(), CaptureError> {
    let capture = desktop
        .crop(area)
        .ok_or(CaptureError::AreaOutsideDesktop(area))?;
    match output {
        Output::Clipboard => {
            platform.copy_image(&capture)?;
            let copied = capture.bounds();
            log::info!(
                "copied a {}x{} capture to the clipboard",
                copied.w,
                copied.h
            );
            Ok(())
        }
        Output::SaveDialog => Err(CaptureError::OutputNotAvailable(output)),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::mpsc::Sender;

    use klip_core::{Action, WindowId};

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
            Ok(desktop())
        }

        fn monitor_at_cursor(&self) -> Result<Rect, PlatformError> {
            Ok(LEFT_MONITOR)
        }

        fn window_rect_at(&self, _x: i32, _y: i32, _exclude: WindowId) -> Option<Rect> {
            None
        }

        fn place_overlay(&self, _window: WindowId, _area: Rect) -> Result<(), PlatformError> {
            Ok(())
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

    fn desktop() -> Frame {
        Frame::new(DESKTOP, [LEFT_PIXELS, RIGHT_PIXELS].concat()).unwrap()
    }

    #[test]
    fn full_screen_to_clipboard_copies_only_the_monitor_under_the_cursor() {
        let platform = TwoMonitors::default();

        full_screen(&platform, Output::Clipboard).unwrap();

        assert_eq!(
            platform.copied.take(),
            Some((LEFT_MONITOR, LEFT_PIXELS.to_vec()))
        );
    }

    #[test]
    fn deliver_crops_the_requested_area_out_of_the_frozen_desktop() {
        let platform = TwoMonitors::default();
        let area = Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };

        deliver(&platform, &desktop(), area, Output::Clipboard).unwrap();

        assert_eq!(platform.copied.take(), Some((area, vec![3, 3, 3, 255])));
    }

    #[test]
    fn deliver_reports_an_area_outside_the_desktop_and_copies_nothing() {
        let platform = TwoMonitors::default();
        let area = Rect {
            x: 100,
            y: 100,
            w: 5,
            h: 5,
        };

        let result = deliver(&platform, &desktop(), area, Output::Clipboard);

        assert!(
            matches!(result, Err(CaptureError::AreaOutsideDesktop(reported)) if reported == area)
        );
        assert_eq!(platform.copied.take(), None);
    }

    #[test]
    fn output_without_an_implementation_is_reported_and_copies_nothing() {
        let platform = TwoMonitors::default();

        let result = full_screen(&platform, Output::SaveDialog);

        assert!(matches!(
            result,
            Err(CaptureError::OutputNotAvailable(Output::SaveDialog))
        ));
        assert_eq!(platform.copied.take(), None);
    }
}

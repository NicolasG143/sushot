use std::fmt;

use sushot_core::{Frame, Output, Platform, PlatformError, Rect, default_file_name, load_config};

use crate::save::{Dialogs, SavePlan, Saver};

#[derive(Debug)]
pub(crate) enum CaptureError {
    Platform(PlatformError),
    AreaOutsideDesktop(Rect),
    SaveInProgress,
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::Platform(error) => error.fmt(f),
            CaptureError::AreaOutsideDesktop(area) => {
                write!(f, "area {area:?} does not overlap the captured desktop")
            }
            CaptureError::SaveInProgress => {
                write!(f, "another capture is still waiting to be saved")
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Delivery {
    Copied,
    SaveStarted,
}

pub(crate) fn full_screen(
    platform: &impl Platform,
    saver: &Saver,
    dialogs: impl Dialogs + Send + 'static,
    output: Output,
) -> Result<Delivery, CaptureError> {
    let desktop = platform.grab_virtual_desktop()?;
    let monitor = platform.monitor_at_cursor()?;
    deliver(platform, saver, dialogs, &desktop, monitor, output)
}

/// Crops `area` out of the frozen `desktop` and sends it to `output`.
pub(crate) fn deliver(
    platform: &impl Platform,
    saver: &Saver,
    dialogs: impl Dialogs + Send + 'static,
    desktop: &Frame,
    area: Rect,
    output: Output,
) -> Result<Delivery, CaptureError> {
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
            Ok(Delivery::Copied)
        }
        Output::SaveDialog => {
            platform.grant_foreground();
            let plan = save_plan(platform);
            if saver.start(capture, plan, dialogs) {
                Ok(Delivery::SaveStarted)
            } else {
                Err(CaptureError::SaveInProgress)
            }
        }
    }
}

fn save_plan(platform: &impl Platform) -> SavePlan {
    let config_path = platform.config_path();
    let remembered = config_path
        .as_deref()
        .and_then(|path| load_config(path).last_save_dir)
        .filter(|directory| directory.is_dir());
    SavePlan {
        default_directory: remembered.or_else(|| platform.pictures_dir()),
        file_name: default_file_name(platform.local_time()),
        config_path,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::Sender;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use sushot_core::{Action, LocalTime, WindowId, update_config};

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

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sushot-capture-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("pictures")).unwrap();
            Self(dir)
        }

        fn pictures(&self) -> PathBuf {
            self.0.join("pictures")
        }

        fn config(&self) -> PathBuf {
            self.0.join("config").join("config.toml")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct TwoMonitors {
        scratch: Scratch,
        copied: RefCell<Option<(Rect, Vec<u8>)>>,
    }

    impl TwoMonitors {
        fn new(name: &str) -> Self {
            Self {
                scratch: Scratch::new(name),
                copied: RefCell::default(),
            }
        }
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

        fn grant_foreground(&self) {}

        fn set_autostart(&self, _enabled: bool) -> Result<(), PlatformError> {
            Ok(())
        }

        fn claim_instance(&self) -> Result<Option<sushot_core::InstanceGuard>, PlatformError> {
            Ok(None)
        }

        fn unregister_hotkeys(&self) {}

        fn local_time(&self) -> LocalTime {
            LocalTime {
                year: 2026,
                month: 11,
                day: 22,
                hour: 18,
                minute: 30,
                second: 5,
            }
        }

        fn config_path(&self) -> Option<PathBuf> {
            Some(self.scratch.config())
        }

        fn pictures_dir(&self) -> Option<PathBuf> {
            Some(self.scratch.pictures())
        }

        fn copy_image(&self, image: &Frame) -> Result<(), PlatformError> {
            self.copied
                .replace(Some((image.bounds(), image.rgba().to_vec())));
            Ok(())
        }

        fn register_hotkeys(
            &self,
            _key: sushot_core::HotkeyKey,
            _actions: Sender<Action>,
        ) -> Result<(), PlatformError> {
            Ok(())
        }
    }

    type Asked = Vec<(Option<PathBuf>, String)>;

    #[derive(Clone, Default)]
    struct Recording {
        asked: Arc<Mutex<Asked>>,
        answer: Arc<Mutex<Option<PathBuf>>>,
    }

    impl Dialogs for Recording {
        fn pick_path(&self, directory: Option<&Path>, file_name: &str) -> Option<PathBuf> {
            self.asked
                .lock()
                .unwrap()
                .push((directory.map(Path::to_path_buf), file_name.to_owned()));
            self.answer.lock().unwrap().clone()
        }

        fn retry_after_failure(&self, _path: &Path, _error: &io::Error) -> bool {
            false
        }
    }

    fn desktop() -> Frame {
        Frame::new(DESKTOP, [LEFT_PIXELS, RIGHT_PIXELS].concat()).unwrap()
    }

    fn wait_until_idle(saver: &Saver) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while saver.is_busy() {
            assert!(Instant::now() < deadline, "the save never finished");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn full_screen_to_clipboard_copies_only_the_monitor_under_the_cursor() {
        let platform = TwoMonitors::new("monitor");

        let delivery = full_screen(
            &platform,
            &Saver::default(),
            Recording::default(),
            Output::Clipboard,
        )
        .unwrap();

        assert_eq!(delivery, Delivery::Copied);
        assert_eq!(
            platform.copied.take(),
            Some((LEFT_MONITOR, LEFT_PIXELS.to_vec()))
        );
    }

    #[test]
    fn deliver_crops_the_requested_area_out_of_the_frozen_desktop() {
        let platform = TwoMonitors::new("crop");
        let area = Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };

        deliver(
            &platform,
            &Saver::default(),
            Recording::default(),
            &desktop(),
            area,
            Output::Clipboard,
        )
        .unwrap();

        assert_eq!(platform.copied.take(), Some((area, vec![3, 3, 3, 255])));
    }

    #[test]
    fn deliver_reports_an_area_outside_the_desktop_and_copies_nothing() {
        let platform = TwoMonitors::new("outside");
        let area = Rect {
            x: 100,
            y: 100,
            w: 5,
            h: 5,
        };

        let result = deliver(
            &platform,
            &Saver::default(),
            Recording::default(),
            &desktop(),
            area,
            Output::Clipboard,
        );

        assert!(
            matches!(result, Err(CaptureError::AreaOutsideDesktop(reported)) if reported == area)
        );
        assert_eq!(platform.copied.take(), None);
    }

    #[test]
    fn save_dialog_output_opens_in_pictures_with_the_timestamped_name_and_skips_the_clipboard() {
        let platform = TwoMonitors::new("pictures");
        let saver = Saver::default();
        let dialogs = Recording::default();
        let target = platform.scratch.pictures().join("shot.png");
        *dialogs.answer.lock().unwrap() = Some(target.clone());

        let delivery = full_screen(&platform, &saver, dialogs.clone(), Output::SaveDialog).unwrap();
        wait_until_idle(&saver);

        assert_eq!(delivery, Delivery::SaveStarted);
        assert_eq!(
            *dialogs.asked.lock().unwrap(),
            vec![(
                Some(platform.scratch.pictures()),
                "Captura_2026-11-22_18-30-05.png".to_owned()
            )]
        );
        assert!(target.is_file());
        assert_eq!(platform.copied.take(), None);
    }

    #[test]
    fn save_dialog_output_reopens_in_the_remembered_folder() {
        let platform = TwoMonitors::new("remembered");
        let remembered = platform.scratch.0.join("elsewhere");
        fs::create_dir_all(&remembered).unwrap();
        update_config(&platform.scratch.config(), |config| {
            config.last_save_dir = Some(remembered.clone());
        })
        .unwrap();
        let saver = Saver::default();
        let dialogs = Recording::default();

        full_screen(&platform, &saver, dialogs.clone(), Output::SaveDialog).unwrap();
        wait_until_idle(&saver);

        assert_eq!(dialogs.asked.lock().unwrap()[0].0, Some(remembered));
    }

    #[test]
    fn save_dialog_output_falls_back_to_pictures_when_the_remembered_folder_is_gone() {
        let platform = TwoMonitors::new("gone");
        update_config(&platform.scratch.config(), |config| {
            config.last_save_dir = Some(platform.scratch.0.join("deleted"));
        })
        .unwrap();
        let saver = Saver::default();
        let dialogs = Recording::default();

        full_screen(&platform, &saver, dialogs.clone(), Output::SaveDialog).unwrap();
        wait_until_idle(&saver);

        assert_eq!(
            dialogs.asked.lock().unwrap()[0].0,
            Some(platform.scratch.pictures())
        );
    }
}

use std::fs::File;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use klip_core::{Frame, update_config, write_png};

const SAVE_THREAD: &str = "klip-save";

pub(crate) trait Dialogs {
    /// Asks where to save. `None` means the user cancelled.
    fn pick_path(&self, directory: Option<&Path>, file_name: &str) -> Option<PathBuf>;

    /// Reports a failed save. `true` asks to try again.
    fn retry_after_failure(&self, path: &Path, error: &io::Error) -> bool;
}

pub(crate) struct SavePlan {
    pub(crate) default_directory: Option<PathBuf>,
    pub(crate) file_name: String,
    pub(crate) config_path: Option<PathBuf>,
}

/// Keeps one save dialog open at a time and moves the slow work off the UI thread.
#[derive(Clone, Default)]
pub(crate) struct Saver {
    busy: Arc<AtomicBool>,
}

impl Saver {
    /// Returns `false` without doing anything while a previous save is still open.
    pub(crate) fn start(
        &self,
        image: Frame,
        plan: SavePlan,
        dialogs: impl Dialogs + Send + 'static,
    ) -> bool {
        if self.busy.swap(true, Ordering::AcqRel) {
            log::debug!("a save dialog is already open; ignoring the new capture");
            return false;
        }
        let busy = self.busy.clone();
        let spawned = thread::Builder::new().name(SAVE_THREAD.into()).spawn({
            let busy = busy.clone();
            move || {
                run(&image, &plan, &dialogs);
                busy.store(false, Ordering::Release);
            }
        });
        if let Err(error) = spawned {
            log::error!("save thread did not start: {error}");
            busy.store(false, Ordering::Release);
            return false;
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }
}

/// The image stays in memory until the user saves it or gives up.
pub(crate) fn run(image: &Frame, plan: &SavePlan, dialogs: &impl Dialogs) {
    let mut directory = plan.default_directory.clone();
    loop {
        let Some(path) = dialogs.pick_path(directory.as_deref(), &plan.file_name) else {
            log::info!("save cancelled; the capture was discarded");
            return;
        };
        match write(image, &path) {
            Ok(()) => {
                log::info!("saved {}", path.display());
                remember_directory(plan.config_path.as_deref(), &path);
                return;
            }
            Err(error) => {
                log::error!("could not save {}: {error}", path.display());
                if !dialogs.retry_after_failure(&path, &error) {
                    log::info!("save abandoned; the capture was discarded");
                    return;
                }
                directory = path.parent().map(Path::to_path_buf).or(directory);
            }
        }
    }
}

fn write(image: &Frame, path: &Path) -> io::Result<()> {
    let file = File::create(path)?;
    let mut sink = BufWriter::new(file);
    write_png(&mut sink, image).map_err(io::Error::other)?;
    sink.into_inner().map_err(io::IntoInnerError::into_error)?;
    Ok(())
}

fn remember_directory(config_path: Option<&Path>, saved: &Path) {
    let (Some(config_path), Some(directory)) = (config_path, saved.parent()) else {
        return;
    };
    if let Err(error) = update_config(config_path, |config| {
        config.last_save_dir = Some(directory.to_path_buf());
    }) {
        log::warn!("last save folder was not remembered: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::time::{Duration, Instant};

    use klip_core::{Rect, load_config};

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("klip-save-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct Scripted {
        picks: RefCell<Vec<Option<PathBuf>>>,
        asked_directories: RefCell<Vec<Option<PathBuf>>>,
        retry: bool,
        failures: RefCell<Vec<PathBuf>>,
    }

    impl Scripted {
        fn new(picks: Vec<Option<PathBuf>>, retry: bool) -> Self {
            Self {
                picks: RefCell::new(picks.into_iter().rev().collect()),
                asked_directories: RefCell::default(),
                retry,
                failures: RefCell::default(),
            }
        }
    }

    impl Dialogs for Scripted {
        fn pick_path(&self, directory: Option<&Path>, _file_name: &str) -> Option<PathBuf> {
            self.asked_directories
                .borrow_mut()
                .push(directory.map(Path::to_path_buf));
            self.picks.borrow_mut().pop().flatten()
        }

        fn retry_after_failure(&self, path: &Path, _error: &io::Error) -> bool {
            self.failures.borrow_mut().push(path.to_path_buf());
            self.retry
        }
    }

    fn image() -> Frame {
        let bounds = Rect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        };
        Frame::new(bounds, vec![10, 20, 30, 255, 40, 50, 60, 255]).unwrap()
    }

    fn plan(scratch: &Scratch) -> SavePlan {
        SavePlan {
            default_directory: Some(scratch.0.clone()),
            file_name: "Captura.png".into(),
            config_path: Some(scratch.path("config").join("config.toml")),
        }
    }

    fn decode(path: &Path) -> (u32, u32, Vec<u8>) {
        let mut reader = png::Decoder::new(std::io::BufReader::new(File::open(path).unwrap()))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        (info.width, info.height, pixels)
    }

    #[test]
    fn chosen_path_receives_a_valid_png_and_the_folder_is_remembered() {
        let scratch = Scratch::new("success");
        let target = scratch.path("shot.png");
        let dialogs = Scripted::new(vec![Some(target.clone())], false);
        let plan = plan(&scratch);

        run(&image(), &plan, &dialogs);

        assert_eq!(
            decode(&target),
            (2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255])
        );
        let config = load_config(plan.config_path.as_deref().unwrap());
        assert_eq!(config.last_save_dir, Some(scratch.0.clone()));
    }

    #[test]
    fn cancelling_the_dialog_writes_nothing_and_keeps_the_config_untouched() {
        let scratch = Scratch::new("cancel");
        let dialogs = Scripted::new(vec![None], true);
        let plan = plan(&scratch);

        run(&image(), &plan, &dialogs);

        assert!(dialogs.failures.borrow().is_empty());
        assert!(!plan.config_path.unwrap().exists());
        assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 0);
    }

    #[test]
    fn a_failed_write_is_reported_and_retrying_saves_the_same_capture() {
        let scratch = Scratch::new("retry");
        let missing_folder = scratch.path("gone").join("shot.png");
        let good = scratch.path("shot.png");
        let dialogs = Scripted::new(vec![Some(missing_folder.clone()), Some(good.clone())], true);

        run(&image(), &plan(&scratch), &dialogs);

        assert_eq!(*dialogs.failures.borrow(), vec![missing_folder]);
        assert_eq!(
            decode(&good),
            (2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255])
        );
    }

    #[test]
    fn declining_the_retry_discards_the_capture_and_remembers_nothing() {
        let scratch = Scratch::new("abandon");
        let missing_folder = scratch.path("gone").join("shot.png");
        let dialogs = Scripted::new(vec![Some(missing_folder.clone())], false);
        let plan = plan(&scratch);

        run(&image(), &plan, &dialogs);

        assert_eq!(dialogs.failures.borrow().len(), 1);
        assert!(!plan.config_path.unwrap().exists());
    }

    #[test]
    fn the_dialog_reopens_in_the_folder_of_the_failed_attempt() {
        let scratch = Scratch::new("reopen");
        let elsewhere = scratch.path("gone").join("shot.png");
        let dialogs = Scripted::new(vec![Some(elsewhere.clone()), None], true);

        run(&image(), &plan(&scratch), &dialogs);

        assert_eq!(
            dialogs.asked_directories.borrow().last().cloned().flatten(),
            Some(scratch.path("gone"))
        );
    }

    struct Blocking(std::sync::mpsc::Receiver<()>);

    impl Dialogs for Blocking {
        fn pick_path(&self, _directory: Option<&Path>, _file_name: &str) -> Option<PathBuf> {
            self.0.recv().ok();
            None
        }

        fn retry_after_failure(&self, _path: &Path, _error: &io::Error) -> bool {
            false
        }
    }

    #[test]
    fn a_second_capture_is_ignored_while_the_dialog_is_open() {
        let scratch = Scratch::new("busy");
        let saver = Saver::default();
        let (release, gate) = std::sync::mpsc::channel();

        assert!(saver.start(image(), plan(&scratch), Blocking(gate)));
        let (_unused, ignored_gate) = std::sync::mpsc::channel();
        assert!(!saver.start(image(), plan(&scratch), Blocking(ignored_gate)));
        assert!(saver.is_busy());

        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while saver.is_busy() {
            assert!(Instant::now() < deadline, "the save thread never finished");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

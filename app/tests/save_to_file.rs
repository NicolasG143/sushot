#![cfg(windows)]

mod common;

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use klip_core::{Config, Platform};
use klip_platform_windows::WindowsPlatform;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_ESCAPE, VK_MENU, VK_RETURN};
use windows::core::w;

use common::{
    RunningKlip, become_dpi_aware, png_size, press_print_screen_with, tap, type_text,
    wait_for_window,
};

/// Puts the user's real configuration back once the test ends.
struct ConfigBackup {
    path: PathBuf,
    original: Option<String>,
}

impl ConfigBackup {
    fn take() -> Self {
        let path = WindowsPlatform.config_path().expect("a roaming profile");
        let original = fs::read_to_string(&path).ok();
        Self { path, original }
    }
}

impl Drop for ConfigBackup {
    fn drop(&mut self) {
        match &self.original {
            Some(text) => fs::write(&self.path, text).expect("the configuration is restored"),
            None => {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

fn wait_for_file(path: &PathBuf) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::metadata(path).map(|meta| meta.len()).unwrap_or(0) == 0 {
        assert!(
            Instant::now() < deadline,
            "{} was never written",
            path.display()
        );
        thread::sleep(Duration::from_millis(50));
    }
    thread::sleep(Duration::from_millis(300));
}

#[test]
#[ignore = "drives the real save dialog with keystrokes and edits the user's klip configuration"]
fn ctrl_alt_print_screen_saves_a_png_and_remembers_the_folder() {
    become_dpi_aware();
    let _config = ConfigBackup::take();
    let folder = std::env::temp_dir().join(format!("klip-e2e-save-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let target = folder.join("shot.png");
    let _klip = RunningKlip::start();

    press_print_screen_with(&[VK_CONTROL, VK_MENU]);
    wait_for_window(w!("Guardar captura"));
    assert_eq!(common::foreground_title(), "Guardar captura");
    type_text(target.to_str().unwrap());
    tap(VK_RETURN);
    wait_for_file(&target);

    let monitor = WindowsPlatform.monitor_at_cursor().unwrap();
    let png = fs::read(&target).unwrap();
    assert_eq!(png_size(png), (monitor.w, monitor.h));
    let config = Config::parse(&fs::read_to_string(&_config.path).unwrap()).unwrap();
    assert_eq!(config.last_save_dir, Some(folder.clone()));

    press_print_screen_with(&[VK_CONTROL, VK_MENU]);
    wait_for_window(w!("Guardar captura"));
    assert_eq!(common::foreground_title(), "Guardar captura");
    tap(VK_ESCAPE);
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        fs::read_dir(&folder).unwrap().count(),
        1,
        "cancelling saved a file"
    );

    let _ = fs::remove_dir_all(&folder);
}

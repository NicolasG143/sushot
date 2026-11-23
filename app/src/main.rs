#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("klip has no platform backend for this target");

mod capture;
mod logger;

use std::process::ExitCode;
use std::sync::mpsc;

use klip_core::{Platform, PlatformError};
use klip_platform_windows::WindowsPlatform;

fn main() -> ExitCode {
    logger::init();
    match run(&WindowsPlatform) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(platform: &impl Platform) -> Result<(), PlatformError> {
    let (actions_tx, actions_rx) = mpsc::channel();
    platform.register_hotkeys(actions_tx)?;
    log::info!("klip {} ready", env!("CARGO_PKG_VERSION"));

    for action in actions_rx {
        if let Err(error) = capture::perform(platform, action) {
            log::error!("capture failed: {error}");
        }
    }

    log::info!("hotkey listener stopped; exiting");
    Ok(())
}

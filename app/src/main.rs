#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("klip has no platform backend for this target");

mod app;
mod capture;
mod interaction;
mod logger;
mod overlay;

use std::error::Error;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use klip_core::Platform;
use klip_platform_windows::WindowsPlatform;
use winit::event_loop::EventLoop;

use crate::app::{App, UiEvent};

fn main() -> ExitCode {
    logger::init();
    match run(WindowsPlatform) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(platform: impl Platform) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<UiEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let (actions_tx, actions_rx) = mpsc::channel();
    platform.register_hotkeys(actions_tx)?;
    thread::Builder::new()
        .name("klip-actions".into())
        .spawn(move || {
            for action in actions_rx {
                if proxy.send_event(UiEvent::Action(action)).is_err() {
                    break;
                }
            }
        })?;
    log::info!("klip {} ready", env!("CARGO_PKG_VERSION"));

    event_loop.run_app(&mut App::new(platform))?;
    log::info!("event loop stopped; exiting");
    Ok(())
}

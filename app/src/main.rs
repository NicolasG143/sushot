#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("sushot has no platform backend for this target");

mod app;
mod capture;
mod cli;
mod dialogs;
mod icon;
mod interaction;
#[cfg(unix)]
mod ipc;
mod logger;
mod overlay;
mod save;
mod settings;
mod tray;

use std::error::Error;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use sushot_core::{Action, Platform, load_config};
use tray_icon::menu::MenuEvent;
use winit::event_loop::EventLoop;

use crate::app::{App, UiEvent};
use crate::cli::Command;

#[cfg(windows)]
fn backend() -> impl Platform {
    sushot_platform_windows::WindowsPlatform
}

#[cfg(target_os = "linux")]
fn backend() -> impl Platform {
    sushot_platform_linux::LinuxPlatform
}

fn main() -> ExitCode {
    logger::init();
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!(
                "sushot: {}
{}",
                error.0,
                cli::USAGE
            );
            return ExitCode::from(2);
        }
    };
    let platform = backend();
    let outcome = match command {
        Command::Run => run(platform),
        Command::Capture(action) => request_capture(&platform, action),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            eprintln!("sushot: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(unix)]
fn request_capture(platform: &impl Platform, action: Action) -> Result<(), Box<dyn Error>> {
    let path = platform
        .ipc_path()
        .ok_or("capture requests need XDG_RUNTIME_DIR")?;
    ipc::send(&path, action)
}

#[cfg(not(unix))]
fn request_capture(_platform: &impl Platform, _action: Action) -> Result<(), Box<dyn Error>> {
    Err("capture requests are only available on Linux".into())
}

fn run(platform: impl Platform) -> Result<(), Box<dyn Error>> {
    let Some(_instance) = platform.claim_instance()? else {
        log::info!("another sushot instance is already running; exiting");
        return Ok(());
    };

    let config_path = platform.config_path();
    let first_run = config_path.as_deref().is_some_and(|path| !path.exists());
    let config = config_path.as_deref().map(load_config).unwrap_or_default();
    if let Err(error) = platform.set_autostart(config.autostart) {
        log::warn!("autostart entry was not reconciled: {error}");
    }

    let event_loop = EventLoop::<UiEvent>::with_user_event().build()?;
    let menu_proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if menu_proxy.send_event(UiEvent::Menu(event.id)).is_err() {
            log::debug!("menu event dropped: the event loop is closed");
        }
    }));

    #[cfg(unix)]
    if let Some(path) = platform.ipc_path() {
        ipc::listen(&path, event_loop.create_proxy())?;
    }

    let (actions_tx, actions_rx) = mpsc::channel();
    if let Err(error) = platform.register_hotkeys(config.hotkey, actions_tx) {
        log::warn!(
            "{error}; sushot keeps running, use `sushot --region` and its siblings from a system shortcut"
        );
    }
    let action_proxy = event_loop.create_proxy();
    thread::Builder::new()
        .name("sushot-actions".into())
        .spawn(move || {
            for action in actions_rx {
                if action_proxy.send_event(UiEvent::Action(action)).is_err() {
                    break;
                }
            }
        })?;
    log::info!("sushot {} ready", env!("CARGO_PKG_VERSION"));
    if first_run {
        dialogs::announce_first_run();
    }

    let mut app = App::new(platform, config_path);
    let outcome = event_loop.run_app(&mut app);
    app.release();
    outcome?;
    log::info!("event loop stopped; exiting");
    Ok(())
}

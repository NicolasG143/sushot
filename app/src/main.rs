#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("klip has no platform backend for this target");

mod app;
mod capture;
mod dialogs;
mod icon;
mod interaction;
mod logger;
mod overlay;
mod save;
mod settings;
mod tray;

use std::error::Error;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

use klip_core::{Platform, load_config};
use klip_platform_windows::WindowsPlatform;
use tray_icon::menu::MenuEvent;
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
    let Some(_instance) = platform.claim_instance()? else {
        log::info!("another klip instance is already running; exiting");
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

    let (actions_tx, actions_rx) = mpsc::channel();
    platform.register_hotkeys(actions_tx)?;
    let action_proxy = event_loop.create_proxy();
    thread::Builder::new()
        .name("klip-actions".into())
        .spawn(move || {
            for action in actions_rx {
                if action_proxy.send_event(UiEvent::Action(action)).is_err() {
                    break;
                }
            }
        })?;
    log::info!("klip {} ready", env!("CARGO_PKG_VERSION"));
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

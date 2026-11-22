use std::path::PathBuf;
use std::time::{Duration, Instant};

use klip_core::{Action, Mode, Platform, load_config};
use tray_icon::menu::MenuId;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::WindowId;

use crate::capture::{self, Delivery};
use crate::dialogs::NativeDialogs;
use crate::interaction::Outcome;
use crate::overlay::Overlay;
use crate::save::Saver;
use crate::settings;
use crate::tray::Tray;

const FLASH_DURATION: Duration = Duration::from_millis(180);
const TOAST_TITLE: &str = "klip";
const TOAST_TEXT: &str = "Captura copiada al portapapeles";

pub(crate) enum UiEvent {
    Action(Action),
    Menu(MenuId),
}

pub(crate) struct App<P> {
    platform: P,
    config_path: Option<PathBuf>,
    overlay: Option<Overlay>,
    saver: Saver,
    tray: Option<Tray>,
    flash_until: Option<Instant>,
}

impl<P: Platform> App<P> {
    pub(crate) fn new(platform: P, config_path: Option<PathBuf>) -> Self {
        Self {
            platform,
            config_path,
            overlay: None,
            saver: Saver::default(),
            tray: None,
            flash_until: None,
        }
    }

    /// Removes the tray icon and the keyboard hook before the process exits.
    pub(crate) fn release(&mut self) {
        self.tray = None;
        self.platform.unregister_hotkeys();
    }

    fn start(&mut self, event_loop: &ActiveEventLoop, action: Action) {
        if self.overlay.is_some() {
            log::debug!("ignoring {action:?}: a selection is already in progress");
            return;
        }
        if !self.platform.has_selection_overlay() {
            let result = self
                .platform
                .capture_with_system_ui(action.mode)
                .map_err(capture::CaptureError::from)
                .and_then(|frame| {
                    capture::deliver(
                        &self.platform,
                        &self.saver,
                        NativeDialogs,
                        &frame,
                        frame.bounds(),
                        action.output,
                    )
                });
            self.report(result);
            return;
        }
        if action.mode == Mode::FullScreen {
            let result =
                capture::full_screen(&self.platform, &self.saver, NativeDialogs, action.output);
            self.report(result);
            return;
        }
        let desktop = match self.platform.grab_virtual_desktop() {
            Ok(desktop) => desktop,
            Err(error) => {
                log::error!("capture failed: {error}");
                return;
            }
        };
        match Overlay::open(event_loop, &self.platform, action, desktop) {
            Ok(overlay) => self.overlay = Some(overlay),
            Err(error) => log::error!("overlay could not open: {error}"),
        }
    }

    fn finish(&mut self, outcome: Outcome) {
        let Some(overlay) = self.overlay.as_ref() else {
            return;
        };
        let result = match outcome {
            Outcome::Pending => return,
            Outcome::Cancelled => {
                log::info!("selection cancelled");
                None
            }
            Outcome::Confirmed(area) => {
                let area = overlay.to_desktop(area);
                log::debug!("selection confirmed: {area:?}");
                Some(capture::deliver(
                    &self.platform,
                    &self.saver,
                    NativeDialogs,
                    overlay.desktop(),
                    area,
                    overlay.action().output,
                ))
            }
        };
        self.overlay = None;
        if let Some(result) = result {
            self.report(result);
        }
    }

    fn report(&mut self, result: Result<Delivery, capture::CaptureError>) {
        match result {
            Ok(Delivery::Copied) => self.confirm_copy(),
            Ok(Delivery::SaveStarted) => {}
            Err(error) => log::error!("capture failed: {error}"),
        }
    }

    fn confirm_copy(&mut self) {
        let Some(tray) = self.tray.as_ref() else {
            return;
        };
        tray.flash();
        self.flash_until = Some(Instant::now() + FLASH_DURATION);
        let show_toast = self
            .config_path
            .as_deref()
            .is_some_and(|path| load_config(path).show_toast);
        if show_toast {
            tray.notify(TOAST_TITLE, TOAST_TEXT);
        }
    }

    fn toggle_autostart(&self) {
        let Some(tray) = self.tray.as_ref() else {
            return;
        };
        let enabled = tray.autostart_checked();
        let result = settings::set_autostart(self.config_path.as_deref(), enabled, |enabled| {
            self.platform.set_autostart(enabled)
        });
        match result {
            Ok(()) => log::info!("autostart {}", if enabled { "enabled" } else { "disabled" }),
            Err(error) => {
                log::error!("{error}");
                tray.set_autostart_checked(!enabled);
            }
        }
    }
}

impl<P: Platform> ApplicationHandler<UiEvent> for App<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.tray.is_some() {
            return;
        }
        let autostart = self
            .config_path
            .as_deref()
            .is_some_and(|path| load_config(path).autostart);
        match Tray::new(autostart) {
            Ok(tray) => self.tray = Some(tray),
            Err(error) => {
                log::error!("tray icon could not be created: {error}");
                event_loop.exit();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UiEvent) {
        match event {
            UiEvent::Action(action) => self.start(event_loop, action),
            UiEvent::Menu(id) => {
                let Some(tray) = self.tray.as_ref() else {
                    return;
                };
                if id == *tray.quit_id() {
                    log::info!("quit requested from the tray menu");
                    event_loop.exit();
                } else if id == *tray.autostart_id() {
                    self.toggle_autostart();
                }
            }
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };
        if overlay.window_id() != id {
            return;
        }
        if matches!(event, WindowEvent::Resized(_)) {
            overlay.keep_bounds(&self.platform);
            return;
        }
        let outcome = overlay.handle(&self.platform, event);
        self.finish(outcome);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.flash_until {
            Some(until) if Instant::now() >= until => {
                if let Some(tray) = self.tray.as_ref() {
                    tray.restore();
                }
                self.flash_until = None;
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            Some(until) => event_loop.set_control_flow(ControlFlow::WaitUntil(until)),
            None => {}
        }
    }
}

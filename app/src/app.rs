use klip_core::{Action, Mode, Platform};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::capture;
use crate::dialogs::NativeDialogs;
use crate::interaction::Outcome;
use crate::overlay::Overlay;
use crate::save::Saver;

pub(crate) enum UiEvent {
    Action(Action),
}

pub(crate) struct App<P> {
    platform: P,
    overlay: Option<Overlay>,
    saver: Saver,
}

impl<P: Platform> App<P> {
    pub(crate) fn new(platform: P) -> Self {
        Self {
            platform,
            overlay: None,
            saver: Saver::default(),
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop, action: Action) {
        if self.overlay.is_some() {
            log::debug!("ignoring {action:?}: a selection is already in progress");
            return;
        }
        if action.mode == Mode::FullScreen {
            if let Err(error) =
                capture::full_screen(&self.platform, &self.saver, NativeDialogs, action.output)
            {
                log::error!("capture failed: {error}");
            }
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
        match outcome {
            Outcome::Pending => return,
            Outcome::Cancelled => log::info!("selection cancelled"),
            Outcome::Confirmed(area) => {
                let area = overlay.to_desktop(area);
                log::debug!("selection confirmed: {area:?}");
                if let Err(error) = capture::deliver(
                    &self.platform,
                    &self.saver,
                    NativeDialogs,
                    overlay.desktop(),
                    area,
                    overlay.action().output,
                ) {
                    log::error!("capture failed: {error}");
                }
            }
        }
        self.overlay = None;
    }
}

impl<P: Platform> ApplicationHandler<UiEvent> for App<P> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UiEvent) {
        match event {
            UiEvent::Action(action) => self.start(event_loop, action),
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
}

use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use klip_core::{Action, Canvas, Frame, Mode, Platform, Rect, WindowId};
use softbuffer::{Context, Surface};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{CursorIcon, Window, WindowLevel};

use crate::interaction::{Interaction, Outcome};

pub(crate) struct Overlay {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    native_id: WindowId,
    desktop: Frame,
    canvas: Canvas,
    interaction: Interaction,
    action: Action,
    painted: Option<Rect>,
    pending: Option<Rect>,
    presented_once: bool,
}

impl Overlay {
    pub(crate) fn open(
        event_loop: &ActiveEventLoop,
        platform: &impl Platform,
        action: Action,
        desktop: Frame,
    ) -> Result<Self, Box<dyn Error>> {
        let started = Instant::now();
        let bounds = desktop.bounds();
        let attributes = Window::default_attributes()
            .with_title("klip overlay")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_position(PhysicalPosition::new(bounds.x, bounds.y))
            .with_inner_size(PhysicalSize::new(bounds.w, bounds.h));
        let window = Arc::new(event_loop.create_window(attributes)?);
        window.set_cursor(CursorIcon::Crosshair);

        let native_id = native_id(&window)?;
        let context = Context::new(window.clone())?;
        let mut surface = Surface::new(&context, window.clone())?;
        surface.resize(
            NonZeroU32::new(bounds.w).ok_or("overlay width is zero")?,
            NonZeroU32::new(bounds.h).ok_or("overlay height is zero")?,
        )?;

        let canvas = Canvas::new(&desktop);
        platform.place_overlay(native_id, bounds)?;
        let mut overlay = Self {
            window,
            surface,
            native_id,
            desktop,
            canvas,
            interaction: Interaction::new(action.mode),
            action,
            painted: None,
            pending: None,
            presented_once: false,
        };
        overlay.present()?;
        platform.reveal_overlay(native_id)?;
        log::debug!("overlay ready in {:?}", started.elapsed());
        Ok(overlay)
    }

    pub(crate) fn window_id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    pub(crate) fn action(&self) -> Action {
        self.action
    }

    pub(crate) fn desktop(&self) -> &Frame {
        &self.desktop
    }

    /// Converts an overlay-local area into virtual desktop coordinates.
    pub(crate) fn to_desktop(&self, area: Rect) -> Rect {
        let origin = self.desktop.bounds();
        area.translate(origin.x, origin.y)
    }

    pub(crate) fn handle(&mut self, platform: &impl Platform, event: WindowEvent) -> Outcome {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                log::trace!("cursor at {position:?}");
                self.interaction
                    .move_to((position.x.round() as i32, position.y.round() as i32));
                if self.interaction.mode() == Mode::Window {
                    self.interaction.hover(self.window_under_cursor(platform));
                }
                self.refresh();
                Outcome::Pending
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let outcome = match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => self.interaction.press(),
                    (MouseButton::Left, ElementState::Released) => self.interaction.release(),
                    (MouseButton::Right, ElementState::Pressed) => Outcome::Cancelled,
                    _ => Outcome::Pending,
                };
                self.refresh();
                outcome
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if event.logical_key == Key::Named(NamedKey::Escape) {
                    return Outcome::Cancelled;
                }
                if let Some((dx, dy)) = arrow_step(&event.logical_key)
                    && self.interaction.nudge(dx, dy)
                {
                    self.refresh();
                }
                Outcome::Pending
            }
            WindowEvent::ScaleFactorChanged {
                mut inner_size_writer,
                ..
            } => {
                let bounds = self.desktop.bounds();
                if let Err(error) =
                    inner_size_writer.request_inner_size(PhysicalSize::new(bounds.w, bounds.h))
                {
                    log::warn!("overlay size was not kept after a scale change: {error}");
                }
                Outcome::Pending
            }
            WindowEvent::CloseRequested => Outcome::Cancelled,
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.present() {
                    log::error!("overlay redraw failed: {error}");
                }
                Outcome::Pending
            }
            _ => Outcome::Pending,
        }
    }

    fn window_under_cursor(&self, platform: &impl Platform) -> Option<Rect> {
        let origin = self.desktop.bounds();
        let (x, y) = self.interaction.cursor();
        platform
            .window_rect_at(x + origin.x, y + origin.y, self.native_id)
            .map(|window| window.translate(-origin.x, -origin.y))
            .and_then(|window| self.canvas.area().intersection(window))
    }

    fn refresh(&mut self) {
        if self.interaction.highlight() != self.painted {
            self.window.request_redraw();
        }
    }

    /// Paints only at redraw time, so a burst of cursor moves costs a single paint.
    fn paint(&mut self) {
        let highlight = self.interaction.highlight();
        if highlight == self.painted {
            return;
        }
        let dirty = [self.painted, highlight]
            .into_iter()
            .flatten()
            .map(|area| self.canvas.footprint(area))
            .reduce(Rect::union);
        self.painted = highlight;
        let Some(dirty) = dirty else {
            return;
        };
        self.canvas.paint(&self.desktop, highlight, dirty);
        self.pending = Some(self.pending.map_or(dirty, |pending| pending.union(dirty)));
    }

    fn present(&mut self) -> Result<(), Box<dyn Error>> {
        self.paint();
        if self.presented_once && self.pending.is_none() {
            return Ok(());
        }
        let width = self.canvas.width() as usize;
        let mut buffer = self.surface.buffer_mut()?;
        let area = match (buffer.age(), self.pending.take()) {
            (1, Some(pending)) if self.presented_once => pending,
            _ => self.canvas.area(),
        };
        let area = self
            .canvas
            .area()
            .intersection(area)
            .ok_or("dirty area is empty")?;

        let columns = area.x as usize..area.x as usize + area.w as usize;
        for row in area.y as usize..area.y as usize + area.h as usize {
            buffer[row * width + columns.start..row * width + columns.end].copy_from_slice(
                &self.canvas.pixels()[row * width + columns.start..row * width + columns.end],
            );
        }
        let damage = softbuffer::Rect {
            x: area.x as u32,
            y: area.y as u32,
            width: NonZeroU32::new(area.w).ok_or("dirty area has no width")?,
            height: NonZeroU32::new(area.h).ok_or("dirty area has no height")?,
        };
        buffer.present_with_damage(&[damage])?;
        self.presented_once = true;
        Ok(())
    }

    /// Keeps the overlay glued to the virtual desktop when Windows rescales it.
    pub(crate) fn keep_bounds(&self, platform: &impl Platform) {
        let bounds = self.desktop.bounds();
        let size = self.window.inner_size();
        if size.width != bounds.w || size.height != bounds.h {
            log::warn!(
                "overlay is {}x{} instead of {}x{}; placing it again",
                size.width,
                size.height,
                bounds.w,
                bounds.h
            );
            if let Err(error) = platform.place_overlay(self.native_id, bounds) {
                log::error!("{error}");
            }
        }
    }
}

fn arrow_step(key: &Key) -> Option<(i32, i32)> {
    match key {
        Key::Named(NamedKey::ArrowLeft) => Some((-1, 0)),
        Key::Named(NamedKey::ArrowRight) => Some((1, 0)),
        Key::Named(NamedKey::ArrowUp) => Some((0, -1)),
        Key::Named(NamedKey::ArrowDown) => Some((0, 1)),
        _ => None,
    }
}

fn native_id(window: &Window) -> Result<WindowId, Box<dyn Error>> {
    match window.window_handle()?.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(WindowId(handle.hwnd.get())),
        RawWindowHandle::Xlib(handle) => Ok(WindowId(handle.window as isize)),
        RawWindowHandle::Xcb(handle) => Ok(WindowId(handle.window.get() as isize)),
        other => Err(format!("unsupported window handle {other:?}").into()),
    }
}

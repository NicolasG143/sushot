use std::fmt::Display;

use sushot_core::{Frame, PlatformError, Rect, WindowId};
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{
    AtomEnum, ConfigureWindowAux, ConnectionExt as _, ImageFormat, ImageOrder, MapState, StackMode,
    Window,
};
use x11rb::rust_connection::RustConnection;

const BYTES_PER_PIXEL: usize = 4;

pub(crate) fn fail<E: Display>(operation: &'static str) -> impl Fn(E) -> PlatformError {
    move |error| PlatformError::new(operation, error.to_string())
}

pub(crate) struct X11 {
    pub(crate) conn: RustConnection,
    pub(crate) root: Window,
    depth: u8,
}

impl X11 {
    pub(crate) fn connect() -> Result<Self, PlatformError> {
        let (conn, screen_number) =
            x11rb::connect(None).map_err(fail("connect to the X server"))?;
        let screen = &conn.setup().roots[screen_number];
        let (root, depth) = (screen.root, screen.root_depth);
        Ok(Self { conn, root, depth })
    }

    fn atom(&self, name: &[u8]) -> Result<u32, PlatformError> {
        self.conn
            .intern_atom(false, name)
            .map_err(fail("look up an atom"))?
            .reply()
            .map(|reply| reply.atom)
            .map_err(fail("look up an atom"))
    }

    fn cardinals(&self, window: Window, property: &[u8], length: u32) -> Option<Vec<u32>> {
        let atom = self.atom(property).ok()?;
        let reply = self
            .conn
            .get_property(false, window, atom, AtomEnum::ANY, 0, length)
            .ok()?
            .reply()
            .ok()?;
        Some(reply.value32()?.collect())
    }
}

pub(crate) fn grab_virtual_desktop() -> Result<Frame, PlatformError> {
    let x = X11::connect()?;
    let operation = "capture the root window";
    let geometry = x
        .conn
        .get_geometry(x.root)
        .map_err(fail(operation))?
        .reply()
        .map_err(fail(operation))?;

    let setup = x.conn.setup();
    let format = setup
        .pixmap_formats
        .iter()
        .find(|format| format.depth == x.depth)
        .ok_or_else(|| PlatformError::new(operation, "no pixmap format for the root depth"))?;
    if format.bits_per_pixel != 32 || setup.image_byte_order != ImageOrder::LSB_FIRST {
        return Err(PlatformError::new(
            operation,
            format!(
                "unsupported pixel layout: {} bits per pixel, {:?} byte order",
                format.bits_per_pixel, setup.image_byte_order
            ),
        ));
    }

    let image = x
        .conn
        .get_image(
            ImageFormat::Z_PIXMAP,
            x.root,
            0,
            0,
            geometry.width,
            geometry.height,
            u32::MAX,
        )
        .map_err(fail(operation))?
        .reply()
        .map_err(fail(operation))?;

    let bounds = Rect {
        x: 0,
        y: 0,
        w: u32::from(geometry.width),
        h: u32::from(geometry.height),
    };
    Frame::new(bounds, bgrx_to_rgba(&image.data))
        .map_err(|error| PlatformError::new("build captured frame", error.to_string()))
}

fn bgrx_to_rgba(bgrx: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgrx.len());
    for pixel in bgrx.as_chunks::<BYTES_PER_PIXEL>().0 {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], u8::MAX]);
    }
    rgba
}

pub(crate) fn monitor_at_cursor() -> Result<Rect, PlatformError> {
    let x = X11::connect()?;
    let pointer = x
        .conn
        .query_pointer(x.root)
        .map_err(fail("read the pointer position"))?
        .reply()
        .map_err(fail("read the pointer position"))?;
    let monitors = x
        .conn
        .randr_get_monitors(x.root, true)
        .map_err(fail("list monitors"))?
        .reply()
        .map_err(fail("list monitors"))?
        .monitors;

    let rects: Vec<Rect> = monitors
        .iter()
        .map(|monitor| Rect {
            x: i32::from(monitor.x),
            y: i32::from(monitor.y),
            w: u32::from(monitor.width),
            h: u32::from(monitor.height),
        })
        .collect();
    pick_monitor(&rects, i32::from(pointer.root_x), i32::from(pointer.root_y))
        .ok_or_else(|| PlatformError::new("list monitors", "the server reports no active monitor"))
}

fn pick_monitor(monitors: &[Rect], x: i32, y: i32) -> Option<Rect> {
    monitors
        .iter()
        .copied()
        .find(|monitor| monitor.contains(x, y))
        .or_else(|| monitors.first().copied())
}

pub(crate) fn window_rect_at(x: i32, y: i32, exclude: WindowId) -> Option<Rect> {
    let server = X11::connect().ok()?;
    let stacking = server.cardinals(server.root, b"_NET_CLIENT_LIST_STACKING", u32::MAX)?;
    for &window in stacking.iter().rev() {
        if i64::from(window) == exclude.0 as i64 {
            continue;
        }
        if let Some(bounds) = visible_bounds(&server, window)
            && bounds.contains(x, y)
        {
            return Some(bounds);
        }
    }
    None
}

fn visible_bounds(server: &X11, window: Window) -> Option<Rect> {
    let attributes = server
        .conn
        .get_window_attributes(window)
        .ok()?
        .reply()
        .ok()?;
    if attributes.map_state != MapState::VIEWABLE {
        return None;
    }
    let geometry = server.conn.get_geometry(window).ok()?.reply().ok()?;
    let origin = server
        .conn
        .translate_coordinates(window, server.root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    let extents = server
        .cardinals(window, b"_NET_FRAME_EXTENTS", 4)
        .filter(|extents| extents.len() == 4)
        .unwrap_or_else(|| vec![0; 4]);
    Some(Rect {
        x: i32::from(origin.dst_x) - extents[0] as i32,
        y: i32::from(origin.dst_y) - extents[2] as i32,
        w: u32::from(geometry.width) + extents[0] + extents[1],
        h: u32::from(geometry.height) + extents[2] + extents[3],
    })
}

pub(crate) fn place_overlay(window: WindowId, area: Rect) -> Result<(), PlatformError> {
    let operation = "place overlay window";
    let x = X11::connect()?;
    let placement = ConfigureWindowAux::new()
        .x(area.x)
        .y(area.y)
        .width(area.w)
        .height(area.h)
        .stack_mode(StackMode::ABOVE);
    x.conn
        .configure_window(window.0 as u32, &placement)
        .map_err(fail(operation))?
        .check()
        .map_err(fail(operation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_swap_to_rgba_with_full_opacity() {
        assert_eq!(
            bgrx_to_rgba(&[1, 2, 3, 0, 4, 5, 6, 77]),
            vec![3, 2, 1, 255, 6, 5, 4, 255]
        );
    }

    #[test]
    fn monitor_under_the_pointer_wins_and_the_first_is_the_fallback() {
        let left = Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let right = Rect {
            x: 100,
            y: 0,
            w: 100,
            h: 100,
        };

        assert_eq!(pick_monitor(&[left, right], 150, 10), Some(right));
        assert_eq!(pick_monitor(&[left, right], 900, 900), Some(left));
        assert_eq!(pick_monitor(&[], 0, 0), None);
    }
}

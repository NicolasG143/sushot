use std::ffi::c_void;
use std::ptr;
use std::slice;

use klip_core::{Frame, PlatformError, Rect};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GdiFlush, GetDC, HBITMAP, HDC, HGDIOBJ, ROP_CODE,
    ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::error::{last_os_error, os_error};

const BYTES_PER_PIXEL: usize = 4;

pub(crate) fn grab_virtual_desktop() -> Result<Frame, PlatformError> {
    let bounds = virtual_desktop_bounds()?;
    let screen = ScreenDc::acquire()?;
    let memory = MemoryDc::compatible_with(&screen)?;
    let bitmap = TopDownBitmap::create(&screen, bounds)?;
    let _selection = Selection::select(&memory, &bitmap);

    unsafe {
        BitBlt(
            memory.0,
            0,
            0,
            bounds.w as i32,
            bounds.h as i32,
            Some(screen.0),
            bounds.x,
            bounds.y,
            ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
        )
    }
    .map_err(os_error("copy screen contents"))?;

    Frame::new(bounds, bitmap.to_rgba())
        .map_err(|error| PlatformError::new("build captured frame", error.to_string()))
}

fn virtual_desktop_bounds() -> Result<Rect, PlatformError> {
    let (x, y, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    if width <= 0 || height <= 0 {
        return Err(PlatformError::new(
            "measure virtual desktop",
            format!("reported size is {width}x{height}"),
        ));
    }
    Ok(Rect {
        x,
        y,
        w: width as u32,
        h: height as u32,
    })
}

struct ScreenDc(HDC);

impl ScreenDc {
    fn acquire() -> Result<Self, PlatformError> {
        let dc = unsafe { GetDC(None) };
        if dc.is_invalid() {
            return Err(last_os_error("open screen device context"));
        }
        Ok(Self(dc))
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        if unsafe { ReleaseDC(None, self.0) } == 0 {
            log::warn!("screen device context was not released");
        }
    }
}

struct MemoryDc(HDC);

impl MemoryDc {
    fn compatible_with(screen: &ScreenDc) -> Result<Self, PlatformError> {
        let dc = unsafe { CreateCompatibleDC(Some(screen.0)) };
        if dc.is_invalid() {
            return Err(last_os_error("create memory device context"));
        }
        Ok(Self(dc))
    }
}

impl Drop for MemoryDc {
    fn drop(&mut self) {
        if !unsafe { DeleteDC(self.0) }.as_bool() {
            log::warn!("memory device context was not deleted");
        }
    }
}

struct TopDownBitmap {
    handle: HBITMAP,
    bgra: *const u8,
    len: usize,
}

impl TopDownBitmap {
    fn create(screen: &ScreenDc, bounds: Rect) -> Result<Self, PlatformError> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bounds.w as i32,
                // A negative height asks GDI for rows ordered top to bottom.
                biHeight: -(bounds.h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = ptr::null_mut();
        let handle =
            unsafe { CreateDIBSection(Some(screen.0), &info, DIB_RGB_COLORS, &mut bits, None, 0) }
                .map_err(os_error("allocate capture bitmap"))?;

        Ok(Self {
            handle,
            bgra: bits.cast(),
            len: bounds.w as usize * bounds.h as usize * BYTES_PER_PIXEL,
        })
    }

    fn to_rgba(&self) -> Vec<u8> {
        // GDI batches drawing calls; the pixels are only guaranteed to be in the
        // section after a flush.
        if !unsafe { GdiFlush() }.as_bool() {
            log::warn!("pending GDI drawing calls were not flushed before reading the capture");
        }
        // SAFETY: CreateDIBSection allocated `len` bytes at `bgra`, and they stay
        // valid until the bitmap handle is deleted in `drop`.
        let mut pixels = unsafe { slice::from_raw_parts(self.bgra, self.len) }.to_vec();
        for pixel in pixels.as_chunks_mut::<BYTES_PER_PIXEL>().0 {
            pixel.swap(0, 2);
            // GDI leaves the fourth byte at zero, which would read as fully transparent.
            pixel[3] = u8::MAX;
        }
        pixels
    }
}

impl Drop for TopDownBitmap {
    fn drop(&mut self) {
        if !unsafe { DeleteObject(HGDIOBJ(self.handle.0)) }.as_bool() {
            log::warn!("capture bitmap was not deleted");
        }
    }
}

struct Selection<'a> {
    memory: &'a MemoryDc,
    previous: HGDIOBJ,
}

impl<'a> Selection<'a> {
    fn select(memory: &'a MemoryDc, bitmap: &TopDownBitmap) -> Self {
        let previous = unsafe { SelectObject(memory.0, HGDIOBJ(bitmap.handle.0)) };
        Self { memory, previous }
    }
}

impl Drop for Selection<'_> {
    fn drop(&mut self) {
        unsafe { SelectObject(self.memory.0, self.previous) };
    }
}

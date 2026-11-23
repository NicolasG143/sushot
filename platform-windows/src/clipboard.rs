use std::mem;
use std::slice;
use std::thread;
use std::time::Duration;

use klip_core::{Frame, PlatformError, encode_png};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_DIBV5;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
};
use windows::core::{PCWSTR, w};

use crate::dib_v5;
use crate::error::{last_os_error, os_error};

const OPEN_ATTEMPTS: u32 = 5;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(10);

/// Publishes `image` as "PNG" and as CF_DIBV5. Both payloads are built before the
/// clipboard is opened so that it stays locked for as little time as possible.
pub(crate) fn write_image(image: &Frame) -> Result<(), PlatformError> {
    let png = encode_png(image)
        .map_err(|error| PlatformError::new("encode capture as PNG", error.to_string()))?;
    let png_payload = GlobalPayload::filled(png.len(), |bytes| bytes.copy_from_slice(&png))?;
    let dib_payload = GlobalPayload::filled(dib_v5::byte_len(image)?, |bytes| {
        dib_v5::write(image, bytes)
    })?;
    let png_format = register_png_format()?;

    let owner = OwnerWindow::create()?;
    let _clipboard = OpenedClipboard::acquire(&owner)?;
    unsafe { EmptyClipboard() }.map_err(os_error("empty clipboard"))?;
    // Some applications take the first format they understand, and PNG is the one
    // that keeps the image intact everywhere.
    png_payload.publish(png_format)?;
    dib_payload.publish(u32::from(CF_DIBV5.0))
}

fn register_png_format() -> Result<u32, PlatformError> {
    match unsafe { RegisterClipboardFormatW(w!("PNG")) } {
        0 => Err(last_os_error("register PNG clipboard format")),
        format => Ok(format),
    }
}

/// A clipboard opened without an owner window can be taken over by any other
/// process that opens it the same way, which makes the pending writes fail.
struct OwnerWindow(HWND);

impl OwnerWindow {
    fn create() -> Result<Self, PlatformError> {
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                PCWSTR::null(),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .map(Self)
        .map_err(os_error("create clipboard owner window"))
    }
}

impl Drop for OwnerWindow {
    fn drop(&mut self) {
        if let Err(error) = unsafe { DestroyWindow(self.0) } {
            log::warn!("clipboard owner window was not destroyed: {error}");
        }
    }
}

struct OpenedClipboard;

impl OpenedClipboard {
    fn acquire(owner: &OwnerWindow) -> Result<Self, PlatformError> {
        let mut attempt = 1;
        loop {
            match unsafe { OpenClipboard(Some(owner.0)) } {
                Ok(()) => return Ok(Self),
                Err(error) if attempt == OPEN_ATTEMPTS => {
                    return Err(os_error("open clipboard")(error));
                }
                Err(error) => {
                    log::debug!("clipboard is busy on attempt {attempt}: {error}");
                    attempt += 1;
                    thread::sleep(OPEN_RETRY_DELAY);
                }
            }
        }
    }
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        if let Err(error) = unsafe { CloseClipboard() } {
            log::warn!("clipboard was not closed: {error}");
        }
    }
}

struct GlobalPayload(HGLOBAL);

impl GlobalPayload {
    fn filled(len: usize, fill: impl FnOnce(&mut [u8])) -> Result<Self, PlatformError> {
        let payload = unsafe { GlobalAlloc(GMEM_MOVEABLE, len) }
            .map(Self)
            .map_err(os_error("allocate clipboard memory"))?;

        let bytes = unsafe { GlobalLock(payload.0) };
        if bytes.is_null() {
            return Err(last_os_error("lock clipboard memory"));
        }
        // SAFETY: GlobalAlloc returned a block of `len` bytes and it stays locked,
        // and therefore in place, until GlobalUnlock below.
        fill(unsafe { slice::from_raw_parts_mut(bytes.cast::<u8>(), len) });

        // GlobalUnlock reports failure with a success code when it releases the last lock.
        if let Err(error) = unsafe { GlobalUnlock(payload.0) }
            && error.code().is_err()
        {
            return Err(os_error("unlock clipboard memory")(error));
        }
        Ok(payload)
    }

    fn publish(self, format: u32) -> Result<(), PlatformError> {
        unsafe { SetClipboardData(format, Some(HANDLE(self.0.0))) }
            .map_err(os_error("set clipboard data"))?;
        // The system owns the memory once the clipboard accepted it.
        mem::forget(self);
        Ok(())
    }
}

impl Drop for GlobalPayload {
    fn drop(&mut self) {
        if let Err(error) = unsafe { GlobalFree(Some(self.0)) } {
            log::warn!("clipboard memory was not freed: {error}");
        }
    }
}

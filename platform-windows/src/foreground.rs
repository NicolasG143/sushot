use std::num::NonZeroIsize;

use sushot_core::PlatformError;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VK_F24,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DestroyWindow, GetForegroundWindow,
    GetWindowThreadProcessId, SetForegroundWindow, WINDOW_EX_STYLE, WS_POPUP, WS_VISIBLE,
};
use windows::core::w;

use crate::error::os_error;
use crate::monitor;

/// An invisible top-level window that holds the foreground, so the dialogs it owns open
/// in front of the active application instead of behind it. Windows activates a new
/// window only when its thread owns the foreground, so create the owner on the thread
/// that shows the dialogs. It also gives the dialogs a taskbar button labelled "sushot".
pub struct ForegroundOwner(HWND);

impl ForegroundOwner {
    pub fn new() -> Result<Self, PlatformError> {
        let (x, y) = monitor::monitor_at_cursor()
            .inspect_err(|error| log::debug!("dialog owner placed at the origin: {error}"))
            .map(|area| (area.x + area.w as i32 / 2, area.y + area.h as i32 / 2))
            .unwrap_or_default();
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("sushot"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .map_err(os_error("create dialog owner window"))?;
        let owner = Self(window);
        owner.take_foreground();
        Ok(owner)
    }

    pub fn handle(&self) -> NonZeroIsize {
        NonZeroIsize::new(self.0.0 as isize).expect("CreateWindowExW never returns a null window")
    }

    pub fn is_foreground(&self) -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        foreground == self.0
    }

    /// The injected key makes this process the owner of the latest input, which is what
    /// Windows checks before honoring `SetForegroundWindow`. Input from the user that
    /// arrives in between can revoke that, so attaching to the input queue of the
    /// foreground thread is the fallback.
    fn take_foreground(&self) {
        inject_unused_key();
        if unsafe { SetForegroundWindow(self.0) }.as_bool() && self.is_foreground() {
            return;
        }

        let foreground = unsafe { GetForegroundWindow() };
        let their_thread = unsafe { GetWindowThreadProcessId(foreground, None) };
        let our_thread = unsafe { GetCurrentThreadId() };
        if their_thread != 0 && their_thread != our_thread {
            let attached = unsafe { AttachThreadInput(our_thread, their_thread, true) }.as_bool();
            let _ = unsafe { BringWindowToTop(self.0) };
            let _ = unsafe { SetForegroundWindow(self.0) };
            if attached {
                let _ = unsafe { AttachThreadInput(our_thread, their_thread, false) };
            }
        }

        if !self.is_foreground() {
            log::warn!("dialog owner did not get the foreground; the dialog may open behind");
        }
    }
}

impl Drop for ForegroundOwner {
    fn drop(&mut self) {
        if let Err(error) = unsafe { DestroyWindow(self.0) } {
            log::warn!("dialog owner window was not destroyed: {error}");
        }
    }
}

fn inject_unused_key() {
    let key = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_F24,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let inputs = [key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)];
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        log::warn!("foreground permission was not granted: {sent} of 2 inputs were accepted");
    }
}

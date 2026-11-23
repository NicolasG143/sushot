use sushot_core::PlatformError;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_INFO, NIIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
};

const OPERATION: &str = "show tray notification";

/// Raises a notification, which Windows 10 and later render as a toast, from the tray
/// icon identified by `window` and `icon_id`.
pub(crate) fn show(
    window: isize,
    icon_id: u32,
    title: &str,
    text: &str,
) -> Result<(), PlatformError> {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(window as *mut _),
        uID: icon_id,
        uFlags: NIF_INFO,
        dwInfoFlags: NIIF_INFO,
        ..Default::default()
    };
    copy_truncated(&mut data.szInfoTitle, title);
    copy_truncated(&mut data.szInfo, text);
    if unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
        Ok(())
    } else {
        Err(PlatformError::new(
            OPERATION,
            "the shell rejected the notification",
        ))
    }
}

fn copy_truncated(target: &mut [u16], text: &str) {
    let capacity = target.len() - 1;
    for (slot, unit) in target.iter_mut().zip(text.encode_utf16().take(capacity)) {
        *slot = unit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_cut_to_the_buffer_and_keeps_its_terminator() {
        let mut buffer = [0u16; 4];

        copy_truncated(&mut buffer, "abcdef");

        assert_eq!(buffer, [97, 98, 99, 0]);
    }

    #[test]
    fn short_text_is_copied_whole() {
        let mut buffer = [0u16; 8];

        copy_truncated(&mut buffer, "ok");

        assert_eq!(buffer, [111, 107, 0, 0, 0, 0, 0, 0]);
    }
}

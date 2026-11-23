#![cfg(windows)]

use std::io::{BufRead, BufReader, Cursor};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{ptr, slice};

use klip_core::Platform;
use klip_platform_windows::WindowsPlatform;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::Graphics::Gdi::BITMAPV5HEADER;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_DIBV5;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CONTROL, VK_SNAPSHOT,
};
use windows::core::w;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

struct RunningKlip(Child);

impl RunningKlip {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_klip"))
            .stderr(Stdio::piped())
            .spawn()
            .expect("klip starts");
        let log = BufReader::new(child.stderr.take().expect("stderr is piped"));
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::spawn(move || {
            for line in log.lines().map_while(Result::ok) {
                eprintln!("{line}");
                if line.contains("ready") && ready_tx.send(()).is_err() {
                    break;
                }
            }
        });
        ready_rx
            .recv_timeout(STARTUP_TIMEOUT)
            .expect("klip reports that the hook is installed");
        Self(child)
    }
}

impl Drop for RunningKlip {
    fn drop(&mut self) {
        self.0.kill().expect("klip is still running");
        self.0.wait().expect("klip exit status is readable");
    }
}

/// Opens the clipboard without an owner window, the way a clipboard manager that
/// polls for changes does.
struct PollerClipboard;

impl PollerClipboard {
    fn open() -> Option<Self> {
        unsafe { OpenClipboard(None) }.ok().map(|()| Self)
    }

    fn bytes_of(&self, format: u32) -> Option<Vec<u8>> {
        let handle = HGLOBAL(unsafe { GetClipboardData(format) }.ok()?.0);
        let data = unsafe { GlobalLock(handle) };
        assert!(!data.is_null(), "clipboard memory locks");
        let bytes =
            unsafe { slice::from_raw_parts(data.cast::<u8>(), GlobalSize(handle)) }.to_vec();
        let _ = unsafe { GlobalUnlock(handle) };
        Some(bytes)
    }
}

impl Drop for PollerClipboard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

struct CopiedImage {
    png: Vec<u8>,
    dib: Vec<u8>,
}

fn empty_clipboard() {
    let _clipboard = PollerClipboard::open().expect("the clipboard opens");
    unsafe { EmptyClipboard() }.expect("the clipboard empties");
}

fn wait_for_copied_image() -> CopiedImage {
    let png_format = unsafe { RegisterClipboardFormatW(w!("PNG")) };
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    loop {
        if let Some(clipboard) = PollerClipboard::open()
            && let Some(png) = clipboard.bytes_of(png_format)
        {
            let dib = clipboard
                .bytes_of(u32::from(CF_DIBV5.0))
                .expect("CF_DIBV5 is published");
            return CopiedImage { png, dib };
        }
        assert!(Instant::now() < deadline, "no image reached the clipboard");
        thread::sleep(POLL_INTERVAL);
    }
}

fn key(virtual_key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn press_ctrl_print_screen() {
    let pressed = KEYBD_EVENT_FLAGS(0);
    let keystrokes = [
        key(VK_CONTROL, pressed),
        key(VK_SNAPSHOT, pressed),
        key(VK_SNAPSHOT, KEYEVENTF_KEYUP),
        key(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe { SendInput(&keystrokes, size_of::<INPUT>() as i32) };
    assert_eq!(
        sent as usize,
        keystrokes.len(),
        "the system rejected the injected keystrokes"
    );
}

#[test]
#[ignore = "sends real keystrokes and overwrites the clipboard; needs an interactive desktop"]
fn ctrl_print_screen_copies_the_monitor_under_the_cursor() {
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
        .expect("the test process becomes per-monitor DPI aware");
    empty_clipboard();
    let _klip = RunningKlip::start();

    press_ctrl_print_screen();
    let copied = wait_for_copied_image();

    let monitor = WindowsPlatform
        .monitor_at_cursor()
        .expect("the cursor is on a monitor");
    let expected_size = (monitor.w, monitor.h);

    let mut png = png::Decoder::new(Cursor::new(copied.png))
        .read_info()
        .expect("PNG header");
    let mut rgba = vec![0; png.output_buffer_size().expect("PNG fits in memory")];
    let info = png.next_frame(&mut rgba).expect("PNG pixels");
    assert_eq!((info.width, info.height), expected_size, "PNG size");
    assert!(
        rgba.as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == u8::MAX),
        "PNG is opaque"
    );

    assert!(
        copied.dib.len() >= size_of::<BITMAPV5HEADER>(),
        "CF_DIBV5 holds a V5 header"
    );
    let header = unsafe { ptr::read_unaligned(copied.dib.as_ptr().cast::<BITMAPV5HEADER>()) };
    assert_eq!(
        (header.bV5Width as u32, header.bV5Height as u32),
        expected_size,
        "CF_DIBV5 size"
    );
}

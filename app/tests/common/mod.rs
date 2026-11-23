#![allow(dead_code)]

use std::io::{BufRead, BufReader, Cursor};
use std::process::{Child, Command, Stdio};
use std::slice;
use std::sync::{Once, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_DIBV5;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
    SendInput, VIRTUAL_KEY, VK_SNAPSHOT,
};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible, SetCursorPos};
use windows::core::{PCWSTR, w};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct RunningSushot(Child);

impl RunningSushot {
    pub fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sushot"))
            .stderr(Stdio::piped())
            .spawn()
            .expect("sushot starts");
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
            .expect("sushot reports that the hook is installed");
        Self(child)
    }
}

impl Drop for RunningSushot {
    fn drop(&mut self) {
        self.0.kill().expect("sushot is still running");
        self.0.wait().expect("sushot exit status is readable");
    }
}

/// Opens the clipboard without an owner window, the way a clipboard manager that
/// polls for changes does.
pub struct PollerClipboard;

impl PollerClipboard {
    pub fn open() -> Option<Self> {
        unsafe { OpenClipboard(None) }.ok().map(|()| Self)
    }

    pub fn bytes_of(&self, format: u32) -> Option<Vec<u8>> {
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

pub struct CopiedImage {
    pub png: Vec<u8>,
    pub dib: Vec<u8>,
}

pub fn empty_clipboard() {
    let _clipboard = PollerClipboard::open().expect("the clipboard opens");
    unsafe { EmptyClipboard() }.expect("the clipboard empties");
}

pub fn wait_for_copied_image() -> CopiedImage {
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

pub fn key(virtual_key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
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

pub fn become_dpi_aware() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
            .expect("the test process becomes per-monitor DPI aware");
    });
}

pub fn press_print_screen_with(modifiers: &[VIRTUAL_KEY]) {
    let pressed = KEYBD_EVENT_FLAGS(0);
    let mut keystrokes: Vec<INPUT> = modifiers.iter().map(|&m| key(m, pressed)).collect();
    keystrokes.push(key(VK_SNAPSHOT, pressed));
    keystrokes.push(key(VK_SNAPSHOT, KEYEVENTF_KEYUP));
    keystrokes.extend(modifiers.iter().rev().map(|&m| key(m, KEYEVENTF_KEYUP)));
    send(&keystrokes);
}

pub fn tap(virtual_key: VIRTUAL_KEY) {
    send(&[
        key(virtual_key, KEYBD_EVENT_FLAGS(0)),
        key(virtual_key, KEYEVENTF_KEYUP),
    ]);
}

pub fn move_cursor(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y) }.expect("the cursor moves");
    thread::sleep(Duration::from_millis(60));
}

pub fn left_button(down: bool) {
    let flags: MOUSE_EVENT_FLAGS = if down {
        MOUSEEVENTF_LEFTDOWN
    } else {
        MOUSEEVENTF_LEFTUP
    };
    send(&[INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    }]);
    thread::sleep(Duration::from_millis(60));
}

fn send(inputs: &[INPUT]) {
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    assert_eq!(
        sent as usize,
        inputs.len(),
        "the system rejected the injected input"
    );
}

pub fn overlay_is_visible() -> bool {
    unsafe { FindWindowW(None, w!("sushot overlay")) }
        .is_ok_and(|window| unsafe { IsWindowVisible(window) }.as_bool())
}

pub fn wait_for_overlay(visible: bool) {
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    while overlay_is_visible() != visible {
        assert!(
            Instant::now() < deadline,
            "overlay visibility never became {visible}"
        );
        thread::sleep(POLL_INTERVAL);
    }
}

pub fn png_size(png: Vec<u8>) -> (u32, u32) {
    let mut decoder = png::Decoder::new(Cursor::new(png))
        .read_info()
        .expect("PNG header");
    let mut rgba = vec![0; decoder.output_buffer_size().expect("PNG fits in memory")];
    let info = decoder.next_frame(&mut rgba).expect("PNG pixels");
    (info.width, info.height)
}

pub fn type_text(text: &str) {
    let mut inputs = Vec::new();
    for unit in text.encode_utf16() {
        for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wScan: unit,
                        dwFlags: flags,
                        ..Default::default()
                    },
                },
            });
        }
    }
    send(&inputs);
}

pub fn wait_for_window(title: PCWSTR) {
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    while unsafe { FindWindowW(None, title) }.is_err() {
        assert!(Instant::now() < deadline, "the dialog never appeared");
        thread::sleep(POLL_INTERVAL);
    }
    thread::sleep(Duration::from_millis(400));
}

pub fn foreground_title() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    let mut title = [0u16; 256];
    let length = unsafe { GetWindowTextW(GetForegroundWindow(), &mut title) };
    String::from_utf16_lossy(&title[..length.max(0) as usize])
}

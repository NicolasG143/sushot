use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use klip_core::{Action, PlatformError};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT, VK_SNAPSHOT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PostQuitMessage,
    PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::error::os_error;
use crate::snapshot_key::{Modifiers, SnapshotKey, Transition, Verdict};

const INSTALL: &str = "install keyboard hook";
const SWALLOWED: LRESULT = LRESULT(1);

struct Listener {
    key: SnapshotKey,
    actions: Sender<Action>,
}

struct HookThread {
    id: u32,
    handle: JoinHandle<()>,
}

static HOOK_THREAD: Mutex<Option<HookThread>> = Mutex::new(None);

thread_local! {
    static LISTENER: RefCell<Option<Listener>> = const { RefCell::new(None) };
}

pub(crate) fn register(actions: Sender<Action>) -> Result<(), PlatformError> {
    let (installed_tx, installed_rx) = mpsc::channel();
    let handle = thread::Builder::new()
        .name("klip-hotkeys".into())
        .spawn(move || run(actions, installed_tx))
        .map_err(|error| PlatformError::new(INSTALL, error.to_string()))?;

    let id = installed_rx
        .recv()
        .map_err(|_| PlatformError::new(INSTALL, "hook thread ended before reporting"))??;
    if let Some(previous) = lock_hook_thread().replace(HookThread { id, handle }) {
        log::warn!("a keyboard hook was already installed; stopping the previous one");
        stop(previous);
    }
    Ok(())
}

/// Removes the hook and waits until its thread is gone.
pub(crate) fn unregister() {
    if let Some(hook) = lock_hook_thread().take() {
        stop(hook);
    }
}

fn lock_hook_thread() -> std::sync::MutexGuard<'static, Option<HookThread>> {
    HOOK_THREAD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn stop(hook: HookThread) {
    if let Err(error) = unsafe { PostThreadMessageW(hook.id, WM_QUIT, WPARAM(0), LPARAM(0)) } {
        log::warn!("keyboard hook thread could not be asked to stop: {error}");
        return;
    }
    if hook.handle.join().is_err() {
        log::error!("keyboard hook thread panicked while stopping");
    }
}

fn run(actions: Sender<Action>, installed: Sender<Result<u32, PlatformError>>) {
    LISTENER.set(Some(Listener {
        key: SnapshotKey::default(),
        actions,
    }));

    let hook = match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) } {
        Ok(hook) => hook,
        Err(error) => {
            if installed.send(Err(os_error(INSTALL)(error))).is_err() {
                log::error!("keyboard hook failed to install and nobody waits for the result");
            }
            return;
        }
    };
    if installed.send(Ok(unsafe { GetCurrentThreadId() })).is_err() {
        log::warn!("keyboard hook installed but nobody waits for the result");
    }
    log::info!("keyboard hook installed");

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {}

    match unsafe { UnhookWindowsHookEx(hook) } {
        Ok(()) => log::info!("keyboard hook removed"),
        Err(error) => log::warn!("keyboard hook was not removed: {error}"),
    }
}

/// Windows silently removes a low-level hook whose callback is slow, so this only
/// classifies the key and hands the action to another thread.
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for HC_ACTION the system passes a KBDLLHOOKSTRUCT that outlives the call.
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if event.vkCode == u32::from(VK_SNAPSHOT.0)
            && let Some(transition) = transition_of(wparam.0 as u32)
            && swallows(transition)
        {
            return SWALLOWED;
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn transition_of(message: u32) -> Option<Transition> {
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN => Some(Transition::Pressed),
        WM_KEYUP | WM_SYSKEYUP => Some(Transition::Released),
        _ => None,
    }
}

fn swallows(transition: Transition) -> bool {
    LISTENER.with_borrow_mut(|listener| {
        let Some(listener) = listener else {
            return false;
        };
        match listener.key.on_transition(transition, pressed_modifiers()) {
            Verdict::PassThrough => false,
            Verdict::Swallow => true,
            Verdict::Trigger(action) => {
                log::debug!("hotkey resolved to {action:?}");
                if listener.actions.send(action).is_err() {
                    log::info!("nobody receives actions anymore; stopping the keyboard hook");
                    unsafe { PostQuitMessage(0) };
                }
                true
            }
        }
    })
}

fn pressed_modifiers() -> Modifiers {
    Modifiers {
        ctrl: is_pressed(VK_CONTROL),
        shift: is_pressed(VK_SHIFT),
        alt: is_pressed(VK_MENU),
    }
}

fn is_pressed(key: VIRTUAL_KEY) -> bool {
    let state = unsafe { GetAsyncKeyState(i32::from(key.0)) };
    state.is_negative()
}

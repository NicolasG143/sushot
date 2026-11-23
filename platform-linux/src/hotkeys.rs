use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use klip_core::{Action, PlatformError, resolve};
use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConnectionExt as _, CreateWindowAux, EventMask, GrabMode,
    Keycode, ModMask, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;

use crate::x11::{X11, fail};

const KEYSYM_PRINT: u32 = 0xff61;
const INSTALL: &str = "grab Print Screen";

struct Listener {
    stop_window: Window,
    handle: JoinHandle<()>,
}

static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<Listener>> {
    LISTENER.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn register(actions: Sender<Action>) -> Result<(), PlatformError> {
    let server = X11::connect()?;
    let keycode = print_keycode(&server.conn)?;
    for (modifiers, _) in modifier_combinations() {
        for locks in lock_variants() {
            server
                .conn
                .grab_key(
                    true,
                    server.root,
                    ModMask::from(modifiers | locks),
                    keycode,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )
                .map_err(fail(INSTALL))?
                .check()
                .map_err(fail(INSTALL))?;
        }
    }

    let stop_window = server.conn.generate_id().map_err(fail(INSTALL))?;
    server
        .conn
        .create_window(
            COPY_DEPTH_FROM_PARENT,
            stop_window,
            server.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            0,
            &CreateWindowAux::new(),
        )
        .map_err(fail(INSTALL))?
        .check()
        .map_err(fail(INSTALL))?;

    let handle = thread::Builder::new()
        .name("klip-hotkeys".into())
        .spawn(move || listen(server, keycode, stop_window, actions))
        .map_err(fail(INSTALL))?;
    if let Some(previous) = lock().replace(Listener {
        stop_window,
        handle,
    }) {
        log::warn!("hotkeys were already registered; stopping the previous listener");
        stop(previous);
    }
    log::info!("Print Screen grabbed on the X root window");
    Ok(())
}

pub(crate) fn unregister() {
    if let Some(listener) = lock().take() {
        stop(listener);
    }
}

fn stop(listener: Listener) {
    let wake = X11::connect().and_then(|server| {
        let message =
            ClientMessageEvent::new(32, listener.stop_window, AtomEnum::CARDINAL, [0u32; 5]);
        server
            .conn
            .send_event(false, listener.stop_window, EventMask::NO_EVENT, message)
            .map_err(fail("wake the hotkey listener"))?;
        server
            .conn
            .flush()
            .map_err(fail("wake the hotkey listener"))
    });
    if let Err(error) = wake {
        log::warn!("hotkey listener was not woken: {error}");
        return;
    }
    if listener.handle.join().is_err() {
        log::error!("hotkey listener panicked while stopping");
    }
}

fn listen(server: X11, keycode: Keycode, stop_window: Window, actions: Sender<Action>) {
    loop {
        match server.conn.wait_for_event() {
            Ok(Event::KeyPress(press)) if press.detail == keycode => {
                let state = u16::from(press.state);
                if let Some(action) = action_for(state)
                    && actions.send(action).is_err()
                {
                    break;
                }
            }
            Ok(Event::ClientMessage(message)) if message.window == stop_window => break,
            Ok(_) => {}
            Err(error) => {
                log::error!("lost the X connection: {error}");
                break;
            }
        }
    }
    if let Err(error) = server.conn.ungrab_key(0, server.root, ModMask::ANY) {
        log::warn!("Print Screen was not released: {error}");
    }
    if let Err(error) = server.conn.flush() {
        log::warn!("X requests were not flushed on exit: {error}");
    }
    log::info!("hotkey listener stopped");
}

/// Reads the pressed modifiers while ignoring Caps Lock and Num Lock.
fn action_for(state: u16) -> Option<Action> {
    let has = |mask: ModMask| state & u16::from(mask) != 0;
    resolve(has(ModMask::CONTROL), has(ModMask::SHIFT), has(ModMask::M1))
}

/// Every modifier set that resolves to an action, as raw masks.
fn modifier_combinations() -> Vec<(u16, Action)> {
    let mut combinations = Vec::new();
    for bits in 0..8u8 {
        let (ctrl, shift, alt) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        if let Some(action) = resolve(ctrl, shift, alt) {
            let mut mask = 0u16;
            if ctrl {
                mask |= u16::from(ModMask::CONTROL);
            }
            if shift {
                mask |= u16::from(ModMask::SHIFT);
            }
            if alt {
                mask |= u16::from(ModMask::M1);
            }
            combinations.push((mask, action));
        }
    }
    combinations
}

/// Caps Lock and Num Lock change the modifier state of every key press, so each
/// combination must be grabbed with all four lock states.
fn lock_variants() -> [u16; 4] {
    let caps = u16::from(ModMask::LOCK);
    let num = u16::from(ModMask::M2);
    [0, caps, num, caps | num]
}

fn print_keycode(conn: &RustConnection) -> Result<Keycode, PlatformError> {
    let setup = conn.setup();
    let first = setup.min_keycode;
    let count = setup.max_keycode - first + 1;
    let mapping = conn
        .get_keyboard_mapping(first, count)
        .map_err(fail(INSTALL))?
        .reply()
        .map_err(fail(INSTALL))?;
    let per_keycode = usize::from(mapping.keysyms_per_keycode);
    mapping
        .keysyms
        .chunks(per_keycode)
        .position(|symbols| symbols.contains(&KEYSYM_PRINT))
        .map(|index| first + index as u8)
        .ok_or_else(|| PlatformError::new(INSTALL, "the keyboard has no Print Screen key"))
}

#[cfg(test)]
mod tests {
    use klip_core::{Mode, Output};

    use super::*;

    #[test]
    fn six_modifier_sets_are_grabbed() {
        assert_eq!(modifier_combinations().len(), 6);
    }

    #[test]
    fn lock_states_do_not_change_the_resolved_action() {
        let control = u16::from(ModMask::CONTROL);
        let num_lock = u16::from(ModMask::M2);
        let caps_lock = u16::from(ModMask::LOCK);

        let action = action_for(control | num_lock | caps_lock).unwrap();

        assert_eq!(action.mode, Mode::FullScreen);
        assert_eq!(action.output, Output::Clipboard);
    }

    #[test]
    fn alt_selects_the_save_dialog_and_control_with_shift_is_ignored() {
        let alt = u16::from(ModMask::M1);
        let both = u16::from(ModMask::CONTROL) | u16::from(ModMask::SHIFT);

        assert_eq!(action_for(alt).unwrap().output, Output::SaveDialog);
        assert_eq!(action_for(both), None);
    }

    #[test]
    fn four_lock_variants_cover_caps_and_num_lock() {
        let variants = lock_variants();

        assert_eq!(variants[0], 0);
        assert_eq!(variants[3], variants[1] | variants[2]);
    }
}

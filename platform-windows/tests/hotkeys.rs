#![cfg(windows)]

use std::sync::mpsc;

use sushot_core::{HotkeyKey, Platform};
use sushot_platform_windows::WindowsPlatform;

#[test]
fn hooks_can_be_removed_and_installed_again() {
    for _ in 0..2 {
        let (actions, _receiver) = mpsc::channel();

        WindowsPlatform
            .register_hotkeys(HotkeyKey::PrintScreen, actions)
            .unwrap();
        WindowsPlatform.unregister_hotkeys();
    }
}

#[test]
fn removing_hooks_that_were_never_installed_is_harmless() {
    WindowsPlatform.unregister_hotkeys();
}

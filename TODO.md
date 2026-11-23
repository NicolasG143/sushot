# TODO

Pending, deferred or known-incomplete work. Grouped by milestone.

## M0 – Setup

- Have a lawyer review the licensing before selling. The source is under PolyForm Strict 1.0.0, which covers noncommercial use only and grants no right to distribute or modify, so two documents are still missing: commercial terms (EULA) for people who buy the binary, and a contributor agreement that both assigns or licenses contributions and permits contributors to modify the code.
- State the copyright holder (individual or company) in the README. It is not recorded anywhere yet.
- Check the trademark availability of "Sushot" with a trademark professional. The owner confirmed the name on 2026-11-23; it replaced "Klip" on 2026-11-22 because of an existing Windows capture tool and live "KLIP" registrations in software classes.
- Confirm the license policy exception for `unicode-ident` (`Unicode-3.0`). It is a compile-time dependency of procedural macros and is not linked into the binary, but `Unicode-3.0` is outside the allowed MIT/Apache-2.0/BSD/ISC list.

## M1 – Core and minimal Windows capture

- `Platform` does not declare `set_autostart` yet (M4).
- The keyboard hook swallows all six valid combinations. Region and window modes work with the clipboard output; the save dialog output (M3) is reported as not available.
- Logs go to stderr in debug builds and to the debugger output channel in every build. There is no log file.
- The `sushot` binary fails to compile for non-Windows targets until the Linux backend exists (M6).
- The idle memory goal is under 5 MB without naming the metric. Private bytes (1.8 MB) meet it and working set (10.3 MB) does not; decide which one counts. See `METRICS.md`.

## M2 – Selection overlay

- Measure the 60 FPS drag requirement. Redraws repaint and copy only the dirty rectangle, but no frame time was measured.
- The overlay appeared after about 82 ms in the end-to-end test, including the 20 ms polling granularity of the test. The 100 ms budget has little margin; profile `grab_virtual_desktop` and window creation on larger desktops.
- Window mode skips click-through layered windows, cloaked windows and minimized windows, and otherwise picks the topmost visible top-level window, which can be the desktop shell window when no application is under the cursor.
- The end-to-end overlay tests inject real mouse and keyboard input, so a physical mouse movement during the run makes them fail. Run them without touching the mouse.

## M3 – File output

- The save dialog opens over an invisible owner window (`ForegroundOwner`) that takes the foreground on the dialog thread, falling back to `AttachThreadInput` when Windows refuses. The end-to-end test passed 7 of 8 runs on 2026-11-22; in the failed run the dialog was in front but the typed path never produced a file, probably because the first, cold dialog was not ready for input yet. Run `save_to_file` on an idle desktop: it checks that the dialog has the focus before typing.
- Only `Ctrl + Alt + Print Screen` was driven end to end through the native dialog. Region and window with `Alt` share the same code path and are covered by unit tests.
- The save dialog and error messages are in Spanish, like the tray menu; there is no localization layer.
- Saving encodes from a cropped copy of the frozen frame, so a full-desktop save holds the frame and the crop in memory while the dialog is open.

## M4 – Tray, configuration and autostart

- The toast reuses the tray icon through `Shell_NotifyIconW` with the icon id 1, which is the first id `tray-icon` hands out. A newer `tray-icon` that numbers icons differently would make the toast fail with a warning in the log.
- The first-run notice appears when `config.toml` does not exist yet, on every Windows version, and its text mentions Windows 11. It was launched but not seen on screen.
- The MSIX build must use the `StartupTask` instead of the `Run` registry key; `set_autostart` only handles the `Run` key.
- `show_toast` and `autostart` are read from `config.toml` at use time or at startup; there is no file watcher, so edits take effect on the next copy or restart.

## M5 – Packaging and release

- Reserve the name "Sushot" in Partner Center, fill the placeholders in `packaging/msix/AppxManifest.xml` (`Identity` name and publisher, publisher display name) with the values Partner Center assigns, and replace the generated logos if a final brand exists.
- Install the signed MSIX and confirm the global keyboard hook works in the packaged, full-trust environment, and that the `StartupTask` toggle works (`set_startup_task` was only compiled, never run packaged). Installing needs a trusted signing certificate, which was not available.
- Sign the executable and the package (code signing certificate or Azure Trusted Signing). `packaging/build-msix.ps1` signs when given a `.pfx`.
- Run the manual checks on Windows 10. On 2026-11-23 the owner checked by hand, on Windows 11: pasting into Paint, a browser, Word and Slack, Snipping Tool suppression, the tray menu and autostart, and two monitors with mixed DPI.
- Verify pasting into Discord and image editors.
- Publish to the Microsoft Store, enable the direct download and build the landing page with the payment gateway. These need accounts and decisions that belong to the owner: Partner Center, a payment provider, the buyer terms listed under M0 and the domain.
- The stability test covers capture and clipboard in-process. A run of 1000 captures through the tray, overlay and hotkey path was not done because it would need 1000 injected key presses on the user's desktop.
- Native image editors may prefer other clipboard formats than `PNG` and `CF_DIBV5`; this was not checked.

## M6 – Linux X11 and M7 – Linux Wayland

- On 2026-11-23 the workspace was built and tested on Ubuntu 26.04 under WSL2: `cargo test --workspace`, clippy and rustfmt pass, including the Linux-only unit tests. On an Xvfb server, `Ctrl + Print Screen` grabbed through X11 captured the full 1600x900 screen and another process (`xclip`) read the PNG from the clipboard; `sushot --full` and the Unix socket worked too. Run `cargo test -p sushot-platform-linux --test clipboard_x11 -- --ignored` with an X server in `DISPLAY` and `xclip` installed.
- Still unverified on Linux: region and window modes with the overlay, RandR with several monitors, EWMH window picking, the save dialog, autostart, the `ksni` tray, and everything on Wayland (portal screenshot and portal shortcuts). WSLg cannot cover them: it has no `xdg-desktop-portal` or StatusNotifierItem host, and its rootless XWayland answers `GetImage` on the root window with `BadMatch` (plain `xwd -root` fails the same way). They need a real X11 desktop and a GNOME or KDE Wayland session.
- In WSLg, `/tmp/.X11-unix` is read-only, so Xvfb only starts with `-listen tcp -nolisten unix` and `DISPLAY=127.0.0.1:<n>`.
- Print Screen is grabbed for the six combinations with the four Caps Lock and Num Lock states. A grab fails if another client already owns the key (a desktop screenshot tool), and the error is reported at startup.
- Under Wayland, `XGrabKey` does not receive keys outside XWayland windows, so sushot uses the `GlobalShortcuts` portal. If registering hotkeys fails for any reason, sushot logs a warning and keeps running so the CLI with a system shortcut still works; on Windows this means a failed hook leaves a tray icon with no hotkeys.
- `capture_with_system_ui` with `Mode::Window` and `Mode::Region` both open the portal's interactive picker; the portal decides what the user can choose. The portal screenshot file is deleted after reading it.
- The Wayland portal paths ignore `unregister_hotkeys`; the shortcut thread ends with the process.
- The X11 clipboard relies on `arboard` keeping the selection owner alive; sushot stays resident, but pasting after sushot exits yields nothing, as expected with X11 selections.
- `place_overlay` on X11 raises and sizes the window through `ConfigureWindow`; window managers may ignore a position request for a normal window. An override-redirect overlay may be needed.
- Linux notifications for `show_toast` only write a log line; there is no desktop notification yet.
- `sushot --region` and its siblings fail with a clear message when no instance is running instead of running a one-shot capture.
- Build the AppImage with `packaging/linux/build-appimage.sh` on Ubuntu and test the six combinations (M6), then GNOME and KDE Wayland (M7). Optional `.deb` packaging is not written.
- Publish the Wayland support table on the landing page once it exists.

## v1.1

- Configurable shortcuts only change the base key (`hotkey`); modifiers per action are fixed. The key is read at startup, and a changed value needs a restart. Only the Windows hook and unit tests exercised it; the X11 grab and portal trigger for other keys were not run.
- Arrow-key nudging only works while the mouse button is held during a drag. Adjusting a region after releasing the button would require keeping the selection open until a confirm key or click, which changes the one-gesture flow.
- Windows Arm64: `cargo build --release --target aarch64-pc-windows-msvc` compiles every crate and fails at the link step because the Visual Studio ARM64 build tools are not installed. The `aarch64-pc-windows-msvc` Rust target was added to this machine with `rustup`.
- The round loupe replaces the pointer in region mode only after the first mouse move inside the overlay; until then the crosshair is visible. It covers the selection border around the pointer while dragging. It is not shown in window mode.
- Left Alt + Left Shift + Print Screen also toggles Windows High Contrast, which the system handles before the low-level hook, so sushot cannot block it. The README and the first-run notice explain the right-hand modifiers and the setting; using the right-hand keys was not tested.
- The keyboard hook sometimes never receives the Print Screen release when the overlay opens while the key is still held; the cause is unknown. A press more than one second after the previous one counts as a new keystroke, so a second press within that second after such a loss is still swallowed.

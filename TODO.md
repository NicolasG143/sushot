# TODO

Pending, deferred or known-incomplete work. Milestones are tracked in `ROADMAP.md`.

## M0 – Setup

- Have a lawyer review the licensing before selling. The source is under PolyForm Strict 1.0.0, which covers noncommercial use only and grants no right to distribute or modify, so two documents are still missing: commercial terms (EULA) for people who buy the binary, and a contributor agreement that both assigns or licenses contributions and permits contributors to modify the code.
- State the copyright holder (individual or company) in the README. It is not recorded anywhere yet.
- Settle the product name. A preliminary web search (2026-11-22) found an existing Windows clipboard and screen capture tool distributed as "Klip" (winget id `PoBruno.Klip`) and several live "KLIP" trademark registrations in software classes. Confirm availability in Microsoft Partner Center and with a trademark professional, or rename.
- Confirm the license policy exception for `unicode-ident` (`Unicode-3.0`). It is a compile-time dependency of procedural macros and is not linked into the binary, but `Unicode-3.0` is outside the MIT/Apache-2.0/BSD/ISC list in `SPECS.md`.

## M1 – Core and minimal Windows capture

- Manually confirm the rest of the M1 exit criterion: paste a `Ctrl + Print Screen` capture into Paint and into a browser, and check on Windows 11 that Snipping Tool does not open while "Use the Print screen key to open screen capture" is enabled.
- Verify coordinates on two monitors with different DPI scaling and a negative virtual desktop origin. Only a single 1920x1080 monitor was available.
- `SPECS.md` sections 7.1 and 10 still list `arboard` for the Windows clipboard and `xcap` as a capture option. The implementation uses Win32 directly for both; update the spec once the decision is confirmed.
- `Platform` does not declare `set_autostart` yet (M4).
- The keyboard hook swallows all six valid combinations. Region and window modes work with the clipboard output; the save dialog output (M3) is reported as not available.
- Logs go to stderr in debug builds and to the debugger output channel in every build. There is no log file.
- The `klip` binary fails to compile for non-Windows targets until the Linux backend exists (M6).
- The idle memory goal in `SPECS.md` section 12 is under 5 MB without naming the metric. Private bytes (1.8 MB) meet it and working set (10.3 MB) does not; decide which one counts. See `METRICS.md`.

## M2 – Selection overlay

- Verify the overlay on two monitors with different DPI scaling and a negative virtual desktop origin. Only a single 1920x1080 monitor was available. The overlay is placed with `SetWindowPos` in physical pixels and answers `ScaleFactorChanged` by requesting the desktop size again, but this path is untested.
- Measure the 60 FPS drag requirement. Redraws repaint and copy only the dirty rectangle, but no frame time was measured.
- The overlay appeared after about 82 ms in the end-to-end test, including the 20 ms polling granularity of the test. The 100 ms budget has little margin; profile `grab_virtual_desktop` and window creation on larger desktops.
- Window mode skips click-through layered windows, cloaked windows and minimized windows, and otherwise picks the topmost visible top-level window, which can be the desktop shell window when no application is under the cursor.
- The end-to-end overlay tests inject real mouse and keyboard input, so a physical mouse movement during the run makes them fail. Run them without touching the mouse.

## M3 – File output

- The save dialog does not always open in the foreground. `grant_foreground` injects an unused key (F24) before the dialog so Windows lets the process take focus, and the end-to-end test failed intermittently because another window (File Explorer, Photos, Discord) stayed in front. Investigate an owner window with `AttachThreadInput` or `SetForegroundWindow` on the dialog itself. Run `save_to_file` only on an idle desktop: it types into whatever window has focus when the dialog fails to come forward.
- Only `Ctrl + Alt + Print Screen` was driven end to end through the native dialog. Region and window with `Alt` share the same code path and are covered by unit tests.
- The save dialog and error messages are in Spanish, like the tray text in `SPECS.md`; there is no localization layer.
- Saving encodes from a cropped copy of the frozen frame, so a full-desktop save holds the frame and the crop in memory while the dialog is open.

## M4 – Tray, configuration and autostart

- Manually verify the tray: menu content, the autostart checkbox, "Salir" releasing the hook and the icon, the icon flash after a copy, the optional toast with `show_toast = true`, and that autostart launches klip after signing in again. None of this could be driven without a person at the keyboard.
- The toast reuses the tray icon through `Shell_NotifyIconW` with the icon id 1, which is the first id `tray-icon` hands out. A newer `tray-icon` that numbers icons differently would make the toast fail with a warning in the log.
- The first-run notice appears when `config.toml` does not exist yet, on every Windows version, and its text mentions Windows 11. It was launched but not seen on screen.
- The MSIX build must use the `StartupTask` instead of the `Run` registry key (`SPECS.md` section 9.1); `set_autostart` only handles the `Run` key.
- `show_toast` and `autostart` are read from `config.toml` at use time or at startup; there is no file watcher, so edits take effect on the next copy or restart.

## M5 – Packaging and release

- Fill the placeholders in `packaging/msix/AppxManifest.xml` (`Identity` name and publisher, publisher display name) with the values Partner Center assigns, and replace the generated logos if a final brand exists. The name conflict recorded in M0 must be settled first.
- Install the signed MSIX and confirm the global keyboard hook works in the packaged, full-trust environment, and that the `StartupTask` toggle works (`set_startup_task` was only compiled, never run packaged). Installing needs a trusted signing certificate, which was not available.
- Sign the executable and the package (code signing certificate or Azure Trusted Signing). `packaging/build-msix.ps1` signs when given a `.pfx`.
- Complete the manual matrix: Windows 10 and 11, one and two monitors, DPI 100 %, 125 %, 150 % and mixed. Only Windows 11 at 100 % with one monitor was available.
- Verify pasting into Paint, Word, Slack, Discord, a browser and image editors.
- Publish to the Microsoft Store, enable the direct download and build the landing page with the payment gateway (`SPECS.md` section 13.3). These need accounts and decisions that belong to the owner: Partner Center, a payment provider, the buyer terms listed under M0 and the domain.
- The stability test covers capture and clipboard in-process. A run of 1000 captures through the tray, overlay and hotkey path was not done because it would need 1000 injected key presses on the user's desktop.
- Native image editors may prefer other clipboard formats than `PNG` and `CF_DIBV5`; this was not checked.

## M6 – Linux X11 and M7 – Linux Wayland

- None of the Linux code has been run. It type-checks and passes clippy for `x86_64-unknown-linux-gnu` from Windows, and the portable pieces (XDG paths, `.desktop` entry, lockfile, PNG decoding, CLI parsing, session detection, request protocol) have tests that pass on Windows. The X11 capture, key grabs, RandR, EWMH windows, `arboard` clipboard, portal screenshot, portal shortcuts, Unix socket and `ksni` tray need a real session to confirm.
- To check Linux compilation from Windows, `x11` build scripts need `pkg-config`; a stub `pkg-config.exe` that prints `-lX11` plus `PKG_CONFIG_ALLOW_CROSS=1` was enough. A real Linux build needs the packages listed in `README.md`.
- Unit tests that exist only for Linux (`x11.rs`, `hotkeys.rs`, `portal.rs`, `ipc.rs`) compile but were never executed.
- Print Screen is grabbed for the six combinations with the four Caps Lock and Num Lock states. A grab fails if another client already owns the key (a desktop screenshot tool), and the error is reported at startup.
- Under Wayland, `XGrabKey` does not receive keys outside XWayland windows, so klip uses the `GlobalShortcuts` portal. If registering hotkeys fails for any reason, klip logs a warning and keeps running so the CLI with a system shortcut still works; on Windows this means a failed hook leaves a tray icon with no hotkeys.
- `capture_with_system_ui` with `Mode::Window` and `Mode::Region` both open the portal's interactive picker; the portal decides what the user can choose. The portal screenshot file is deleted after reading it.
- The Wayland portal paths ignore `unregister_hotkeys`; the shortcut thread ends with the process.
- The X11 clipboard relies on `arboard` keeping the selection owner alive; klip stays resident, but pasting after klip exits yields nothing, which `SPECS.md` section 7.1 already states.
- `place_overlay` on X11 raises and sizes the window through `ConfigureWindow`; window managers may ignore a position request for a normal window. An override-redirect overlay may be needed.
- Linux notifications for `show_toast` only write a log line; there is no desktop notification yet.
- `klip --region` and its siblings fail with a clear message when no instance is running instead of running a one-shot capture.
- Build the AppImage with `packaging/linux/build-appimage.sh` on Ubuntu and test the six combinations (`ROADMAP.md`, M6), then GNOME and KDE Wayland (M7). Optional `.deb` packaging is not written.
- Publish the Wayland support table on the landing page once it exists.

## v1.1

- Configurable shortcuts only change the base key (`hotkey`); modifiers per action are fixed. The key is read at startup, and a changed value needs a restart. Only the Windows hook and unit tests exercised it; the X11 grab and portal trigger for other keys were not run.
- Arrow-key nudging only works while the mouse button is held during a drag. Adjusting a region after releasing the button would require keeping the selection open until a confirm key or click, which changes the one-gesture flow.
- Windows Arm64: `cargo build --release --target aarch64-pc-windows-msvc` compiles every crate and fails at the link step because the Visual Studio ARM64 build tools are not installed. The `aarch64-pc-windows-msvc` Rust target was added to this machine with `rustup`.
- The round loupe replaces the pointer in region mode only after the first mouse move inside the overlay; until then the crosshair is visible. It covers the selection border around the pointer while dragging. It is not shown in window mode.

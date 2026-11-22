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

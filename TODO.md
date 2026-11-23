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
- Idle memory of the release build was 6.5 MB of working set and 1.1 MB of private bytes in a single informal reading. The goal in `SPECS.md` section 12 is under 5 MB without naming the metric; define it and measure properly in M5.

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

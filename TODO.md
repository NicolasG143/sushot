# TODO

Pending, deferred or known-incomplete work. Milestones are tracked in `ROADMAP.md`.

## M0 – Setup

- Choose the source license (PolyForm Noncommercial 1.0.0 or BSL 1.1) and add `LICENSE` at the repository root. The workspace crates stay `publish = false` and carry no `license` field until this is decided.
- Settle the product name. A preliminary web search (2026-11-22) found an existing Windows clipboard and screen capture tool distributed as "Klip" (winget id `PoBruno.Klip`) and several live "KLIP" trademark registrations in software classes. Confirm availability in Microsoft Partner Center and with a trademark professional, or rename.
- Confirm the license policy exception for `unicode-ident` (`Unicode-3.0`). It is a compile-time dependency of procedural macros and is not linked into the binary, but `Unicode-3.0` is outside the MIT/Apache-2.0/BSD/ISC list in `SPECS.md`.

## M1 – Core and minimal Windows capture

- Manually confirm the rest of the M1 exit criterion: paste a `Ctrl + Print Screen` capture into Paint and into a browser, and check on Windows 11 that Snipping Tool does not open while "Use the Print screen key to open screen capture" is enabled.
- Verify coordinates on two monitors with different DPI scaling and a negative virtual desktop origin. Only a single 1920x1080 monitor was available.
- `SPECS.md` sections 7.1 and 10 still list `arboard` for the Windows clipboard and `xcap` as a capture option. The implementation uses Win32 directly for both; update the spec once the decision is confirmed.
- `Platform` declares only the operations M1 uses. `window_rect_at` (M2) and `set_autostart` (M4) are not part of the trait yet.
- The keyboard hook swallows all six valid combinations, but only `Ctrl + Print Screen` produces a capture. Region and window modes (M2) and the save dialog output (M3) are logged as not available.
- klip can only be stopped by ending the process until the tray menu exists (M4).
- Logs go to stderr in debug builds and to the debugger output channel in every build. There is no log file.
- The `klip` binary fails to compile for non-Windows targets until the Linux backend exists (M6).
- Idle memory of the release build was 6.5 MB of working set and 1.1 MB of private bytes in a single informal reading. The goal in `SPECS.md` section 12 is under 5 MB without naming the metric; define it and measure properly in M5.

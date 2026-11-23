# M2 to v1.1: overlay, file output, tray and Windows release work

Objective: complete the bullets of ROADMAP.md from M2 onward. Branch: `feat/m0-m1-core-windows`.
Delivery: one commit per verified checkpoint; push and PR slicing stay with the user. Test runner: `cargo test --workspace`.

## Specs

- **S1** — "Read ./SPECS.md | README.md | CODESTYLE.md | ROADMAP.md, complete bullet points on ROADMAP, commit on every significant checkpoint. M0 and M1 should already be finished, check that and then go ahead. Conventional commits "prefix: description" using CLI. Do not abuse comments or tokens. Format before commit."
- **S2** — `CODESTYLE.md` applies to every source file; pending work lives in `TODO.md`.
- **S3** — Each milestone's "Listo cuando" line in `ROADMAP.md` is its exit criterion.

## Tasks

| ID | Task | Specs | Route | Status | Commit |
|---|---|---|---|---|---|
| T0 | Verify M0 and M1 (fmt, tests, release build, deny) | S1 | inline | done | none needed |
| T1 | M2 canvas, geometry and selection logic in core | S2, S3 | inline | done | f5ce31f |
| T2 | M2 overlay, window hit-test, e2e tests | S2, S3 | inline | done, 2-monitor check pending | 7e5e0d8 |
| T3 | M3 config, filename, streaming PNG | S2, S3 | inline | done | 8247d31 |
| T4 | M3 save dialog flow | S2, S3 | inline | done, dialog focus flaky | ca37cfd |
| T5 | M4 tray, autostart, single instance, feedback | S2, S3 | inline | done, manual tray check pending | d9f352d |
| T6 | M5 metrics, stability, MSIX package, faster clipboard PNG | S2, S3 | inline | partial: signing, Store, landing, manual matrix pending | 145089a |
| T7 | v1.1 arrow-key nudging | S2 | inline | done | closing commit |
| T8 | M6 Linux X11, M7 Linux Wayland | S2, S3 | inline | written; compile-checked and pure-logic tests only, never run on Linux | closing commit |

## Log

- **L1** (2026-11-22) User request, verbatim: "Read ./SPECS.md | README.md | CODESTYLE.md | ROADMAP.md, complete bullet points on ROADMAP, commit on every significant checkpoint. M0 and M1 should already be finished, check that and then go ahead. Conventional commits "prefix: description" using CLI. Do not abuse comments or tokens. Format before commit."
- **L2** Route: every task inline; units are sequential and share files, so no writer trigger holds. Receipt-driven development was not enabled by the user for this work.
- **L3** Decisions recorded in `ROADMAP.md`: window hit-test through `EnumWindows` z-order, manual config parser, no action on tray left click, full screen captures the monitor under the cursor.
- **L4** Real-input tests (`overlay_to_clipboard`, `save_to_file`) are flaky on a desktop in use: a moving mouse or a window that keeps focus breaks them. Once, before a focus assertion existed, `save_to_file` typed a file path and Enter into the focused browser.
- **L5** Measured on 2026-11-22, one 1920x1080 monitor: see `METRICS.md`. PNG for the clipboard moved to the fastest compression to meet the 150 ms goal (365 ms to 22 ms).
- **L6** Not verifiable here: two monitors with mixed DPI, packaged MSIX behavior, signing, Store, landing, Linux.
- **L7** After the stop hook asked for the Linux milestones, the backend was written against `x11rb`, `arboard`, `ashpd` and `ksni` and verified with `cargo clippy --target x86_64-unknown-linux-gnu` (stub `pkg-config` for the X11 build scripts). Nothing ran on Linux; see `TODO.md`.

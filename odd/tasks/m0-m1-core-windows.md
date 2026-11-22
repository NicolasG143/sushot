# M0 + M1: workspace setup and minimal Windows capture

Objective: complete ROADMAP.md phases M0 and M1 only. Branch: `feat/m0-m1-core-windows`.
Delivery: one commit per checkpoint; push and PR slicing stay with the user. Test runner: `cargo test --workspace`.

## Specs

- **S1** — "Lee CODESTYLE.md": every source file follows `CODESTYLE.md` (no meta, reasoning or WHAT comments, no stubs, no TODO/FIXME in code, meaningful logging, pending work recorded in `TODO.md` in English).
- **S2** — "completá SOLO las fases 0 y 1 del ROADMAP": scope is the bullets under `## M0 – Preparación` and `## M1 – Core + Windows mínimo` in `ROADMAP.md`. Nothing from M2 onwards.
- **S3** — M0 exit criterion, verbatim: "**Listo cuando:** `cargo build --release` compila el workspace vacío y `cargo deny check` pasa."
- **S4** — M1 exit criterion, verbatim: "**Listo cuando:** `Ctrl + Impr Pant` copia el monitor bajo el cursor, la imagen se pega en Paint y en un navegador, y Recortes no se activa."
- **S5** — "Commiteá en cada checkpoint": Conventional Commits, no AI attribution, one commit per verified work unit.

## Tasks

| ID | Task | Specs | Route | Status | Commit |
|---|---|---|---|---|---|
| T0 | Baseline docs on the feature branch | S5 | inline | done | 22cf563 |
| T1 | Cargo workspace, four crates, release profile | S2, S3 | inline | done | d55b9be |
| T2 | `cargo deny` policy and `cargo about` generation | S2, S3 | inline | done | d71d631 |
| T3 | License choice and name availability | S2 | inline | needs user decision | — |
| T4 | `klip-core`: `resolve()`, `Rect`, `Frame` crop, `Platform` trait, PNG encode, tests | S1, S2 | inline | done | 4f8c89b, 7a514cc |
| T5 | `klip-platform-windows`: GDI capture and monitor under cursor | S1, S2 | inline | done | 78ccfb3 |
| T6 | `klip-platform-windows`: `WH_KEYBOARD_LL` hook | S1, S2 | inline | done | 78ccfb3 |
| T7 | `klip-platform-windows`: clipboard writer (PNG + CF_DIBV5) | S1, S2 | inline | done | 1259443 |
| T8 | `klip` app: manifest, static CRT, orchestration, end-to-end check | S1, S2, S4 | inline | done, manual checks pending | closing commit |

## Log

- **L1** (2026-11-22) User request, verbatim: "Lee CODESTYLE.md y completá SOLO las fases 0 y 1 del ROADMAP. Commiteá en cada checkpoint."
- **L2** Route: every task inline; no writer trigger holds (units are sequential and depend on each other). Receipt-driven development is off (global), so no native review runs.
- **L3** The package for `core/` is named `klip-core` because a package named `core` collides with the Rust built-in crate.
- **L4** RED then GREEN observed for: `resolve()` and `Frame::crop` (core), `SnapshotKey` and GDI capture (platform-windows), `perform` (app), PNG encode and DIBV5 layout.
- **L5** The end-to-end test failed first with arboard: `SetClipboardData` returned error 1418 while the test polled the clipboard. A two-process probe showed that a clipboard opened with a NULL owner is taken over by another `OpenClipboard(NULL)`, and that an owner window makes the second open fail with access denied. `cargo deny` also rejected arboard's Windows dependencies `clipboard-win` and `error-code` (BSL-1.0). Decision: Win32 clipboard writer with a message-only owner window, PNG and DIBV5 built before opening, five open attempts.
- **L6** Capture uses GDI directly instead of `xcap`; `xcap` was not measured.
- **L7** `Platform` declares four operations (grab, monitor, copy image, hotkeys); `window_rect_at` and `set_autostart` belong to M2 and M4.
- **L8** Verification on 2026-11-22, single 1920x1080 monitor: `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` 21 passed, 1 ignored; ignored end-to-end test passed when run explicitly; `cargo deny check` ok; release binary 367104 bytes, manifest embedded, no dynamic CRT import.
- **L9** Not verified: paste into Paint and a browser, Snipping Tool suppression on Windows 11, multi-monitor and mixed DPI. Recorded in `TODO.md`.
- **L10** Commit 7a514cc carries a `Cargo.lock` that already listed app dependencies not yet declared at that commit; later commits are consistent.
- **L11** Open for the user: source license (PolyForm Noncommercial 1.0.0 or BSL 1.1), product name conflict found in a preliminary search, confirmation of the `unicode-ident` license exception.

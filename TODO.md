# TODO

Pending, deferred or known-incomplete work. Milestones are tracked in `ROADMAP.md`.

## M0 – Setup

- Choose the source license (PolyForm Noncommercial 1.0.0 or BSL 1.1) and add `LICENSE` at the repository root. The workspace crates stay `publish = false` and carry no `license` field until this is decided.
- Settle the product name. A preliminary web search (2026-11-22) found an existing Windows clipboard and screen capture tool distributed as "Klip" (winget id `PoBruno.Klip`) and several live "KLIP" trademark registrations in software classes. Confirm availability in Microsoft Partner Center and with a trademark professional, or rename.
- Confirm the license policy exception for `unicode-ident` (`Unicode-3.0`). It is a compile-time dependency of procedural macros and is not linked into the binary, but `Unicode-3.0` is outside the MIT/Apache-2.0/BSD/ISC list in `SPECS.md`.

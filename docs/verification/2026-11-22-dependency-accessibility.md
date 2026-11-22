# Dependency and accessibility follow-up — 2026-11-22

This receipt follows the [56-tool pass](2026-11-22-56-tool-pass.md). The user approved phases 1 and 2 of
[ADR 0005](../adr/0005-dependency-security-remediation.md), including three pinned upstream source
backports, Age 0.12.1 and Jiff 1.0.2. Exact eframe 0.37.0, Rust 1.95, all 56 stable tool IDs, saved data,
offline defaults and the unpublished draft are retained. Clean-machine acceptance remains intentionally pending.

The Age 0.12.1 candidate subsequently failed its security gate: it adds the vulnerable `crypto-bigint
0.5.5` family through mandatory HPKE/P-256 paths. The application retains exact Age 0.11.5 while a
[separate localization-only remedy](../adr/0006-age-upgrade-security-gate.md) is reviewed. Its successful
compile/interoperability results below are historical candidate evidence, not acceptance of that upgrade.

## Compatibility evidence

- Before changing production time code, the integrator captured the original public API corpus:
  `crates/oxbox-core/tests/fixtures/chrono-time-compat.json`, 675,194 bytes, SHA-256
  `c83f87e6b476346672f2cf0dfc9d8185c599b8f954e6aabbed82cbecb8797b85`.
  It contains 597 ordered zone names, 1,863 clock rows, 40 timestamp-range rows, 44 local-time cases,
  48 recurrence cases, eight restore cases, saved records and 17 rate dates. All five baseline comparison
  tests passed against the original Chrono implementation. This frozen fixture must not be regenerated
  from the new implementation merely to make a comparison pass.
- A separate original-parser probe reproduced signed extended years, numeric padding and Unicode
  whitespace acceptance. A new currency regression then failed for `+10000-02-29` with direct Jiff
  parsing. The checked compatibility adapter preserves the original accepted calendar range and raw
  stored date strings, using Jiff to validate the repeating Gregorian calendar. That regression passes.
- Two future-time regressions failed against the original implementation: after Chrono-TZ's finite
  transition table, summer New York clocks and a year-2200 alarm used EST indefinitely. The maintained
  time implementation must apply the zone's recurring future rules. This is an explicit behavior correction;
  it must be distinguished from exact preservation of the frozen historical/current corpus.
- The integrator ran both directions of a generated Age interoperability harness: actual Age 0.11.5
  factor-10 output decrypted in the current core, and real current-core factor-18 output decrypted in
  Age 0.11.5 with ceiling 20. Both recovered the same generated 196,731 bytes, SHA-256
  `669f2290ddeaf7c800b997b38c82798cb253edaf4027a3757a7eb86fe0ab8bd5`.
  Wrong passwords, authenticated corruption, truncation, real-progress cancellation, pre-cancel,
  excessive-factor rejection, exclusive outputs and unchanged originals passed. These compatibility
  checks were expected to pass; no failing application behavior is claimed for them.
- A checked-in low-factor historical fixture, producer and reproduction guide keep legacy decrypt
  compatibility independently runnable without adding historical Age as an application dependency.
  Generation with actual Age 0.11.5 and its old-reader self-check passed; ciphertext SHA-256
  `122a8bf7081fffe5ddf353f3a1f3f8e60860b63ea15dd0f668a6eaea7adc7d5e`.
- The isolated [configuration harness](../../tests/dependency-compat/README.md) passed five generated
  tests for valid, invalid and missing Cargo/localization configuration, Unicode values, defaults and
  parent fallback. Production Age localization macros also compiled. This preserves the distinction
  between a missing optional configuration and malformed configuration.

## Accessibility fixes and scope

Reproduced behavioral regressions cover popup Escape dismissal; actual expanded/collapsed metadata;
screen-reader Expand/Collapse action dispatch for dropdowns and disclosure headers; disabled text and
numeric fields refusing accessibility value edits; keyboard-editable numeric screenshot-area bounds;
distinct decoded-barcode Copy names; and stable contextual image-row identity after another row is removed.
Tests failed before their corresponding fixes. No test captures a screen, moves the pointer, reads the
clipboard, deletes user files or changes an OS setting.

The integrated native accessibility suite passed **146 tests** before the time migration. All 56 default
routes have named interactive controls and actual combo expansion state. These headless AccessKit checks
prove those properties; they do not prove spoken announcements, visible focus or complete keyboard traversal.
The user offered to assist with Narrator speech. Actual final-binary speech evidence remains pending.

The final local held-Age Windows workspace run passed **294 core tests, five frozen comparisons,
148 native tests and 12 retained-host tests**, zero failed/ignored. It includes three additional checked
time-range tests and a reproduced/fixed legacy alarm grammar regression for Unicode numeric whitespace
and signed padded years. Strict all-target workspace Clippy and formatting passed on Rust 1.95.
The frozen compatibility fixture still has its original hash.

## Source identity and supporting checks

The local integrity verifier passed for all **572 upstream package files** and five provenance/patch
artifacts. Notice generation now binds patched package identity to the verified directory, tree hash,
base archive, immutable upstream revision and exact patch; it refuses mismatched or missing provenance.
The native build wrapper verifies source integrity before distribution work.

The combined source-integrity, notices and installer/build regressions passed **27/27**; legacy UI
tests passed **39/39** across 11 files; Svelte check reported zero errors/warnings; legacy production
build passed; website generation preserved 56 tools/eight categories and its four regressions passed.

## Final acceptance still required

The fresh audit for the held Age 0.11.5 resolution reports **zero vulnerability entries and six
maintenance warnings**, using advisory database `d1fe6ac3eba823a3515e849045f36dc34543885d`, without
ignores. Chrono remains locked for RXing's preserved optional client/WASM support; Chrono-TZ,
basic-toml and the introduced crypto-bigint chain are removed. GLib is a path package omitted by the
scanner, so the reviewed source advisory remains visible separately. The native Windows graph check
passed with **345 package names** and no Tauri/WebView host. Regenerated Windows notices verify
**358 packages, zero missing texts and three human attribution reviews**.

Optimized Linux GLib execution, cross-platform CI, exact production executable, installer/static and
runtime observations must still be recorded before calling this candidate verified. Prior executable
hashes and measurements do not certify changed dependencies. ADR 0006's fourth source-patch proposal
is pending user approval; it has not been applied.

GTK macro and AccessKit/Zbus maintenance migrations remain outside the approved implementation scope.
Never suppress advisories or disable AccessKit to present a clean audit. Patched GLib retains its original
version and needs source-level accounting even if a scanner omits local path packages.

Clean-machine install/login/upgrade/uninstall, full per-tool native effects, another screen reader,
IME/DPI/high contrast and other-OS runtime accessibility, DNS/broker/short-flow coverage, independent
security/attribution reviews and signing remain separate gates. Keep the release a draft/prerelease;
do not create a version tag or publish it.

# Oxbox task list

Details, acceptance criteria and dependencies for each task are in [plan.md](plan.md).
Work top to bottom; one task per branch/PR. Tick a box only when its acceptance criteria and AGENTS.md
"Definition of done" are met.

## Implementation status — 2026-11-21

The detailed v0.1 and user-promoted Phase 5 source is integrated on `codex/complete-v0-1`. All 48 catalogue entries load
their views; core tests, typed command groups, background lifecycle, and documentation are present.
The original acceptance boxes below remain open where Windows manual checks, independent review, runtime privacy
or public release signoff are outstanding. [Public CI passed at 5117e6d](https://github.com/ninad-k/oxbox/actions/runs/44909470452).
These boxes are not a list of missing source implementations.

| Tasks | Source status | Acceptance still required |
|---|---|---|
| 1.1–1.6 | Implemented; automated coverage added | Full native interaction, restart/login/shortcut and hidden-notification smoke checks |
| 2.1–2.5 | Implemented; hostile-archive/path/cancellation tests added | Large-file UI/drop/Trash recovery checks; human security checkpoint |
| 3.1–3.6 | Implemented; lifecycle and fake-clock tests added | Real Windows power/break effects and Quit verification on a consenting test machine |
| 4.1–4.6 | Implemented; media/privacy tests added | Multi-monitor/scaling/capture/clipboard and OS-specific runtime checks |
| 4.7 | Windows MSI 4.07 MiB / NSIS 2.83 MiB and checksums prepared; native builds pass on three OSes | Clean-machine/tool runtime acceptance, cold startup, WebView2 privacy gap, publication |
| Phase 5 system/apps | Twelve tools implemented | Real Windows window/display/eject/uninstall effects in a consenting test account |
| Phase 5 media/vision | Ten tools implemented | Device/provider/session-specific recording, OCR and color sampling checks |
| Phase 5 presentation/time | Five tools implemented | External-display presentation and native alarm/OS-control checks |
| Phase 5 currency | Offline import and opt-in ECB refresh implemented; issue #1 posted with approval | UI opt-in/cache/error verification and default-offline runtime observation |

See [validation evidence](../docs/VALIDATION.md), [test matrix](../docs/TESTING.md), and
[release checklist](../docs/RELEASE.md) before closing a gate.

## Phase 0: Framework
- [x] 0.1 Workspace, Tauri 2 + SvelteKit, icons
- [x] 0.2 Tray icon and close-to-tray
- [x] 0.3 Launcher UI (sidebar, search, favourites, tool view, light/dark)
- [x] 0.4 Reference tools: Transform Text, Verify Checksum
- [x] 0.5 CI and release workflows
- [x] 0.6 Docs and licenses

## Phase 1: App essentials + simple tools
- [ ] 1.1 Settings store, theme, launch at login
- [ ] 1.2 Global shortcut and single instance
- [ ] 1.3 Unit Converter
- [ ] 1.4 Timer & Stopwatch
- [ ] 1.5 World Time
- [ ] 1.6 QR & Barcode
- [ ] **Checkpoint 1:** CI green, manual pass on Windows, review

## Phase 2: File tools
- [ ] 2.1 Jobs infrastructure (progress + cancel)
- [ ] 2.2 File drop component + safe trash helper
- [ ] 2.3 Find Duplicates
- [ ] 2.4 Archive & Unarchive
- [ ] 2.5 Clean Drive
- [ ] **Checkpoint 2:** security review of file tools

## Phase 3: System & power
- [ ] 3.1 System Monitor
- [ ] 3.2 Network Usage
- [ ] 3.3 Do Not Sleep
- [ ] 3.4 Lock Screen
- [ ] 3.5 Sleep Timer
- [ ] 3.6 Break Time
- [ ] **Checkpoint 3:** manual run of power tools; nothing runs after Quit

## Phase 4: Capture, media, privacy
- [ ] 4.1 Screenshot (screen, window)
- [ ] 4.2 Screenshot (area selection overlay)
- [ ] 4.3 Clipboard History
- [ ] 4.4 Resize Images
- [ ] 4.5 Hide Desktop
- [ ] 4.6 Encrypt Files
- [ ] 4.7 Release v0.1.0
- [ ] **Checkpoint 4:** v0.1.0 published

## Phase 5+: Roadmap
The user promoted the planned rows on 2026-11-21. Source implementations are complete; acceptance remains tracked below.

- [x] Promote and implement [twelve system/app tools](phase5-system.md)
- [x] Promote and implement [ten media/vision tools](phase5-media.md)
- [x] Promote and implement [five presentation/time tools](phase5-time.md)
- [x] Implement [currency reference conversion](phase5-currency.md); raise [network issue #1](https://github.com/ninad-k/oxbox/issues/1)
- [ ] Full Phase 5 Windows/provider/hardware smoke matrix and cross-platform acceptance
- [ ] Human review and public release signoff

Download Video/Audio and Free Memory remain explicitly deferred/excluded by the plan.

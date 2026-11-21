# Oxbox task list

Details, acceptance criteria and dependencies for each task are in [plan.md](plan.md).
Work top to bottom; one task per branch/PR. Tick a box only when its acceptance criteria and AGENTS.md
"Definition of done" are met.

## Implementation status — 2026-11-21

The detailed v0.1, user-promoted Phase 5 and Mouse Activity source is integrated on `codex/complete-v0-1`.
All **49 catalogue entries** register their views; core tests, typed command groups, background lifecycle,
and documentation are present. Dashboard defaults to favorites only; the new original brand and normal
800 × 800 starting window are source changes awaiting current-revision native acceptance.
The original acceptance boxes below remain open where Windows manual checks, independent review, runtime privacy
or public release signoff are outstanding. [Public CI passed at the earlier 5117e6d revision](https://github.com/ninad-k/oxbox/actions/runs/44909470452);
that result does not certify the newer launcher or Mouse Activity revision.
These boxes are not a list of missing source implementations.

| Tasks | Source status | Acceptance still required |
|---|---|---|
| 1.1–1.6 | Implemented; automated coverage added | Full native interaction, restart/login/shortcut and hidden-notification smoke checks |
| 2.1–2.5 | Implemented; hostile-archive/path/cancellation tests added | Large-file UI/drop/Trash recovery checks; human security checkpoint |
| 3.1–3.6 | Implemented; lifecycle and fake-clock tests added | Real Windows power/break effects and Quit verification on a consenting test machine |
| 4.1–4.6 | Implemented; media/privacy tests added | Multi-monitor/scaling/capture/clipboard and OS-specific runtime checks |
| 4.7 | Earlier 5117e6d Windows draft artifacts and three-platform builds are recorded in validation; current revision is separate | Current-source packaging evidence, clean-machine/tool runtime acceptance, cold startup, WebView2 privacy gap, publication |
| Phase 5 system/apps | Twelve tools implemented | Real Windows window/display/eject/uninstall effects in a consenting test account |
| Phase 5 media/vision | Ten tools implemented | Device/provider/session-specific recording, OCR and color sampling checks |
| Phase 5 presentation/time | Five tools implemented | External-display presentation and native alarm/OS-control checks |
| Phase 5 currency | Offline import and opt-in ECB refresh implemented; issue #1 posted with approval | UI opt-in/cache/error verification and default-offline runtime observation |
| Mouse Activity | Windows-only explicit sessions, idle/input/expiry models, native adapter and fake-backend regressions implemented | Consenting Windows pointer/held-input/multi-monitor/Stop/Quit/restart acceptance |
| Launcher and website | Favorites-only dashboard, shortcut controls, original brand, square native frame and generated Pages site implemented | Current native UI/shortcut acceptance, browser QA and actual Pages deployment evidence |

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

## Launcher, Mouse Activity and website revision

- [x] Implement the [Windows-only Mouse Activity source and fake-backend tests](mouse-activity.md)
- [x] Implement favorites-only Dashboard, Ctrl+Alt+O / Cmd+Option+O default launcher, Alt+1–9 favorites, search and Quit shortcuts
- [x] Add original branding and a normal 800 × 800 starting native window
- [x] Add [GitHub Pages website source and generated catalog](../website/README.md)
- [x] Current native launcher/shortcut smoke and no-motion Mouse Activity Start, banner Stop, Quit and restart checks
- [ ] Mouse Activity idle movement, genuine-input return and full native monitor/desktop acceptance
- [x] Confirm website keyboard/mobile/filter behavior and [successful public Pages deployment](https://github.com/ninad-k/oxbox/actions/runs/44915821604)
- [ ] Complete strict runtime privacy, clean-machine installer, human security/license and full hardware acceptance gates

Download Video/Audio and Free Memory remain explicitly deferred/excluded by the plan.

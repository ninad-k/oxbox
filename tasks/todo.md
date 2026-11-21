# Oxbox task list

Details, acceptance criteria and dependencies for each task are in [plan.md](plan.md).
Work top to bottom; one task per branch/PR. Tick a box only when its acceptance criteria and AGENTS.md
"Definition of done" are met.

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
See the table in [plan.md](plan.md#phase-5-roadmap-after-v01-towards-60-tools). Promote a row to a task here when you start it.

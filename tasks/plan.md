# Implementation Plan: Oxbox

## Overview

Oxbox is a tray launcher with one-click utilities, built as Rust (`oxbox-core` + Tauri 2) with a Svelte 5 UI.
**Phase 0 (framework) is done**: tray, launcher with category sidebar, search, favourites, tool view frame,
design system, CI, and two reference tools (Transform Text, Verify Checksum).
Phases 1–4 deliver the remaining 18 tools of v0.1. Phase 5+ is the longer roadmap towards ~60 tools.

**Implementation update (2026-11-21):** the detailed Phase 1–4 source tasks are integrated; use
[todo.md](todo.md) for the distinction between implementation and remaining acceptance gates. The release is
not published. The workspace now requires Rust 1.93 for bounded 7z decoding ([ADR 0003](../docs/adr/0003-bounded-7z.md)).
The user subsequently promoted both the detailed phases and Phase 5+. The 48-tool source catalogue now includes
[system/apps](phase5-system.md), [media/vision](phase5-media.md), [presentation/time](phase5-time.md), and
[currency](phase5-currency.md) specifications. Deferred/excluded rows remain excluded. Opt-in currency networking
was explicitly approved and raised as [issue #1](https://github.com/ninad-k/oxbox/issues/1).
The Windows release privacy gate now has a separate [runtime privacy follow-up](runtime-privacy.md),
including user-approved existing SDK bindings and production network verification.

Every tool task below is sized for **one focused session of a coding agent** (a local LLM is fine).
Each one touches ≤ 5–6 files and follows the recipe in [`AGENTS.md`](../AGENTS.md#5-recipe-add-a-tool).
The checklist version of this plan is [`todo.md`](todo.md).

## Architecture decisions (already made, see `docs/adr/`)

- All logic in `crates/oxbox-core` (no Tauri). `src-tauri` holds thin commands, tray and window code.
- One tool = core module + command(s) + `api.ts` wrapper + Svelte view + registry entry.
- Offline only; destructive actions confirm and go to the Recycle Bin/Trash.
- Long jobs use the shared progress/cancel infrastructure (task 2.1). Background tools use Tauri managed state.

## How to run this plan with a local LLM

Give the agent **one task at a time** with this prompt (fill in the task id):

```
You are working in the Oxbox repository. First read AGENTS.md completely, then SPEC.md.
Implement task <ID> from tasks/plan.md, and nothing else.
Follow "Recipe: add a tool" in AGENTS.md. Write the failing Rust tests first.
Use only the dependencies listed in the task. Do not change unrelated files.
When done, run: cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings;
cargo test --workspace; pnpm check; pnpm test. Fix everything until all pass.
Then tick the task in tasks/todo.md and update the tool's row in README.md.
Reply with a summary of files changed and the test output.
```

Then run the app (`pnpm tauri dev`), try the tool by hand, and commit (`feat(<tool-id>): …`).
Smaller local models do better if you also paste the two reference files the task names.
At each **checkpoint**, have a stronger model (or a human) review the phase's diff before moving on.

---

## Phase 0: Framework ✅

- [x] 0.1 Cargo workspace (`oxbox-core`, `src-tauri`), Tauri 2 + SvelteKit SPA, icons
- [x] 0.2 Tray icon (left click toggles, menu: Open / Quit), close-to-tray
- [x] 0.3 Launcher UI: category sidebar, search (Ctrl/⌘K, `/`), grouped grid, favourites, tool view frame, planned-tool page, light/dark
- [x] 0.4 Reference tools: Transform Text (pure), Verify Checksum (file + blocking work)
- [x] 0.5 CI (fmt, clippy, tests on Win/macOS/Linux; UI check/test/build), release workflow (draft, on tag)
- [x] 0.6 Docs: SPEC, AGENTS, README, CONTRIBUTING, SECURITY, ADR 0001, licenses

---

## Phase 1: App essentials + simple tools (low risk)

### 1.1 Settings store, theme and launch at login
**Description:** Persist settings with `tauri-plugin-store`: theme (System/Light/Dark, sets `data-theme` on `<html>`),
launch at login (`tauri-plugin-autostart`), favourites (migrate from localStorage). Add a Settings view opened from a
gear button in the sidebar footer and from the tray menu.
**Acceptance:** settings survive restart; theme switches instantly; autostart toggles the OS login item; old localStorage favourites are migrated once.
**Verify:** `pnpm check && pnpm test`; manual: toggle each setting, restart app.
**Deps:** `tauri-plugin-store`, `tauri-plugin-autostart` (+ npm counterparts).
**Files:** `src-tauri/{Cargo.toml,src/lib.rs,capabilities/default.json}`, `src/lib/settings.svelte.ts`, `src/lib/ui/Settings.svelte`, `src/lib/favorites.svelte.ts`. **Size:** M

### 1.2 Global shortcut and single instance
**Description:** `Ctrl+Alt+Space` (⌘⌥Space on macOS) toggles the launcher from anywhere and focuses search; configurable in Settings.
A second launch of the app focuses the existing window instead of starting a new one.
**Acceptance:** shortcut works with the window hidden; conflicting shortcut shows an error in Settings; only one instance runs.
**Verify:** manual on Windows; clippy/tests green.
**Deps:** `tauri-plugin-global-shortcut`, `tauri-plugin-single-instance`.
**Files:** `src-tauri/{Cargo.toml,src/lib.rs,src/tray.rs,capabilities/default.json}`, `src/lib/ui/Settings.svelte`. **Size:** S

### 1.3 Unit Converter (`unit-converter`)
**Description:** Convert length, mass, temperature, data size (SI and binary), time, speed, area, volume. Pure Rust tables.
**Acceptance:** unit tests for every category incl. temperature offsets and binary vs decimal bytes; UI converts as you type, swap button, copy result.
**Deps:** none. **Files:** `oxbox-core/src/tools/unit_convert.rs`, `commands.rs`, `lib.rs`, `api.ts`, `tools/UnitConverter.svelte`, `registry.ts`. **Size:** M

### 1.4 Timer & Stopwatch (`timer-stopwatch`)
**Description:** Countdown (presets 1/5/10/25 min + custom) with a system notification at zero; stopwatch with laps.
Timing runs in the UI (`performance.now()`), not polling Rust.
**Acceptance:** timer keeps correct time when the window is hidden; notification fires; laps listed; keyboard: Space start/pause, R reset.
**Deps:** `tauri-plugin-notification`. **Files:** `src-tauri/{Cargo.toml,src/lib.rs,capabilities/default.json}`, `tools/TimerStopwatch.svelte`, `src/lib/time.ts` + `time.test.ts`, `registry.ts`. **Size:** M

### 1.5 World Time (`world-time`)
**Description:** List of clocks for user-chosen cities/time zones, with offset from local time and day/night hint; add/remove/reorder; persisted in settings.
**Acceptance:** core function returns local time + UTC offset for an IANA zone (tested incl. DST change dates); unknown zone → `InvalidInput`; searchable zone picker.
**Deps:** `chrono`, `chrono-tz`. **Files:** `oxbox-core/src/tools/world_time.rs`, `commands.rs`, `lib.rs`, `api.ts`, `tools/WorldTime.svelte`, `registry.ts`. **Size:** M

### 1.6 QR & Barcode (`qr-code`)
**Description:** Generate QR (error-correction level, size) and Code 128 / EAN-13 barcodes from text; preview as SVG; save PNG/SVG via save dialog; copy image.
**Acceptance:** QR output round-trips through a decoder in tests (`rqrr` dev-dependency); EAN-13 validates checksum digit; empty input shows hint.
**Deps:** `qrcode`, `barcoders`, `image`; dev: `rqrr`. **Files:** `oxbox-core/src/tools/qr.rs`, `commands.rs`, `lib.rs`, `api.ts`, `tools/QrCode.svelte`, `registry.ts`. **Size:** M

### ✔ Checkpoint 1
All checks green in CI; manual pass of each Phase 1 tool on Windows; review the diff before Phase 2.

---

## Phase 2: File tools (progress, cancel, safe deletion)

### 2.1 Jobs infrastructure: progress + cancel
**Description:** Shared mechanism for long-running tools. Core: `Progress { done, total, message }` callback +
`CancelToken(Arc<AtomicBool>)` → `CoreError::Cancelled`. Tauri: `start_job` returns a job id, emits
`tool://<tool-id>/progress`, `cancel_job(id)`. UI: `src/lib/jobs.ts` helper + `ProgressBar.svelte`.
Retrofit Verify Checksum to show progress for big files.
**Acceptance:** unit test cancels a long core loop; progress events throttled to ≤ 10/s; checksum of a 2 GB file shows progress and can be cancelled.
**Deps:** none new. **Files:** `oxbox-core/src/jobs.rs`, `src-tauri/src/jobs.rs`, `commands.rs`, `src/lib/jobs.ts`, `src/lib/ui/ProgressBar.svelte`, `tools/VerifyChecksum.svelte`. **Size:** M

### 2.2 Shared file drop + trash helper
**Description:** `FileDrop.svelte` (drag files/folders onto a tool, plus "Choose…" fallback) using Tauri's drag-drop event;
core `fs_safe::move_to_trash(paths)` using the `trash` crate, refusing system roots.
**Acceptance:** dropping files on Verify Checksum works; trash helper unit-tested on temp files; refuses `C:\`, `/`, home dir root.
**Deps:** `trash`. **Files:** `src/lib/ui/FileDrop.svelte`, `oxbox-core/src/fs_safe.rs`, `commands.rs`, `api.ts`, `tools/VerifyChecksum.svelte`. **Size:** S

### 2.3 Find Duplicates (`find-duplicates`)
**Description:** Pick folders → walk (skip symlinks, hidden optional) → group by size → hash first 64 KB → full BLAKE3 for remaining candidates.
Results grouped with total wasted space; select copies (auto-select "keep oldest/newest"); move selected to trash after confirmation.
**Acceptance:** tests with temp trees: identical files with different names found, same-size different content not grouped, empty files ignored; progress + cancel; nothing deleted without confirm.
**Deps:** `walkdir`, `blake3` (have). **Files:** `oxbox-core/src/tools/duplicates.rs`, `commands.rs`, `lib.rs`, `api.ts`, `tools/FindDuplicates.svelte`, `registry.ts`. **Size:** L (split UI into a second session if needed)

### 2.4 Archive & Unarchive (`archive`)
**Description:** Create `.zip` / `.tar.gz` from files/folders; extract `.zip`, `.tar`, `.tar.gz`, `.7z` to a folder next to the archive (or chosen).
**Acceptance:** round-trip tests; **zip-slip protection** (entries with `..` or absolute paths rejected, tested); guard against zip bombs (abort if extracted size > 20× archive or > user-confirmed limit); progress + cancel; never overwrite without asking.
**Deps:** `zip`, `tar`, `flate2`, `sevenz-rust2` (extract only). **Files:** `oxbox-core/src/tools/archive.rs`, `commands.rs`, `lib.rs`, `api.ts`, `tools/Archive.svelte`, `registry.ts`. **Size:** L

### 2.5 Clean Drive (`clean-drive`)
**Description:** Scan known junk locations per OS (user temp, browser caches, thumbnail caches, package-manager caches, Recycle Bin size) and show sizes per category; user selects categories → move contents to trash (or empty for temp files older than 24 h).
**Acceptance:** location list per OS is a pure function (unit-tested); scan on a fake root in tests; never touches paths outside the allow-list; dry-run total shown before confirmation.
**Deps:** `walkdir`, `trash`, `dirs`. **Files:** `oxbox-core/src/tools/clean_drive.rs`, `platform/*`, `commands.rs`, `api.ts`, `tools/CleanDrive.svelte`, `registry.ts`. **Size:** L

### ✔ Checkpoint 2
Security review of 2.2–2.5 (path handling, deletion, archive extraction) using `agent-skills:security-and-hardening` or a human.

---

## Phase 3: System & power

### 3.1 System Monitor (`system-monitor`)
**Description:** CPU (total + per core), memory, swap, disks (used/free), top 10 processes by CPU/memory; refresh every 1 s while the view is open only.
**Acceptance:** core returns a serialisable snapshot (tested for shape/invariants); polling stops when leaving the view; sparkline of last 60 s CPU.
**Deps:** `sysinfo`. **Files:** `oxbox-core/src/tools/system_monitor.rs`, `commands.rs` (managed `System` state), `lib.rs`, `api.ts`, `tools/SystemMonitor.svelte`, `registry.ts`. **Size:** M

### 3.2 Network Usage (`network-usage`)
**Description:** Live download/upload speed per interface and session totals; optional tray tooltip with current speed.
**Acceptance:** rate calculation is a pure function over two samples (tested, incl. counter reset); loopback/virtual interfaces hidden by default.
**Deps:** `sysinfo`. **Files:** `oxbox-core/src/tools/network.rs`, `commands.rs`, `api.ts`, `tools/NetworkUsage.svelte`, `registry.ts`. **Size:** M

### 3.3 Do Not Sleep (`do-not-sleep`)
**Description:** Keep system (and optionally display) awake: indefinitely, for N minutes, or until a time. Tray menu item toggles it; tray tooltip shows state.
**Acceptance:** managed state with start/stop/status; awake lock released on stop and on app exit; works on Windows/macOS/Linux via crate.
**Deps:** `keepawake`. **Files:** `oxbox-core/src/tools/keep_awake.rs`, `src-tauri/src/{commands.rs,lib.rs,tray.rs}`, `api.ts`, `tools/DoNotSleep.svelte`. **Size:** M

### 3.4 Lock Screen (`lock-screen`)
**Description:** Lock immediately. Windows: `LockWorkStation` (`windows-sys`); macOS: lock via `pmset displaysleepnow` with a note that "require password immediately" must be on; Linux: `loginctl lock-session`. Also a tray menu item.
**Acceptance:** command builder per OS unit-tested; Windows verified by hand.
**Deps:** `windows-sys` (Windows only, feature `Win32_System_Shutdown`). **Files:** `oxbox-core/src/{tools/lock.rs,platform/*}`, `commands.rs`, `api.ts`, `tools/LockScreen.svelte`, `registry.ts`. **Size:** S

### 3.5 Sleep Timer (`sleep-timer`)
**Description:** After a countdown, sleep / shut down / restart / sign out. Persistent banner with Cancel; 60-second warning notification.
**Acceptance:** OS command table unit-tested (args as arrays, no shell strings); cancel always works; confirmation before arming shutdown.
**Deps:** `windows-sys` (Windows power APIs). **Files:** `oxbox-core/src/tools/sleep_timer.rs`, `platform/*`, `commands.rs`, `lib.rs`, `tools/SleepTimer.svelte`. **Size:** M

### 3.6 Break Time (`break-time`)
**Description:** Every N minutes of activity, notify to take a break (snooze 5 min / skip); optional 20-20-20 eye-rest mode; pause during fullscreen (best effort).
**Acceptance:** scheduler logic is pure and tested with a fake clock; settings persisted; runs while window is hidden.
**Deps:** `tauri-plugin-notification` (have from 1.4). **Files:** `oxbox-core/src/tools/break_time.rs`, `src-tauri/src/{commands.rs,lib.rs}`, `api.ts`, `tools/BreakTime.svelte`, `registry.ts`. **Size:** M

### ✔ Checkpoint 3
Manual run of all power tools on Windows (careful with Sleep Timer); verify nothing keeps running after Quit.

---

## Phase 4: Capture, media, privacy → v0.1.0

### 4.1 Screenshot: screen and window (`screenshot`)
**Description:** Capture full screen (choose monitor) or a window (picker list with titles) → save PNG to `Pictures/Oxbox/` with timestamp and copy to clipboard; optional delay 0/3/5/10 s; "Open folder" button.
**Acceptance:** file naming is a tested pure function; multi-monitor works; launcher window hides itself before capture.
**Deps:** `xcap`, `arboard`, `image`, `dirs`. **Files:** `oxbox-core/src/tools/screenshot.rs`, `commands.rs`, `lib.rs`, `api.ts`, `tools/Screenshot.svelte`, `registry.ts`. **Size:** M

### 4.2 Screenshot: area selection
**Description:** Transparent, always-on-top, full-screen overlay window per monitor to drag a rectangle (Esc cancels); crop the captured image to the selection.
**Acceptance:** crop math tested incl. HiDPI scale factors; works on Windows with 100%/150% scaling.
**Deps:** none new. **Files:** `src-tauri/src/overlay.rs`, `src/routes/overlay/+page.svelte`, `oxbox-core/src/tools/screenshot.rs`, `commands.rs`. **Size:** L

### 4.3 Clipboard History (`clipboard-history`)
**Description:** Background watcher (poll 500 ms) records text and images (dedupe, max 200 items / 50 MB); search, pin, click to copy back, delete, clear all, pause. History kept in memory; "remember across restarts" off by default.
**Acceptance:** ring-buffer logic tested; ignores content marked as concealed where the OS exposes it (e.g. Windows `ExcludeClipboardContentFromMonitorProcessing`); privacy note shown in the view.
**Deps:** `arboard`. **Files:** `oxbox-core/src/tools/clipboard_history.rs`, `src-tauri/src/{commands.rs,lib.rs}`, `api.ts`, `tools/ClipboardHistory.svelte`, `registry.ts`. **Size:** L

### 4.4 Resize Images (`resize-images`)
**Description:** Batch resize by percentage / max width-height / exact size (keep aspect), convert PNG/JPEG/WebP, JPEG quality; output to a chosen folder; never overwrite originals by default.
**Acceptance:** dimension maths tested; EXIF orientation respected; progress + cancel (uses 2.1).
**Deps:** `image`, `fast_image_resize`. **Files:** `oxbox-core/src/tools/resize.rs`, `commands.rs`, `api.ts`, `tools/ResizeImages.svelte`, `registry.ts`. **Size:** M

### 4.5 Hide Desktop (`hide-desktop`)
**Description:** Toggle desktop icons. Windows: show/hide the desktop `SysListView32` window (no registry change, restored on exit). macOS: `defaults write com.apple.finder CreateDesktop -bool false` + restart Finder (and restore). Linux: "not available".
**Acceptance:** state always restored on Quit; tray toggle; Windows verified by hand.
**Deps:** `windows-sys` (Win32_UI_WindowsAndMessaging). **Files:** `oxbox-core/src/{tools/hide_desktop.rs,platform/*}`, `commands.rs`, `lib.rs`, `tools/HideDesktop.svelte`. **Size:** M

### 4.6 Encrypt Files (`encrypt-files`)
**Description:** Encrypt files with a passphrase to `.age` (scrypt), decrypt `.age` files; passphrase + confirm field with strength hint; originals kept unless user opts to move them to trash.
**Acceptance:** encrypt→decrypt round-trip test; wrong passphrase → clear error; streams large files; passphrase never logged or stored.
**Deps:** `age`. **Files:** `oxbox-core/src/tools/encrypt.rs`, `commands.rs`, `api.ts`, `tools/EncryptFiles.svelte`, `registry.ts`. **Size:** M

### 4.7 Release v0.1.0
**Description:** README screenshots (light + dark), changelog, version bump, tag `v0.1.0` → release workflow builds draft installers; smoke-test the Windows installer on a clean machine/VM; publish.
**Acceptance:** installers for Windows, macOS (arm64 + x64) and Linux attached; Windows installer < 15 MB.

### ✔ Checkpoint 4: v0.1.0 shipped

---

## Phase 5+: Roadmap after v0.1 (towards ~60 tools)

Each becomes a task in the same format when picked up. Suggested approach in brackets.

| Area | Tool | Approach |
|---|---|---|
| Windows | Window on Top | Win32 `SetWindowPos(HWND_TOPMOST)`; macOS needs Accessibility API |
| | Window Manager (snap halves/thirds/quarters) | Global shortcuts + Win32 / macOS AX / X11 (EWMH) |
| | Show Desktop / Close Apps | Win32 `EnumWindows` + `WM_CLOSE` with confirmation |
| | Focus on Window | Dimming overlay windows around the active window |
| Presentation | Presentation Mode | Combine Do Not Sleep + Hide Desktop + Do Not Disturb; auto-on with external display |
| | Do Not Disturb | Windows Focus Assist / macOS Focus (limited APIs; research first) |
| Audio/video | Mute Microphone | Windows Core Audio (`windows` crate) / CoreAudio / PulseAudio |
| | Record Audio | `cpal` + `hound` (WAV), optional Opus |
| | Record Screen / Area / Window | ffmpeg sidecar (bundled, LGPL build) or `scap` |
| | Make GIF | `gifski` crate |
| | Convert Audio / Video | ffmpeg sidecar with presets |
| | Take Photo / Take Video | `nokhwa` camera crate |
| Vision | Recognize Text (OCR) | Windows.Media.Ocr (`windows` crate), macOS Vision, Linux `tesseract` optional |
| | Barcode Reader | `rqrr` / `rxing` on screenshots or images |
| | Measure Color (picker) | Screen sample under cursor via `xcap`; HEX/RGB/HSL/CMYK |
| System | Switch Resolution | Win32 `ChangeDisplaySettingsEx`; macOS CoreGraphics |
| | Eject Volumes | Windows `CM_Request_Device_Eject`; macOS `diskutil eject`; Linux `udisksctl` |
| | Energy Saver | Switch Windows power plan / macOS `pmset` low power mode |
| | Airplane Mode | Limited APIs on all OSes; research first |
| | Hidden Files toggle | Windows Explorer setting; macOS `AppleShowAllFiles` |
| | CPU Temperature | `sysinfo` Components (limited on Windows; may need WMI/LibreHardwareMonitor) |
| Apps | Launch (app sets) | Saved list of apps/files/URLs opened via `opener` |
| | Uninstall Apps | Windows uninstall registry keys + leftovers scan (careful; confirm) |
| Time | Alarm, Date Countdown | Extend Timer infrastructure |
| Network (needs approval: breaks "offline") | Currency rates in Unit Converter | Opt-in, cached daily |
| Deferred | Download Video/Audio | Not planned: site terms-of-service issues |
| Excluded | Free Memory | Not planned: modern OSes manage memory; "RAM cleaners" are placebo or harmful |

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| OS APIs differ widely (lock, hide desktop, DND) | High | Per-OS modules, honest "not available" state, Windows first |
| Destructive tools delete wrong files | High | Allow-lists, trash not delete, confirmation with totals, security checkpoint 2 |
| Untrusted archives (zip-slip, bombs) | High | Path validation + size guards with tests (task 2.4) |
| Local LLM produces inconsistent code | Medium | One task per session, strict AGENTS.md recipe, CI gates, checkpoint reviews |
| Unsigned installers scare users | Medium | Document warnings; revisit signing (SignPath free OSS program, Apple Developer ID) |
| WebView differences across OSes | Low | Standard CSS only; CI builds all three |

## Open questions

- Code signing: apply to SignPath Foundation (free for OSS) for Windows signing before v0.2?
- Should tool views be openable in their own small windows (pop-out) in addition to the launcher? (Post-v0.1.)

# Changelog

Changes that matter to users and contributors are recorded here. Version 0.1.0 is a release target;
the presence of this changelog does not mean installers have been published.

## 0.1.0 — draft, 2026-11-21

### Release review fixes — 2026-11-22

- Installer running-app checks no longer force-terminate Oxbox and bypass its cleanup. NSIS asks for tray Quit;
  MSI disables automatic Restart Manager shutdown while retaining files-in-use detection.
- Duplicate cleanup revalidates selected copies and kept files against the scanned content before any Trash call.
- Provider discovery rejects unsafe Windows network/device/reparse paths before probing files. Provider checks
  honor cancellation, and owned process trees are terminated before joining inherited pipes on exit or timeout.
- Password fields discard undo history so cleared drafts cannot be restored. New age files use scrypt factor 18;
  decryption accepts through factor 20 and clearly refuses larger factors without changing the original.
- Added contextual accessible names, live user-action errors/status and quiet passive polling, plus stronger
  shared control boundaries. Headless name checks cover all 49 default forms and selected expanded/two-row states.
- Added guarded, reproducible installer/endpoint verification scripts and acceptance records. Existing CI actions
  are pinned to verified upstream commits, with weekly Dependabot updates for GitHub Actions.
- The release remains a draft. Clean-machine, DNS/provider/platform, screen-reader/hardware and human review
  gates remain explicit; the Linux GLib finding has no compatible published 0.18 patch.

### Installed startup default

- Fresh installed Windows/macOS copies enable current-user login startup on first launch; subsequent logins
  open quietly in the tray. Settings allows opt-out, and upgrades preserve existing/OS-disabled choices.
- Development and isolated profiles cannot register login startup. Default startup never enables Mouse Activity,
  clipboard history or recordings. Startup preference writes roll back OS registration on failure.
- Windows command paths are quoted and uninstall cleanup checks exact ownership before removing an entry.

### Native renderer migration

- Replaced the production browser host with exact eframe 0.37.0 and raised the minimum Rust version to
  1.95, following the user-approved ADR 0004. All 49 stable tools have native forms; the former Tauri/Svelte
  source remains available for comparison. There is no hidden fallback webview.
- Shared `catalog/tools.json` now drives the native launcher, website and legacy registry adapters.
- Added native tray/shortcut registration rollback, local single-instance activation, atomic preference
  storage, bounded cancellable workers, independent reminder scheduling and stop/join cleanup.
- Preserved legacy stores while importing into separate native files; currency download consent resets off.
- Restored exact keyboard modifier matching, Escape/Slash navigation and tray controls. Native dialogs retain
  their parent window and restore focus on accept/cancel. Clipboard Pause/Clear/consent withdrawal supersedes
  pending Start, and Quit prevents late clipboard workers from reopening.
- Native manual, platform, installer, hardware, accessibility and privacy acceptance remains tracked separately
  from automated tests. The release stays an unpublished draft until those checks are complete.

### Launcher, Mouse Activity and website revision

- Added Windows-only **Mouse Activity**, bringing the catalog to 49 tools. Sessions require explicit Start,
  wait for a configurable idle delay, pause on resumed input, and offer Stop in the tool, banner and tray.
  Optional duration, Stop, error and Quit release the worker. No clicks, typing or scrolling are generated.
  Tests use fake backends. A bounded Windows production check observed two idle pointer movements,
  pause on controlled input and Stop; full physical-input/hardware acceptance remains open.
- Made Dashboard a favorites-only view, with All tools and category browsing available separately. The normal
  resizable native window starts at 800 × 800 logical pixels and uses the original outlined O/ember brand.
- Changed the default global launcher binding to Ctrl+Alt+O, or Cmd+Option+O on macOS. Existing saved bindings
  remain in effect. Alt+1–9 opens favorites, Ctrl/Cmd+K searches, and Ctrl/Cmd+Q quits inside the app.
- Added a responsive, static GitHub Pages website with a 49-tool catalog generated from shared JSON,
  search/category/platform filters, provider notes and contribution guidance. It uses the same
  SVG brand asset as the app, without external fonts, analytics or CDN scripts.
- Fixed website Reset so the search fields, visible cards and result status update together; regression coverage
  models form-reset default-action ordering.
- The retained legacy host includes app-scoped WebView2 SDK controls: an isolated profile, custom crash reporting,
  OS-account SSO disabled and external web-resource blocking. Its browser-owned background requests motivated
  the native migration; it is not the host in the new native installers.
- Native Windows MSI/NSIS packages were built, inspected and uploaded to the existing unpublished draft;
  downloaded hashes match. Windows/macOS ARM/Linux native CI and Rust 1.95 checks passed. Bounded Windows
  production endpoint observations found no owned TCP/UDP endpoints within their stated intervals.
- Full native tool/hardware/accessibility, clean-machine installer, DNS/other-OS runtime and human security/license
  acceptance remain open. Draft installer and publication status remain separate from source changes.

### Added

- Persistent appearance and favorites, migration of legacy favorites, launch at login, configurable global
  launcher shortcut, and single-instance behavior.
- Unit Converter for eight categories; Timer & Stopwatch with laps; searchable IANA World Time;
  QR, Code 128, and EAN-13 generation with SVG/PNG export.
- Shared cancellable jobs, progress events, file dropping, streaming checksum progress, duplicate finding,
  ZIP/tar.gz creation, ZIP/tar/tar.gz/7z extraction, and allow-listed cache cleanup through Trash.
- Local CPU/memory/disk/process monitoring, per-interface network statistics, Do Not Sleep, Lock Screen,
  cancellable Sleep Timer, and background Break Time reminders.
- Screen/window/area capture, memory-only clipboard history with explicit start, batch image resizing and
  PNG/JPEG/WebP conversion, desktop-icon hiding/restoration, and streaming age passphrase encryption.
- Contributor guides, tool usage, test and release checklists, dependency notes, and architecture records.
- User-promoted Phase 5 catalogue: window positioning/topmost/dimming, desktop/app closing, display rollback,
  removable-volume eject, power profiles, hidden files, temperatures, local app sets and reviewed uninstallers.
- Presentation Mode, documented DND/Airplane system controls, persistent time-zone-aware alarms and date countdowns.
- Explicit microphone/camera/screen recording, local FFmpeg media conversion/GIF tools, native/Tesseract OCR,
  QR/1D barcode reading, and screen color sampling. Optional providers are user-installed and not redistributed.
- Currency conversion inside Unit Converter and as a separate tool, with bounded offline XML import and default-off
  ECB refresh after the approved [network issue #1](https://github.com/ninad-k/oxbox/issues/1).

### Safety and compatibility

- File outputs use new filenames; originals are preserved. File cleanup requires review and confirmation.
- Archive extraction rejects traversal, links, special files, duplicate paths, and excessive output.
- 7z uses a conservative four-coder profile, cumulative model checks and 128 KiB frozen stored/decoded
  metadata limits. Some valid archives outside this resource policy are rejected with a clear error.
- Rust MSRV raised from 1.90 to 1.93 to use the memory-limited 7z reader; see [ADR 0003](docs/adr/0003-bounded-7z.md).
- Hide Desktop is unavailable on Linux. Capture and clipboard support depends on the desktop session and OS permissions.
- Installer, platform, and release signoff remain tracked separately in [the release checklist](docs/RELEASE.md).
- Release review added recording/overlay Quit cancellation, bounded age and TAR metadata parsing, plain Windows
  output filename checks, settings-save rollback, monotonic Sleep Timer display and presentation cleanup regressions.
- Updated the SvelteKit build chain to patched 3.x; configuration lives in `vite.config.js`, with explicit
  `#lib/*` package subpath imports. Node.js 22.17+ is required.
- Updated ZIP to 9.8 and rqrr to 0.11 to remove the ZIP and LRU audit findings. Upstream maintenance and
  Linux GLib warnings are recorded with their scope in [dependency notes](docs/DEPENDENCIES.md).
- Historical Tauri CI passed Windows, macOS and Ubuntu 24.04 builds, UI checks and Rust 1.93. Native
  CI uses Rust 1.95 and is recorded independently in the validation record.
- Windows runtime QA observed independent WebView2 HTTPS traffic. App telemetry claims are explicitly scoped,
  and strict whole-process offline acceptance remains open; see [runtime privacy findings](docs/RUNTIME_PRIVACY.md).

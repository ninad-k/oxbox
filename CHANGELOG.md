# Changelog

Changes that matter to users and contributors are recorded here. Version 0.1.0 is a release target;
the presence of this changelog does not mean installers have been published.

## 0.1.0 — draft, 2026-11-21

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

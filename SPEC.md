# Spec: Oxbox

> A free, open-source, cross-platform toolbox of one-click desktop utilities, written in Rust.
> Status: **Approved 2026-11-21** · License: MIT OR Apache-2.0

## Objective

Oxbox gives people one tray/menu-bar app with many small utilities they would otherwise install separately
(keep awake, screenshots, checksums, duplicate finder, QR codes, and so on).
It is an open-source alternative to commercial toolboxes such as Parallels Toolbox.

**Users:** everyday Windows, macOS and Linux users and developers who want quick utilities without ads,
subscriptions or telemetry.

**How we build it (clean room):** we rebuild the *functionality* described in public product pages.
We do **not** decompile, disassemble or copy any proprietary binaries, code, icons, names, text or UI art.
Oxbox-specific code and copy are original; third-party libraries and Lucide icons retain their licenses and attribution.

**Success looks like:** a user installs a <15 MB app, clicks the tray icon, picks a tool from a searchable grid,
and gets the result in one or two clicks, on all three OSes.

### User stories (v0.1)
1. As a user, I click the tray icon and see a searchable grid of tools grouped by category.
2. As a user, I can pin favourite tools and launch a tool with a global shortcut.
3. As a user, each tool works offline, does nothing destructive without confirmation, and sends no data anywhere.
4. As a contributor, I can add a new tool by adding one Rust module in `oxbox-core`, one native form and one canonical catalogue entry.

## Original v0.1 capability map (20 tools + shell)

| Module id | Responsibility | Depends on |
|---|---|---|
| `shell` | Tray icon, launcher window, tool registry, search, favourites, settings, global shortcuts, notifications | — |
| `power` | **Do Not Sleep**, **Sleep Timer** (sleep/shutdown after N min), **Lock Screen** | shell |
| `privacy` | **Hide Desktop** (desktop icons), **Encrypt Files** (age, passphrase) | shell |
| `capture` | **Capture Screen / Window / Area** (screenshots to file + clipboard) | shell |
| `clipboard` | **Clipboard History** (text + images, local only, capped size) | shell |
| `files` | **Verify Checksum** (SHA-256/SHA-1/MD5/BLAKE3), **Find Duplicates**, **Archive / Unarchive** (zip, tar.gz; extract 7z), **Clean Drive** (temp/cache scan; delete to Recycle Bin/Trash) | shell |
| `media` | **Resize Images** (batch, format convert), **Barcode / QR Generator** | shell |
| `system` | **System Monitor** (CPU, RAM, disk, processes), **Network Usage** (live speed + session totals) | shell |
| `time` | **Break Time** reminders, **Timer**, **Stopwatch**, **World Time** | shell |
| `text` | **Transform Text** (UPPER, lower, Title, camelCase, snake_case, kebab-case…), **Unit Converter** (length, mass, temperature, data size, time; approved currency is now a separate promoted tool) | shell |

Build order: `shell` → `text`, `files` (pure logic, easiest to test) → `system`, `power`, `time` → `media`, `capture`, `clipboard`, `privacy`.

### Platform support targets (v0.1)
- **Windows 10/11:** primary development target. Native spot checks are recorded, while full tool-by-tool acceptance remains open.
- **macOS 12+ and Linux (X11/Wayland, GNOME/KDE):** native builds are configured in CI; current results must be checked. Where an OS API differs, the tool either has an
  implementation or is shown as "not available on this OS". It is never shown broken.
  Known gaps: Hide Desktop on Linux varies by desktop environment; window capture on Wayland is limited.

### Roadmap after v0.1 (subsequently promoted)
Window Manager / Window on Top, Presentation Mode, Do Not Disturb, Mute Microphone, Eject Volumes, Recognize Text (OCR),
Barcode Reader, Measure Color, Switch Resolution, Record Screen/Audio (ffmpeg), Make GIF, Convert Audio/Video,
Uninstall Apps, Launch (app sets), Energy Saver, Airplane Mode, Date Countdown, Alarm, Currency rates.
(Download Video/Audio is deliberately deferred: it raises site terms-of-service issues.)

On 2026-11-21 the user explicitly promoted the planned Phase 5 rows as well as Phases 1–4. Detailed promoted
task files are linked from `tasks/todo.md`. A later explicitly authorized Windows-only Mouse Activity tool brings
the source catalog to **49 tools**; its idle-aware sessions require explicit Start and native acceptance is pending.
The launcher revision uses a favorites-only Dashboard, an original outlined O/ember brand, and a normal
800 × 800 starting window. The excluded/deferred tools remain excluded.
Core commands remain local by default, with explicit opt-in ECB currency refresh allowed after the
user-authorized [network issue #1](https://github.com/ninad-k/oxbox/issues/1). Those requests send no user amounts or files.
The strict whole-process offline goal in user story 3 is not yet accepted on Windows: release QA observed
background WebView2 HTTPS traffic; see [runtime privacy findings](docs/RUNTIME_PRIVACY.md).
An isolated-profile diagnosis subsequently identified vendor configuration and component-update requests;
production testing of supported SDK controls also observed SmartScreen requests while retaining security defaults.
The user approved supported app-scoped SDK controls and existing bindings; strict offline acceptance remains
open. The selected native migration removes the embedded browser dependency; it still requires new production observation and the [migration acceptance gates](tasks/native-migration.md). The earlier [SDK follow-up](tasks/runtime-privacy.md) remains historical evidence.

## Selected native architecture

The user selected **eframe 0.37.0 and Rust 1.95** on 2026-11-21. All 49 tools now have concrete native
forms, reusing the Rust core and existing platform/provider limits. A live Windows native window and
headless route construction have been observed; full native tool, accessibility, privacy and installer
acceptance remains open in [the migration matrix](tasks/native-migration.md).
The retained Tauri/Svelte host is a comparison implementation, not the current native runtime.
[ADR 0004](docs/adr/0004-runtime-offline-boundary.md) records the selected stack and its limits.

| Layer | Choice | Purpose |
|---|---|---|
| Language | Rust, edition 2024, minimum 1.95 | Shared core and native host; exact selected eframe line requires 1.95 |
| UI/renderer | eframe/egui 0.37.0, Glow, bundled/system fonts, AccessKit | Native forms, previews, keyboard/accessibility support; no embedded webview or automatic remote loader |
| Desktop integration | rfd, tray-icon, global-hotkey, auto-launch, open, platform notification adapters | Explicit native dialogs, launcher lifecycle, login setting and user-requested open actions |
| Services | Standard-library workers, bounded Task progress, atomic local storage | Hidden reminders, cancellation/Stop/Quit ownership and recoverable preferences |
| Metadata/assets | catalog/tools.json and flattened existing Lucide vectors | One ordered 49-tool capability catalogue for native, legacy and website interfaces |
| Core | Existing file, crypto, capture, media, system and time crates | Toolkit-neutral operations and tested safety policies; resolved versions in Cargo.lock |
| Development | Cargo; Node/pnpm for legacy checks, static website and existing packaging wrapper | Native runtime does not execute JavaScript or need WebView2 |

Exact package versions/features are in the manifests and lockfile. New dependencies, render backends or
installer tools outside the approved decision still require approval. User-installed FFmpeg/Tesseract
providers remain optional and unbundled.

## Commands

    cargo run -p oxbox-native --locked
    cargo test -p oxbox-core -p oxbox-native --locked
    cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
    cargo fmt --all -- --check
    node scripts/build-native.mjs --check
    node scripts/build-native.mjs --bundle
    pnpm check
    pnpm test
    pnpm build
    node scripts/build-website.mjs
    node --test scripts/website.test.mjs

The pnpm checks retain the legacy UI/catalogue comparison. Use pnpm tauri dev only for an explicit legacy
comparison. Native packaging selects oxbox-native and verifies its graph excludes browser-host crates;
the existing bundler is a packaging tool, not the native runtime.

## Project structure

- crates/oxbox-core/src/tools/: tool logic and normal/edge/error tests; platform/ holds OS adapters.
- crates/oxbox-core/src/{launcher,preferences}.rs: shared metadata/search and bounded atomic storage.
- crates/oxbox-native/src/app.rs: native favorites/search/settings and window lifecycle.
- crates/oxbox-native/src/{host,jobs,integration,instance}.rs: notifications, tasks, OS integration and activation.
- crates/oxbox-native/src/tools/{system,media,utilities}/: all 49 concrete native forms and owned controllers.
- catalog/: canonical category/tool metadata and existing licensed vector assets.
- src/ and src-tauri/: retained Svelte/Tauri parity reference and typed adapters.
- website/: static Pages output built from the canonical JSON, without executing app imports.
- tasks/native-migration.md: per-tool source/automation/manual matrix; docs/VALIDATION.md: dated evidence.

## Code style and service boundary

Core logic has no UI imports and returns CoreError. Never unwrap()/expect() user input in core. Native
forms call typed core APIs directly; disk/CPU work uses Task with CancelToken and Progress. For example:

    self.hash.start(ui.ctx(), move |cancel, progress| {
        checksum::hash_file_with_progress(&path, algorithm, &cancel, progress)
    });

Jobs preserve streaming, exclusive-output and Trash policies. Navigation abandons stale view jobs;
hiding preserves in-flight work. Explicit background controllers and reminder clocks run independently
of UI frames and stop/join on Quit. Host storage imports legacy files into separate native stores without
rewriting originals; invalid input/storage is an error, not an implicit reset. Passphrases are transient.

Native grouped form modules are permitted; every stable ID must render real controls and actual results.
A generic JSON editor, mock completion or placeholder view is not an implementation. Follow AGENTS.md.

## Testing Strategy

- **Unit tests (core, required):** every pure function: text transforms, unit conversions, checksum known vectors,
  duplicate grouping, archive round-trip, encrypt→decrypt round-trip, QR output decodes, image resize dimensions,
  clean-drive scanning (on a temp-dir fixture; never touches real user dirs in tests).
- **Integration tests (core):** `crates/oxbox-core/tests/` with `tempfile`.
- **OS-effect tools** (lock screen, keep awake, hide desktop, sleep timer, screenshot): logic is behind a small
  trait so command construction is unit-tested; the real effect is checked by hand on Windows and listed in the PR.
- **Native UI/services:** headless route/fake-backend tests, consumed texture deltas, task/instance/settings regressions; real keyboard, AccessKit, GPU, dialog and device behavior requires manual testing.
- **Legacy/catalogue/site:** Vitest and svelte-check remain comparison checks; canonical metadata and website filtering must agree.
- **CI:** native Rust 1.95 fmt/clippy/tests/build on Windows, macOS and Ubuntu; legacy UI and static-site checks are separate. CI compilation is not cross-platform runtime acceptance.
- Coverage goal: ≥80% lines in `oxbox-core` pure modules.

## Boundaries

- **Always:** keep tool logic in `oxbox-core`; confirm before deleting, overwriting or shutting down; delete to
  Recycle Bin/Trash, never permanently, by default; stream large files; run fmt, clippy and tests before every commit;
  keep default runtime work offline, with only the approved opt-in manual currency exception.
- **Ask first:** adding a crate or npm dependency not listed above; any feature that needs network access, admin/root
  rights, or a background service; changing the license; CI changes that would cost money (public repo Actions are free).
- **Never:** decompile or copy proprietary code, assets, names or text; use "Parallels" in the product name, icons or
  branding (a neutral "alternative to" mention with a trademark disclaimer in the README is fine); commit secrets;
  ship code that deletes user files without confirmation; disable failing tests to go green.

## Success criteria and open gates

1. Native core/host fmt, clippy, tests and builds pass on all three targets with Rust 1.95; canonical legacy/site checks agree.
2. Native installers, clean-machine behavior and the original Windows installer target of under 15 MB are remeasured;
   historical Tauri bundles do not satisfy native packaging acceptance.
3. Tray/shortcut/single-instance/login behavior and the under-one-second startup target are measured for the native build.
4. All 49 native routes preserve each supported tool's controls, results, safety, provider and platform behavior;
   full Windows/manual/accessibility and macOS/Linux runtime checks remain recorded gates.
5. Production native startup/idle/tools/Quit and owned providers receive bounded DNS/TCP/UDP observation, with
   the manual ECB exception identified. Native rendering does not itself prove a universal zero-network guarantee.
6. Public source, contribution guidance, licenses, notices and issue templates remain available; release publication
   waits for the acceptance records and independent reviews.

## Decisions log

- 2026-11-21: Spec approved. Build the framework first (shell, launcher UI, two reference tools: Transform Text,
  Verify Checksum), push it, then build the remaining tools in phases per `tasks/plan.md`.
- 2026-11-21: Launcher navigation follows the familiar toolbox pattern (category sidebar, searchable tool grid,
  one view per tool), with Oxbox's own visual design.
- 2026-11-21: UI icons from Lucide (`@lucide/svelte`, ISC license), imported per icon.
- 2026-11-21: Repo is `ninad-k/oxbox` (public). v0.1 installers ship unsigned; README documents the OS warnings.

- 2026-11-21: User approved the native-renderer migration: eframe 0.37.0, Rust 1.95 and the approved host adapters. Preserve all 49 tools, legacy data and unpublished-release gates; metadata is shared through catalog/tools.json.

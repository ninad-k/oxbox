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
All code, icons and copy in Oxbox are original.

**Success looks like:** a user installs a <15 MB app, clicks the tray icon, picks a tool from a searchable grid,
and gets the result in one or two clicks, on all three OSes.

### User stories (v0.1)
1. As a user, I click the tray icon and see a searchable grid of tools grouped by category.
2. As a user, I can pin favourite tools and launch a tool with a global shortcut.
3. As a user, each tool works offline, does nothing destructive without confirmation, and sends no data anywhere.
4. As a contributor, I can add a new tool by adding one Rust module in `oxbox-core`, one UI view and one registry entry.

## Capability map (v0.1 = 20 tools + shell)

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
| `text` | **Transform Text** (UPPER, lower, Title, camelCase, snake_case, kebab-case…), **Unit Converter** (length, mass, temperature, data size, time; no currency in v0.1) | shell |

Build order: `shell` → `text`, `files` (pure logic, easiest to test) → `system`, `power`, `time` → `media`, `capture`, `clipboard`, `privacy`.

### Platform support (v0.1)
- **Windows 10/11:** every tool. This is the primary development target, verified on a real machine.
- **macOS 12+ and Linux (X11/Wayland, GNOME/KDE):** builds in CI. Where an OS API differs, the tool either has an
  implementation or is shown as "not available on this OS". It is never shown broken.
  Known gaps: Hide Desktop on Linux varies by desktop environment; window capture on Wayland is limited.

### Roadmap after v0.1 (subsequently promoted)
Window Manager / Window on Top, Presentation Mode, Do Not Disturb, Mute Microphone, Eject Volumes, Recognize Text (OCR),
Barcode Reader, Measure Color, Switch Resolution, Record Screen/Audio (ffmpeg), Make GIF, Convert Audio/Video,
Uninstall Apps, Launch (app sets), Energy Saver, Airplane Mode, Date Countdown, Alarm, Currency rates.
(Download Video/Audio is deliberately deferred: it raises site terms-of-service issues.)

On 2026-11-21 the user explicitly promoted the planned Phase 5 rows as well as Phases 1–4. Detailed promoted
task files are linked from `tasks/todo.md`; source now exposes 48 tools. The excluded/deferred tools remain excluded.
Network behavior remains default-offline, with only explicit opt-in ECB currency refresh allowed after the
user-authorized [network issue #1](https://github.com/ninad-k/oxbox/issues/1). No user amounts or files are uploaded.

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Language | Rust (stable, edition 2024, MSRV 1.93) | Speed, safety, tiny binaries, great OS crates; safe bounded 7z reader requires 1.93 |
| Desktop shell | **Tauri 2** (`tray-icon`, `global-shortcut`, `notification`, `dialog`, `autostart` plugins) | Native installers for all 3 OSes; small; Rust backend |
| UI | **Svelte 5 + TypeScript + Vite**, plain CSS with design tokens (light/dark) | Small bundle, little boilerplate for ~20 views |
| Core crates | `sysinfo`, `xcap`, `arboard`, `keepawake`, `image`, `qrcode`, `age`, `zip`, `tar` + `flate2`, `sevenz-rust`, `blake3`, `sha2`, `sha1`, `md-5`, `walkdir`, `trash`, `chrono` + `chrono-tz`, `serde`, `thiserror`, `tracing` | Mature and widely used; each one is pinned in Cargo.lock |
| Package manager | pnpm (UI), cargo (Rust) | Already installed |

Exact crate versions are pinned when they are added, checked against their docs (source-driven).

## Commands

```
pnpm install                         # UI deps (in repo root)
pnpm tauri dev                       # run app with hot reload
pnpm tauri build                     # release installers (.msi/.exe, .dmg, .deb/.AppImage)
cargo test --workspace               # all Rust unit + integration tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
pnpm check                           # svelte-check + TypeScript
pnpm test                            # vitest (UI logic)
```

## Project Structure

```
oxbox/
├─ crates/oxbox-core/        → Pure-Rust tool logic. No Tauri dependency. One module per tool.
│   ├─ src/tools/<tool>.rs   → e.g. checksum.rs, duplicates.rs, text_transform.rs
│   ├─ src/platform/         → OS-specific code behind cfg(target_os) (windows.rs, macos.rs, linux.rs)
│   └─ tests/                → Integration tests using temp dirs
├─ src-tauri/                → Tauri app: tray, window, thin #[tauri::command] wrappers over oxbox-core
├─ src/                      → Svelte UI
│   ├─ lib/registry.ts       → Tool registry (id, name, category, icon, platforms)
│   ├─ lib/tools/<Tool>.svelte
│   └─ lib/ui/               → Shared components (ToolCard, FilePicker, ResultPanel)
├─ docs/adr/                 → Architecture decision records
├─ .github/workflows/        → CI (test + build matrix win/mac/linux), release
├─ SPEC.md  README.md  CONTRIBUTING.md  LICENSE-MIT  LICENSE-APACHE  CODE_OF_CONDUCT.md  SECURITY.md
```

## Code Style

`cargo fmt` defaults; clippy clean with `-D warnings`. Errors are `thiserror` enums in core, mapped to a
serialisable `ToolError { code, message }` at the Tauri boundary. No `unwrap()` outside tests.
Long-running work runs on a blocking thread and reports progress through events.

```rust
// crates/oxbox-core/src/tools/checksum.rs
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algorithm { Sha256, Sha1, Md5, Blake3 }

/// Hash a file in streaming fashion; never loads the whole file into memory.
pub fn hash_file(path: &Path, algo: Algorithm) -> Result<String, CoreError> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Hasher::new(algo);
    std::io::copy(&mut reader, &mut hasher)?;
    Ok(hasher.finalize_hex())
}

// src-tauri/src/commands.rs: thin wrapper only
#[tauri::command]
async fn checksum(path: PathBuf, algo: Algorithm) -> Result<String, ToolError> {
    tauri::async_runtime::spawn_blocking(move || oxbox_core::checksum::hash_file(&path, algo))
        .await.map_err(ToolError::join)?.map_err(Into::into)
}
```

Naming: Rust `snake_case` modules, tool ids `kebab-case` (`verify-checksum`), Svelte components `PascalCase`.

## Testing Strategy

- **Unit tests (core, required):** every pure function: text transforms, unit conversions, checksum known vectors,
  duplicate grouping, archive round-trip, encrypt→decrypt round-trip, QR output decodes, image resize dimensions,
  clean-drive scanning (on a temp-dir fixture; never touches real user dirs in tests).
- **Integration tests (core):** `crates/oxbox-core/tests/` with `tempfile`.
- **OS-effect tools** (lock screen, keep awake, hide desktop, sleep timer, screenshot): logic is behind a small
  trait so command construction is unit-tested; the real effect is checked by hand on Windows and listed in the PR.
- **UI:** vitest for registry/search/format helpers; `svelte-check` for types; a manual smoke run of every tool on Windows.
- **CI:** GitHub Actions matrix (windows-latest, macos-latest, ubuntu-latest): fmt, clippy, tests, `tauri build`.
- Coverage goal: ≥80% lines in `oxbox-core` pure modules.

## Boundaries

- **Always:** keep tool logic in `oxbox-core`; confirm before deleting, overwriting or shutting down; delete to
  Recycle Bin/Trash, never permanently, by default; stream large files; run fmt, clippy and tests before every commit;
  keep the app offline (no telemetry, no network calls in v0.1).
- **Ask first:** adding a crate or npm dependency not listed above; any feature that needs network access, admin/root
  rights, or a background service; changing the license; CI changes that would cost money (public repo Actions are free).
- **Never:** decompile or copy proprietary code, assets, names or text; use "Parallels" in the product name, icons or
  branding (a neutral "alternative to" mention with a trademark disclaimer in the README is fine); commit secrets;
  ship code that deletes user files without confirmation; disable failing tests to go green.

## Success Criteria (v0.1)

1. `cargo test --workspace`, clippy (`-D warnings`), fmt check and `pnpm check` all pass locally and in CI on all 3 OSes.
2. `pnpm tauri build` produces a Windows installer under 15 MB, and CI produces macOS and Linux bundles.
3. The app starts to the tray in under 1 s. The launcher opens from the tray and from a global shortcut (default `Ctrl/Cmd+Alt+Space`).
4. All 20 tools work end-to-end on Windows 11 (checked by hand, recorded in the release notes); tools unsupported on an OS are shown as such.
5. No network traffic in the default runtime configuration. The promoted currency tool alone can fetch fixed public ECB
   data after explicit opt-in; no HTTP client dependency, telemetry, or user-content upload is introduced.
6. A public GitHub repo `ninad-k/oxbox` with README (features, screenshots, install, trademark disclaimer),
   CONTRIBUTING (how to add a tool), dual MIT/Apache-2.0 license, issue templates, and green CI.

## Decisions log

- 2026-11-21: Spec approved. Build the framework first (shell, launcher UI, two reference tools: Transform Text,
  Verify Checksum), push it, then build the remaining tools in phases per `tasks/plan.md`.
- 2026-11-21: Launcher navigation follows the familiar toolbox pattern (category sidebar, searchable tool grid,
  one view per tool), with Oxbox's own visual design.
- 2026-11-21: UI icons from Lucide (`@lucide/svelte`, ISC license), imported per icon.
- 2026-11-21: Repo is `ninad-k/oxbox` (public). v0.1 installers ship unsigned; README documents the OS warnings.

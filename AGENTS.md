# AGENTS.md: how to work on Oxbox

This file is for AI coding agents (Claude, Codex, local LLMs via Continue/Aider/Cline/OpenCode, …)
and for humans. Read it fully before changing code. Then read `SPEC.md` (what and why)
and `tasks/plan.md` (what to build next, in which order).

## 1. What Oxbox is

A tray/menu-bar desktop app with many small one-click utilities ("tools").
Rust does all the work; a Svelte UI shows a launcher (sidebar of categories + grid of tool tiles)
and one view per tool. Targets: Windows 10/11 (primary), macOS 12+, Linux.

## 2. Golden rules

1. **Logic lives in `crates/oxbox-core`.** No Tauri imports there. The Tauri crate (`src-tauri`) only adapts.
2. **One tool = one core module + one command (or a few) + one Svelte view + one registry entry.** Don't spread a tool across unrelated files.
3. **Never delete user data without confirmation.** Default to moving files to the Recycle Bin/Trash (`trash` crate), never permanent delete.
4. **Offline by default.** No telemetry, no network calls. Anything needing the network must be raised in an issue first.
5. **Never panic on user input.** Return `CoreError`. `unwrap()`/`expect()` are denied in `oxbox-core` outside tests.
6. **Stream big files.** Never `read_to_end` a user file of unknown size.
7. **Ask before adding dependencies** that are not already listed for your tool in `tasks/plan.md`.
8. **Clean-room.** Do not copy code, icons, names or text from proprietary products. Write original code and copy.
9. **Keep the UI consistent.** Use the tokens in `src/app.css` and the shared components in `src/lib/ui/`. No new colours or one-off spacing.
10. **Tests first for logic.** Write the failing test in the core module, then the implementation.

## 3. Repository map

```
crates/oxbox-core/src/
  error.rs            CoreError (+ stable .code()) and CoreResult
  platform/mod.rs     Os detection; put OS-specific code in platform/{windows,macos,linux}.rs
  tools/mod.rs        list of tool modules
  tools/<tool>.rs     tool logic + unit tests  (reference: text_transform.rs, checksum.rs)
src-tauri/src/
  lib.rs              app builder: plugins, tray, window behaviour, generate_handler![...]
  commands.rs         #[tauri::command] adapters (validate size, spawn_blocking, map errors)
  error.rs            ToolError { code, message } sent to the UI
  tray.rs             tray icon + menu
src-tauri/capabilities/default.json   Tauri permissions. Add only what a tool needs.
src/lib/
  registry.ts         THE tool catalogue (id, name, category, icon, platforms, phase, status, load)
  api.ts              typed wrappers for every Rust command. Views never call invoke() directly.
  search.ts           launcher filtering/ranking (+ search.test.ts)
  favorites.svelte.ts favourites store
  ui/                 shared components: Sidebar, ToolTile, ToolView, Notice, Segmented, CopyButton, PlannedTool
  tools/<Tool>.svelte one view per tool (reference: TransformText.svelte, VerifyChecksum.svelte)
src/routes/+page.svelte  launcher shell (search, grid, opening tools)
```

## 4. Commands

```bash
pnpm install                                         # once
pnpm tauri dev                                       # run the desktop app with hot reload
pnpm dev                                             # UI only in a browser (Rust commands unavailable)
cargo test --workspace                               # Rust tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all                                      # format Rust
pnpm check                                           # Svelte + TypeScript type check
pnpm test                                            # UI unit tests (vitest)
pnpm tauri build                                     # release installers
```

Windows tip: if the repo sits in a very deep folder, MSVC's linker fails with `LNK1104` (path too long).
Clone to a short path (e.g. `D:\src\oxbox`) or set `CARGO_TARGET_DIR=C:\t\oxbox`.

## 5. Recipe: add a tool

Example: building `unit-converter`. Follow every step; tick them off in `tasks/todo.md`.

**Step 1: Core logic + tests** in `crates/oxbox-core/src/tools/unit_convert.rs`, registered in `tools/mod.rs`.

```rust
//! Tool: **Unit Converter** (`unit-converter`).
use serde::{Deserialize, Serialize};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unit { Meter, Kilometer, Mile /* … */ }

pub fn convert(value: f64, from: Unit, to: Unit) -> CoreResult<f64> {
    if !value.is_finite() { return Err(CoreError::InvalidInput("value must be a number".into())); }
    // …
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn km_to_miles() { assert!((convert(1.0, Unit::Kilometer, Unit::Mile).unwrap() - 0.621371).abs() < 1e-6); }
}
```

Run `cargo test -p oxbox-core`; it must fail first, then pass.

**Step 2: Command** in `src-tauri/src/commands.rs`, then add it to `generate_handler![…]` in `lib.rs`.

```rust
#[tauri::command]
pub fn convert_unit(value: f64, from: unit_convert::Unit, to: unit_convert::Unit) -> Result<f64, ToolError> {
    Ok(unit_convert::convert(value, from, to)?)
}
// Disk/CPU heavy? make it `async` and wrap the call in `run_blocking(move || …).await`.
```

**Step 3: Typed wrapper** in `src/lib/api.ts`: mirror Rust enums as TS string unions and add a method to `api`.
Tauri converts argument names: Rust `snake_case` params ↔ JS `camelCase` keys.

**Step 4: View** `src/lib/tools/UnitConverter.svelte`. Start from `TransformText.svelte` (instant results)
or `VerifyChecksum.svelte` (file picking + long work). Rules:
- Use `.field`, `.field-label`, `.input`, `.btn`, `.btn-primary`, `.card` classes and `Segmented`, `CopyButton`, `Notice`.
- Every control has a visible label or `aria-label`. State is never shown by colour alone (use `Notice`).
- Handle the states: empty, working (`aria-busy`), success, error (`Notice kind="error"`).
- Guard against stale async results with a `requestId` counter (see both reference views).

**Step 5: Registry**: in `src/lib/registry.ts` set `status: 'ready'` and
`load: () => import('./tools/UnitConverter.svelte')`. Import icons per file:
`import Ruler from '@lucide/svelte/icons/ruler'` (never the barrel import; it slows builds).

**Step 6: Permissions**: if the view uses a Tauri plugin (dialog, notification, …), add the narrowest permission
to `src-tauri/capabilities/default.json`.

**Step 7: Verify** (all must pass): `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `pnpm check`, `pnpm test`, then `pnpm tauri dev` and use the tool by hand.

**Step 8: Docs**: tick the task in `tasks/todo.md`, update the tool's row in `README.md` to "Ready".

## 6. Patterns for harder tools

- **OS-specific behaviour** (lock screen, hide desktop, sleep): add `platform/windows.rs`, `macos.rs`, `linux.rs`
  with the same function signature behind `#[cfg(target_os = "…")]`; unsupported → `Err(CoreError::Unsupported(Os::current().display_name()))`.
  Keep command construction (e.g. which shell command or API call) in a pure function you can unit-test.
  Prefer crates (`keepawake`, `xcap`, `arboard`, `trash`) over shelling out; when shelling out, pass args as an
  array (`std::process::Command::arg`), never via a shell string.
- **Stateful / background tools** (Do Not Sleep, Clipboard History, Break Time): hold state in a struct managed
  by Tauri (`app.manage(Mutex<State>)`) and expose `start`/`stop`/`status` commands. Stop cleanly on app exit.
- **Progress for long jobs** (duplicates, clean drive, archive): pass a `&mut dyn FnMut(Progress)` callback from
  core; in the command, emit `app.emit("tool://<id>/progress", payload)`; listen with `listen()` in the view.
  Support cancel with an `Arc<AtomicBool>` checked in the loop → `CoreError::Cancelled`.
- **Destructive actions**: show a confirmation (`ask()` from `@tauri-apps/plugin-dialog`) listing what will happen,
  then move to trash.

## 7. Definition of done (per tool)

- [ ] Core logic has unit tests covering normal, edge and error cases; they pass.
- [ ] clippy (`-D warnings`), fmt, `pnpm check`, `pnpm test` are clean.
- [ ] The tool works by hand in `pnpm tauri dev` on Windows; macOS/Linux either work or show "not available".
- [ ] Empty, busy, success and error states are handled; keyboard-only use works.
- [ ] No new network access, no permanent deletion, no new deps outside the plan without approval.
- [ ] `tasks/todo.md` and `README.md` updated.

## 8. Git

Conventional Commits (`feat(unit-converter): …`, `fix(tray): …`, `docs: …`). One tool per branch/PR.
Never commit secrets, `target/`, `node_modules/` or `build/`.

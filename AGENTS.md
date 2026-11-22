# AGENTS.md: how to work on Oxbox

Read this file fully, then SPEC.md, tasks/plan.md and the relevant task specification before changing code.
The user selected native eframe 0.37.0 and Rust 1.95 on 2026-11-21; ADR 0004 records the decision.

## 1. What Oxbox is

A native tray/menu-bar desktop toolbox with 56 small utilities, an 800 × 800 favorites-first launcher,
search and category browsing. Rust core logic is shared across Windows 10/11 (primary), macOS and Linux.
The current app is crates/oxbox-native (eframe/egui with Glow), without an embedded browser. The original
Tauri/Svelte source remains a parity reference; its previous validation does not certify the new host.

## 2. Golden rules

1. **Logic lives in crates/oxbox-core.** No eframe, Tauri or other UI imports there. Native host code adapts.
2. **One tool = one core module + a concrete native form + one canonical catalogue entry.** Grouped native
   view modules are allowed; every ID must expose its real controls, results, errors and platform limits.
   Do not replace tools with generic JSON editors, mock results or placeholder routes.
3. **Never delete user data without confirmation.** Default to Recycle Bin/Trash through the shared helper,
   never permanent deletion except the explicitly user-authorized File Shred tool, which requires its own
   exact review, typed DELETE, limitation acknowledgement and confirmation. Outputs use exclusive creation.
4. **Offline by default.** No telemetry, automatic URL loading or background refresh. The user-approved
   manual ECB currency refresh is the documented opt-in exception (issue #1). New network use needs an issue
   and approval. A native renderer alone is not proof of zero runtime network activity.
5. **Never panic on user input.** Return CoreError; unwrap()/expect() remain denied in core outside tests.
6. **Stream big files.** Never read_to_end a user file of unknown size. Bound metadata, previews and queues.
7. **Ask before adding dependencies** outside the approved task/ADR set. The selected native packages and
   versions are already authorized; do not ask again for those. Never silently add another renderer/backend.
8. **Clean-room.** Write original code and copy; do not copy proprietary code, icons, names, text or UI art.
9. **Keep the native UI consistent.** Use shared egui styles, real labels/AccessKit names, native dialogs and
   existing Lucide vector assets. Do not add one-off colors, typography, remote fonts or image loaders.
10. **Tests first for new logic.** Observe the failing core/fake-backend test, then implement. Tests must not
    move the live pointer, capture private content, delete user data, change power/display settings or use the network.
11. **Preserve saved data.** Use bounded atomic host storage and transactions. Import legacy files into separate
    native-*.json stores once; leave originals intact. Surface invalid storage rather than silently resetting it.
12. **Own background work.** Hidden timers/reminders cannot depend on UI frames. Workers, OS leases and media
    children must cancel, join and restore cleanly on Quit. Do not register work after shutdown begins.

## 3. Repository map

- crates/oxbox-core/src/tools/<tool>.rs: toolkit-neutral logic and unit tests.
- crates/oxbox-core/src/platform/: OS-specific core adapters with explicit Unsupported errors.
- crates/oxbox-core/src/{launcher,preferences}.rs: shared catalogue/search and safe local persistence.
- crates/oxbox-native/src/app.rs: launcher, settings, focus, navigation and global UI lifecycle.
- crates/oxbox-native/src/{host,jobs,integration,instance}.rs: storage/notifications, cancellable tasks,
  tray/hotkeys/login and local single-instance activation.
- crates/oxbox-native/src/tools/{system,media,utilities,maintenance}.rs and submodules: concrete native forms.
- catalog/tools.json: the sole tool/category metadata authority for native, legacy and website interfaces.
- catalog/icons.json: flattened existing Lucide vectors; scripts/generate-native-icons.mjs regenerates them.
- src/, src-tauri/: retained legacy comparison host. Legacy registries contain only typed view/icon adapters.
- website/: static Pages source; no application commands or automatic external resources execute there.
- tasks/native-migration.md: route parity matrix and acceptance gates. docs/VALIDATION.md holds dated evidence.

## 4. Commands

Run from the repository root with Rust 1.95 and the native OS development packages installed.

    cargo run -p oxbox-native --locked
    cargo run -p oxbox-native --locked -- --profile <disposable-profile-directory>
    cargo test -p oxbox-core -p oxbox-native --locked
    cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
    cargo fmt --all -- --check
    node scripts/build-native.mjs --check
    node scripts/build-native.mjs
    node scripts/build-native.mjs --bundle

The build wrapper verifies the selected package graph, builds oxbox-native and reuses the installed bundler
only as a packaging tool. Do not substitute pnpm tauri build for a native release build.

Legacy/catalogue and website checks (Node 22.17+ and frozen pnpm dependencies for legacy UI):

    pnpm install --frozen-lockfile
    pnpm check
    pnpm test
    pnpm build
    node scripts/build-website.mjs
    node --test scripts/website.test.mjs

Use pnpm tauri dev only for an explicitly identified legacy comparison. On Windows, coordinate Rust builds/tests
across agents to avoid executable/link locks. Use a short target directory if MSVC paths become too long.

## 5. Recipe: add or change a tool

1. Read its specification and existing core/view reference. Add failing core tests for normal, edge and error
   cases, including cancellation and fake OS effects where relevant. Implement without UI dependencies.
2. Add the actual typed native form in the appropriate tools group. Call core APIs directly or through the
   host task/controller; never invoke a shell string or place business logic in widget callbacks.
3. Use Task for file/CPU jobs, progress and user Cancel. Call abandon() on navigation to discard stale results;
   hiding the window preserves in-flight work. Keep explicit background controllers alive until Stop/expiry/Quit.
4. Use Host load/save/save_at and transactions for app data. Persist alarm/date firing state before notifying;
   recover on storage failure. Passphrases stay transient, are never logged/persisted, and draft fields clear on leave/hide.
5. Update catalog/tools.json. Keep stable IDs, existing platform constraints and provider notes. Maintain the
   legacy component/icon adapters while that comparison host remains. Do not duplicate metadata in TypeScript.
6. Confirm destructive actions with exact reviewed paths/commands/counts. Use shared path guards and Trash.
   Unsupported controls must report real limits; OS settings delegation is acceptable only where specified.
7. Run scoped tests, then the full native checks and relevant legacy/site checks. Verify manually in a
   disposable profile on a consenting desktop; test keyboard, focus, empty/busy/success/error states and Quit.
8. Update task/docs evidence. A checked source row or fake backend is not native manual acceptance. Leave
   installer, hardware, accessibility, privacy and independent review gates open until recorded evidence exists.

## 6. Patterns and lifecycle

Keep command construction pure and testable; pass executable arguments separately, never through shell strings.
Use platform modules and cfg guards for unavailable APIs. Long work checks CancelToken in streaming loops and
reports Progress through the bounded native Task mechanism. Task shutdown cancels and joins its worker.

Native tools may use shared Arc/Mutex state, dedicated workers and cloned Notifier handles. Avoid locks across
UI callbacks or blocking I/O where possible. Explicit Stop must await cleanup. Queued starts/results must not
outlive Stop/navigation/Quit, and settings failures must restore the preceding runtime/storage state.

Tests with egui Context::run_ui are headless: consume/clear returned textures_delta because no GPU painter is
present. Such tests prove route/widget construction, not real GPU, screen-reader or hardware behavior.

## 7. Definition of done

- Core normal/edge/error and meaningful lifecycle tests pass; native fmt/clippy/tests/build are clean.
- Catalogue metadata, icon/view bindings and any website output agree; relevant UI/site checks pass.
- Supported native behavior is tested by hand; unavailable OS/provider behavior is clearly labeled.
- Keyboard-only operation, labels, focus and empty/busy/success/error states are reviewed.
- No unauthorized dependency/network use, permanent deletion, overwritten input or lingering worker exists.
- Task/README/validation records distinguish implemented source from remaining acceptance.
- The release remains an unpublished draft until all required release gates are satisfied.

## 8. Git

Use Conventional Commits and codex/ branches by default; prefer one focused tool/change per PR. Never commit
secrets, target/, node_modules/, generated build/ or website/dist/. Do not push, tag, publish or alter external
services unless the session authorizes it. The integrator owns shared release and CI changes during parallel work.

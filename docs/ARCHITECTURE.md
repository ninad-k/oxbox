# Contributor architecture guide

The selected application is **crates/oxbox-native**, using eframe/egui 0.37 with Glow on Rust 1.95. Its widgets,
previews, tray and workers do not embed a browser. crates/oxbox-core remains toolkit-neutral computation,
filesystem rules, platform behavior and scheduler models. The retained src/ and src-tauri/ trees are the
legacy Svelte/Tauri parity reference, not the native entrypoint. [ADR 0004](adr/0004-runtime-offline-boundary.md)
records the accepted migration and open release gates.

## Catalogue, launcher and assets

catalog/tools.json is the sole ordered 49-tool/category metadata source. The Rust catalogue embeds it;
legacy registries import it and bind typed component/icon adapters; the static website builder reads it
directly. Names, descriptions, keywords, platforms, phase/status and icon keys are not maintained as parallel
TypeScript literals. Canonical notes carry provider/validation caveats, with explicit website-only overrides.
[Catalogue maintenance](../catalog/README.md) describes the consistency checks.

The native app uses a favorites-only Dashboard, category browsing and search. Shared Rust launcher/preferences
models validate stable IDs and bindings. The default global key is CommandOrControl+Alt+O when no saved key
exists. Ctrl/Cmd+K searches, Alt+1–9 opens favorite slots and Ctrl/Cmd+Q quits; text-edit focus guards prevent
shortcuts from corrupting input. Saved keys remain until an explicit successful update.

The decorated, resizable viewport starts at 800 × 800 logical pixels. Existing licensed Lucide vectors are
flattened into catalog/icons.json for native drawing; there is no runtime SVG parser or icon download.
The original outlined O logo remains shared with the website. Renderer/GPU/scaling, keyboard and AccessKit
acceptance require native observations rather than inference from assets or enabled features.

## Typed tool path

1. Canonical metadata selects a stable ID and advertises its existing platform/provider constraints.
2. app.rs routes that ID to a real typed form in native tools/system, tools/media or tools/utilities.
3. Instant calculations call core directly. File/CPU work captures immutable inputs in a cancellable Task.
4. Stateful operations use owned native coordinators and existing core controllers; OS behavior remains in
   core platform adapters or the narrow host adapter. Missing capabilities return honest errors/availability.
5. Results, progress, confirmations, copies/exports and failure states are native controls. Generic JSON
   editors, mock completion and placeholder routes do not satisfy a tool port.

There is no native invoke()/JSON IPC layer. Legacy typed APIs remain available only for comparison and
catalogue regression tests. Toolkit-independent rules and tests still belong in core, not widget callbacks.

## Background ownership

| Owner | Lifetime and cleanup |
|---|---|
| Host/Notifier | Bounded notification delivery worker runs independently of UI frames; joins on Quit; errors are visible |
| Task | Owns worker and cancellation token; latest-only progress plus reliable bounded completion; joins before shutdown returns |
| System runtime | Managed native action boundary, sleep/display deadlines, leases/presentation and recovery; rejects late work after Quit |
| Core awake controller | Dedicated worker creates/drops thread-affine OS guard on the same thread |
| Mouse Activity | Explicit idle-by-default Windows worker; timing-only hooks/message pump and final input/desktop checks; Stop/expiry/error/Quit releases observers |
| Utilities time runtime | Dedicated 250 ms worker for monotonic timer, breaks and persisted alarms/dates; persist-before-notify with rollback; stop/join on Quit |
| Media shared services | Explicit recording child, paused-by-default clipboard history, frozen selectors and owned capture/preview tasks; clean up on leave/Stop/Quit |
| Native overlays | Owned selection/focus viewports with monitor/DPI geometry and native identity; close/cancel/error/Quit restore state |

UI rendering is not the clock for hidden reminders. Timer/stopwatch elapsed time derives from injected
monotonic timestamps; calendar reminders use explicit IANA zones and tested DST resolution. Navigation
abandons ordinary view jobs, discarding stale/enqueued payloads while retaining join ownership. User Cancel
retains the report. Hiding clears unsubmitted passphrase drafts and preserves active file work. Explicit
background controllers continue until their own Stop/expiry/Quit; manual currency import/refresh can finish
and save its cache after leaving the view.

## Persistence and migration

The default profile uses the legacy OS application-data location; --profile selects an isolated fixture
profile. Core preferences helpers bound and atomically save data. Invalid stores surface errors and preserve
originals; settings transactions restore preceding runtime/storage state when writes fail.

| Native store | Data imported/owned |
|---|---|
| native-shell.json | Theme, favorites, launcher key/login preferences; legacy settings preferences imported once |
| native-system.json | Local app sets and presentation options, including existing legacy stores |
| native-utilities.json | Alarms, date countdowns, world zones and break settings; legacy settings keys preserved |
| native-currency.json | Dated rate cache and native manual-refresh opt-in; legacy rates imported, initial native network flag off |

Legacy settings.json, app-sets.json and currency.json are not rewritten by migration. No passphrase,
clipboard history, recording-active state or Mouse Activity session is persisted by these stores. Core
crypto takes a transient zeroizing secret; draft password fields clear on navigation/hide. Alarms/date
notification state saves before notification delivery, with rollback/retry on persistence failure.

## File, process and network boundaries

File outputs use exclusive creation. Archive destinations are new folders and every entry is validated,
including traversal, links, duplicate names, metadata/decoder budgets and total output. Cleanup accepts
internal category IDs, rescans paths and enforces the confirmed file/byte budget. Protected roots and
symlink/reparse components go through shared guards. Cancellation may leave a separately created partial
extraction folder; originals remain intact. Selected cleanup/uninstall/power effects require review.

External programs receive executable-plus-argument arrays, never shell strings. FFmpeg/Tesseract remain
user-installed providers. Captures/devices and clipboard monitoring require explicit user actions. Native
controls do not request elevation or bypass protected desktop/security policies.

Default work is local. The fixed public ECB download is the approved manual opt-in exception (issue #1),
without amounts/files uploaded. The selected native graph removes embedded-browser hosts; the packaging
wrapper checks that graph and disables WebView installation/updater artifacts. A finite production observation
is still required and cannot prove universal zero communication by all drivers/OS/provider processes.
Historical WebView2 findings remain in [runtime privacy records](RUNTIME_PRIVACY.md).

## Static website and verification

The site is plain static HTML/CSS/JavaScript with no automatic external resources or app commands. Its
JSON builder validates canonical IDs, categories, icon bindings, platforms and note overrides, then renders
all 49 cards for no-JavaScript browsing. Category/platform/search filters are progressive enhancement.
Relative assets and fragment targets are checked; generated website/dist stays ignored.

Headless egui tests consume textures_delta without a GPU painter. Existing native tests cover 13 utility and
14 media form routes, selected lifecycle/settings/task/instance regressions and finite icon bindings. The 22
system forms are source-routed/compiled; do not claim all 49 were headless-rendered or manually operated.
Consult [the route matrix](../tasks/native-migration.md), [testing guide](TESTING.md), [website guide](../website/README.md)
and [AGENTS.md](../AGENTS.md). Publication is a separate gate from source builds, tests or website deployment.

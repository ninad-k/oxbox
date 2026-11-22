# Native renderer migration

Approved by the user on 2026-11-21: eframe **0.37.0**, Rust **1.95**, and the direct host packages in
[ADR 0004](../docs/adr/0004-runtime-offline-boundary.md). Preserve all 49 tools and each tool's existing
platform availability, streaming/data safety, settings, background behavior and cleanup. Keep the release draft.

## Shared implementation

- [x] Create `codex/native-renderer`, preserve the legacy source and raise the workspace minimum to 1.95.
- [x] Add the approved native host dependency set, explicitly selecting accesskit, clipboard, default_fonts,
  system_fonts, glow, wayland and x11; automatic links, inspection and remote loaders are disabled.
- [x] Preserve all 49 stable tool IDs in `catalog/tools.json`; reuse the existing ISC-licensed Lucide vectors.
- [x] Observe failing tests before implementing shared search/preferences/storage, clock, clipboard accessor
  and monitor selection geometry. Combined core run and later focused geometry evidence are local logs.
- [ ] Verify the native host: window, tray, global keys, single instance, dialogs, previews and hidden notifications.
- [ ] Verify every tool's native route and feature parity, including keyboard/accessibility and asynchronous states.
- [ ] Verify 1.95 and all-platform tests/clippy/builds, dependency audit and native-target notices.
- [ ] Verify production native startup/idle/tools/Quit network behavior without changing OS protections.
- [ ] Build and inspect native installers, test on clean machines, update the existing unpublished draft.
- [ ] Complete real hardware/power/input, monitor/DPI, accessibility and independent human security/license gates.

## Bounded Windows observations

These checked subsets record finite evidence, not completion of a full tool row or the release gates above.
The controlled production Mouse Activity run used build **A**, executable SHA-256
`6fb64ef38e8a5725c72ef614cf78fc304775d8ed8310606ac72f45a447ba91b5`.

- [x] Actual native Windows screenshots are available for the [dark dashboard](../docs/images/native-dashboard-dark.jpg),
  [light dashboard](../docs/images/native-dashboard-light.jpg), [Unit Converter](../docs/images/native-unit-converter.jpg)
  and [stopped Mouse Activity controls](../docs/images/native-mouse-activity.jpg).
- [x] Build A Mouse Activity: idle threshold 30 seconds, interval 30 seconds; after 71 seconds the GUI
  reported Simulating and two pointer movements. Controlled Tab input reported Paused with idle 0 seconds;
  Stop reported Stopped. [Exact scope and remaining checks](../docs/MOUSE_ACTIVITY.md#contributor-checks).
- [x] A separate bounded **debug** Windows owner-PID TCP/UDP observation is documented with its sampling
  gaps and exclusions in [the privacy record](../docs/RUNTIME_PRIVACY.md#native-migration--2026-11-21).
- [x] Functional source `7132daa` passed Windows/macOS ARM/Linux clippy, tests and native production builds,
  plus the Rust 1.95 check and legacy UI/packaging regressions in [CI](https://github.com/ninad-k/oxbox/actions/runs/44941250679).
- [x] Final Windows MSI was extracted/hash-matched; NSIS source/compiler output was inspected. Both native
  installers and checksums were uploaded to the existing draft and downloaded/hash-verified. Actual install/uninstall remains open.
- [x] Separate build A production endpoint windows cover tools and a complete quiet-tray/activation/Quit loop,
  with no owned TCP/UDP endpoints observed. [Scope and limits](../docs/RUNTIME_PRIVACY.md#bounded-windows-production-observations).
- [x] Exact final build `7132daa` executable completed a separate 360.154-second/7,201-sample normal
  startup/idle/Unit Converter/Quit observation, with zero owned processes at the end and no owned
  TCP rows or UDP bindings observed. [Exact hash and limits](../docs/RUNTIME_PRIVACY.md#exact-final-executable-lifecycle).
- [ ] Complete physical keyboard/mouse, held input, duration, lock/protected-desktop, monitor/DPI and every
  Stop/Quit/restart path on the applicable final production build.
- [ ] Complete current production/per-tool/provider network coverage, DNS/OS-broker attribution and other-OS
  acceptance. The build A GUI run does not establish results for a later executable or a network probe.

## Maintenance and productivity expansion — 2026-11-22

The current catalogue contains **56** tools in eight categories. The 49-row baseline below remains
historical migration scope; `catalog/v01-tool-ids.json` makes preservation executable. New source and
headless/fake-backend tests do not close native manual acceptance. See the
[scope](maintenance-and-productivity.md) and [user/recovery guide](../docs/MAINTENANCE.md).

| Stable tool ID | Native form | Core coverage | Manual acceptance |
|---|---|---|---|
| `batch-rename` | `maintenance/files.rs` | Exact plans, no-replace moves, write-ahead/recovery, guarded Undo | Pending; use disposable fixtures |
| `file-shred` | `maintenance/files.rs` | Typed acknowledgement, identity/hard-link guards, bounded writes, partial cancellation; fake effects | Pending; never use personal data |
| `startup-manager` | `maintenance/system.rs` | Per-OS current-user scope, exclusive backup, immutable recovery review, stale entry refusal; fake effects | Pending on each OS |
| `storage-analyzer` | `maintenance/system.rs` | Generated trees, limits, categories and cancellation; read-only | Pending on each OS |
| `registry-review` | `maintenance/system.rs` | Narrow HKCU App Paths eligibility, backup, stale review/restore; fake registry | Pending on Windows; unavailable elsewhere |
| `browser-review` | `maintenance/browser.rs` | Generated extension metadata, bounded traversal, cancellation; no private browser state | Pending on each supported browser/OS |
| `maintenance-review` | `maintenance.rs` | Named idle routing and existing cleanup models; no automatic mutation | Pending on each OS |

Existing tools also gain screenshot annotations, explicit text cleanup, safe resize/checksum/timer/unit
presets, Quick Actions and new per-OS application review. Their tests cover generated data and native
control/state models; they do not establish device, actual deletion, OS mutation or screen-reader passes.

## Original parallel ownership

- System/window/app group: 22 tools in `crates/oxbox-native/src/tools/system/`.
- Media/capture/vision/clipboard group: 14 tools in `crates/oxbox-native/src/tools/media/`.
- Files/text/conversion/time group: 13 tools in `crates/oxbox-native/src/tools/utilities/`.
- Integrator: shared host/jobs/persistence/catalogue, native shell, lifecycle, packaging, CI and documentation.

Toolkit-neutral logic stays in `oxbox-core`; UI/OS host adapters stay in `oxbox-native`. Existing files in the
legacy profile are imported once into separate `native-*.json` stores and left intact. Profiles that cannot
be parsed must display errors rather than silently overwrite user data. New standalone background workers
must stop and join on Quit, and must not depend on whether a tool's view is visible.

The previous Tauri draft is historical evidence, not native migration acceptance. No installer is published
and no hardware, destructive, power or pointer-moving test is inferred from compilation or fake backends.

## Tool-by-tool native parity matrix

This matrix covers **49 source-routed forms**, not 49 native manual passes. The integrator observed a live
native decorated launcher and an empty favorites dashboard. That shell feasibility observation does not
exercise each tool, device, OS effect, confirmation, GPU path or screen-reader interaction.

- **Core models/fixtures**: the tool's existing core unit/fake-backend/fixture coverage. This does not imply
  real device, destructive, power, clipboard, pointer, camera or network effects ran during tests.
- **H13**: one headless utilities test renders all 13 default routes and checks control names, verifies legacy
  settings/cache bytes are preserved and imported currency networking starts off. It does not click tool actions.
- **H14**: one headless media test renders all 14 default routes and checks control names, without devices or clipboard monitoring.
- **H22**: one headless system test renders all 22 default routes and checks control names with task admission
  closed before rendering. No OS-reading/changing job runs. Separate lifecycle/configuration regressions remain;
  none of these checks prove every expanded state, device or native manual behavior.
- **Pending**: complete per-tool native manual acceptance is still open here, including rows with bounded
  observations above. Update a row only with dated revision/platform/scaling/provider evidence in
  docs/VALIDATION.md; legacy results do not transfer.

| Stable tool ID | Native routing module | Source | Automated scope | Native manual |
|---|---|---|---|---|
| `transform-text` | [utilities/text.rs](../crates/oxbox-native/src/tools/utilities/text.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `clipboard-history` | [media/clipboard.rs](../crates/oxbox-native/src/tools/media/clipboard.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `unit-converter` | [utilities/text.rs](../crates/oxbox-native/src/tools/utilities/text.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `break-time` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `timer-stopwatch` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `world-time` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `verify-checksum` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `find-duplicates` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `archive` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `clean-drive` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `screenshot` | [media/capture.rs](../crates/oxbox-native/src/tools/media/capture.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `resize-images` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `qr-code` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `system-monitor` | [system/monitor.rs](../crates/oxbox-native/src/tools/system/monitor.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `network-usage` | [system/monitor.rs](../crates/oxbox-native/src/tools/system/monitor.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `do-not-sleep` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `sleep-timer` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `lock-screen` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `hide-desktop` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `encrypt-files` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `mouse-activity` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `window-on-top` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `window-manager` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `show-desktop` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `close-apps` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `focus-window` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `switch-resolution` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `eject-volumes` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `energy-saver` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `hidden-files` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `cpu-temperature` | [system/monitor.rs](../crates/oxbox-native/src/tools/system/monitor.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `launch-app-sets` | [system/apps.rs](../crates/oxbox-native/src/tools/system/apps.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `uninstall-apps` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `presentation-mode` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `do-not-disturb` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `airplane-mode` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; native CI builds | Core models/fixtures; H22 | Pending |
| `alarm` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `date-countdown` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |
| `mute-microphone` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `record-audio` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `record-screen` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `make-gif` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `convert-audio-video` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `take-photo` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `take-video` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `recognize-text` | [media/vision.rs](../crates/oxbox-native/src/tools/media/vision.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `barcode-reader` | [media/vision.rs](../crates/oxbox-native/src/tools/media/vision.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `measure-color` | [media/vision.rs](../crates/oxbox-native/src/tools/media/vision.rs) | Implemented; native CI builds | Core models/fixtures; H14 | Pending |
| `currency-converter` | [utilities/currency.rs](../crates/oxbox-native/src/tools/utilities/currency.rs) | Implemented; native CI builds | Core models/fixtures; H13 | Pending |

Every row retains the platform list and provider caveat in catalog/tools.json. Unsupported targets must
show the existing limitation, not a mock success. Recheck copies/exports, stale async results, confirmation,
Cancel/Stop, hide/navigation/restart and Quit behavior where relevant before marking a manual row complete.

## Final review verification

The final review source `d8d1d1e` passed [all-platform native CI](https://github.com/ninad-k/oxbox/actions/runs/44982236463).
Windows MSI/NSIS were rebuilt, inspected, uploaded to the existing draft and downloaded/hash-verified.
Separate final-Windows observations cover generated provider completion and Quit during an inert provider job.
[Exact evidence and remaining gates](../docs/VALIDATION.md#final-review-candidate-d8d1d1e). Every full manual row above remains pending.

## Installed startup default

The user additionally requested startup enabled by default after installation. Packaged Windows/macOS
resources include `oxbox-installed.json`; a fresh first launch configures current-user login startup.
Existing choices, legacy profiles, OS startup records and explicit opt-outs are preserved. Debug builds
and isolated profiles cannot mutate login startup; the portable Linux archive omits the marker.
Default login hides only when the tray is available. Startup never resumes input simulation or media/history.
Core policy/transaction/quoted-path/ownership tests use fakes; actual clean-machine install/login/uninstall
acceptance remains open.

## Community functional follow-up

The [49-tool functional review](../docs/FEATURE_REVIEW.md) traces all native routes and records reproduced
fixes with meaningful generated/fake regressions. The Windows integrated follow-up passed 221 core and 80
native tests before the final cleanup-status regression. Preceding source `a7c91bd` passes all-platform CI:
Windows 225 core / 81 native, macOS ARM 219 / 82 and Linux 218 / 82, with zero failed/ignored. Its Windows
draft assets were rebuilt/download-verified and its exact EXE passed a bounded IANA/EAN-13/Quit observation.
[Dated validation](../docs/VALIDATION.md) records the source, hashes, CI and limits. Contributor onboarding, support, catalogue and maintainer guides
are linked from the [documentation hub](../docs/README.md). Every full manual row remains pending until its
actual native actions and supported platform/provider behavior have been recorded.

## Second bounded functional verification

Source [`67f0715452fb601394f45d3296675d3b85df3fb5`](https://github.com/ninad-k/oxbox/commit/67f0715452fb601394f45d3296675d3b85df3fb5)
passed all five jobs in [CI run 44994827768](https://github.com/ninad-k/oxbox/actions/runs/44994827768):
Windows **226 core / 94 native**, macOS ARM **220 / 95**, and Linux **219 / 95**, with zero failed/ignored
tests. All three targets passed strict Clippy and native production builds, including the dependency-free
Windows activation-manifest build script; the Rust 1.95 locked check, 39 legacy UI tests and 18 packaging
regressions also passed. The separate local combined-host test passed 94 native and 12 legacy Rust tests.

The [dated second verification record](../docs/verification/2026-11-22-native-feature-pass.md) traces all
49 routes and scopes the generated/fake/headless regressions for changed inputs, stale outcomes, restore
navigation, settings admission and owned desktop restoration. Its exact new Windows EXE opened all 49
default native forms and passed Timer/QR state and generated-provider Cancel/hide/restoration/Quit checks.
The two separate bounded endpoint windows, static MSI/NSIS checks and all six downloaded asset hash
matches are recorded in dated validation. The uploaded draft now targets `67f0715`; earlier `a7c91bd`
observations remain scoped to the earlier executable.
Every full manual row, hardware/provider/other-OS runtime check, screen-reader check and clean-machine
install/login/upgrade/uninstall gate remains pending. The release stays unpublished.

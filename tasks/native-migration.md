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

## Parallel ownership

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
- **H13**: one headless utilities test renders all 13 utility routes, verifies legacy settings/cache bytes are
  preserved, and verifies imported currency networking starts off. It does not click tool actions.
- **H14**: one headless media test renders all 14 media routes without devices or clipboard monitoring.
- The 22 system routes have compiled source and separate native lifecycle/configuration regressions;
  no headless all-system-form rendering test is claimed. The native suite also covers tasks, settings,
  instance handling and bundled icon bindings. Counts/results belong to the integrator's dated records.
- **Pending**: actual per-tool native interaction/manual acceptance is still open here. Update a row only
  with dated revision/platform/scaling/provider evidence in docs/VALIDATION.md; legacy results do not transfer.

| Stable tool ID | Native routing module | Source | Automated scope | Native manual |
|---|---|---|---|---|
| `transform-text` | [utilities/text.rs](../crates/oxbox-native/src/tools/utilities/text.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `clipboard-history` | [media/clipboard.rs](../crates/oxbox-native/src/tools/media/clipboard.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `unit-converter` | [utilities/text.rs](../crates/oxbox-native/src/tools/utilities/text.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `break-time` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `timer-stopwatch` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `world-time` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `verify-checksum` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `find-duplicates` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `archive` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `clean-drive` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `screenshot` | [media/capture.rs](../crates/oxbox-native/src/tools/media/capture.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `resize-images` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `qr-code` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `system-monitor` | [system/monitor.rs](../crates/oxbox-native/src/tools/system/monitor.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `network-usage` | [system/monitor.rs](../crates/oxbox-native/src/tools/system/monitor.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `do-not-sleep` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `sleep-timer` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `lock-screen` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `hide-desktop` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `encrypt-files` | [utilities/files.rs](../crates/oxbox-native/src/tools/utilities/files.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `mouse-activity` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `window-on-top` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `window-manager` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `show-desktop` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `close-apps` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `focus-window` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `switch-resolution` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `eject-volumes` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `energy-saver` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `hidden-files` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `cpu-temperature` | [system/monitor.rs](../crates/oxbox-native/src/tools/system/monitor.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `launch-app-sets` | [system/apps.rs](../crates/oxbox-native/src/tools/system/apps.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `uninstall-apps` | [system/windows.rs](../crates/oxbox-native/src/tools/system/windows.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `presentation-mode` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `do-not-disturb` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `airplane-mode` | [system/power.rs](../crates/oxbox-native/src/tools/system/power.rs) | Implemented; debug compiles | Core models/fixtures; native form not headless-tested | Pending |
| `alarm` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `date-countdown` | [utilities/time.rs](../crates/oxbox-native/src/tools/utilities/time.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |
| `mute-microphone` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `record-audio` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `record-screen` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `make-gif` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `convert-audio-video` | [media/images.rs](../crates/oxbox-native/src/tools/media/images.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `take-photo` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `take-video` | [media/recording.rs](../crates/oxbox-native/src/tools/media/recording.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `recognize-text` | [media/vision.rs](../crates/oxbox-native/src/tools/media/vision.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `barcode-reader` | [media/vision.rs](../crates/oxbox-native/src/tools/media/vision.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `measure-color` | [media/vision.rs](../crates/oxbox-native/src/tools/media/vision.rs) | Implemented; debug compiles | Core models/fixtures; H14 | Pending |
| `currency-converter` | [utilities/currency.rs](../crates/oxbox-native/src/tools/utilities/currency.rs) | Implemented; debug compiles | Core models/fixtures; H13 | Pending |

Every row retains the platform list and provider caveat in catalog/tools.json. Unsupported targets must
show the existing limitation, not a mock success. Recheck copies/exports, stale async results, confirmation,
Cancel/Stop, hide/navigation/restart and Quit behavior where relevant before marking a manual row complete.

## Installed startup default

The user additionally requested startup enabled by default after installation. Packaged Windows/macOS
resources include `oxbox-installed.json`; a fresh first launch configures current-user login startup.
Existing choices, legacy profiles, OS startup records and explicit opt-outs are preserved. Debug builds
and isolated profiles cannot mutate login startup; the portable Linux archive omits the marker.
Default login hides only when the tray is available. Startup never resumes input simulation or media/history.
Core policy/transaction/quoted-path/ownership tests use fakes; actual clean-machine install/login/uninstall
acceptance remains open.

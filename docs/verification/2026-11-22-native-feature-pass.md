# Native feature verification — 2026-11-22

This second bounded pass traced all **49 native tool forms** to their core operations: 13 utilities,
14 media tools and 22 system tools. It checked inputs, results, export/copy controls, invalid input,
changed parameters, cancellation, navigation, hidden work and Stop/Quit ownership. Reproduced defects
were fixed with generated data, fake effects and headless controls. This is not a claim that all 49
features passed end-to-end on a real desktop.

The [feature catalogue](../FEATURES.md) describes capabilities and platform limits. The
[migration matrix](../../tasks/native-migration.md) tracks native acceptance; the
[earlier feature review](../FEATURE_REVIEW.md) records preceding fixes. Proposed safe GUI cases from
the parallel review notes are **not performed checks**. No camera, microphone, private screen capture,
clipboard collection, real Trash, power/display/radio change, uninstaller or network refresh was used
for the new regression tests.

## Validation receipt

| Item | Recorded scope / status |
|---|---|
| Reviewed source | [`67f0715452fb601394f45d3296675d3b85df3fb5`](https://github.com/ninad-k/oxbox/commit/67f0715452fb601394f45d3296675d3b85df3fb5), following `1539212` / `a7c91bd`. |
| Native control suite | **94/94 passed**, including all 13 new native regressions and the Timer follow-up. The corrected combined legacy/native test also passed 94 native and 12 legacy tests. Headless tests do not use a GPU or certify screen-reader interaction. |
| Core suite | **226 passed, zero failed/ignored** in `target/verification2-workspace-tests.log`, including the real age round trip and new desktop-identity guard. The later combined-host check excluded only this already-passed core harness. |
| Formatting / strict Clippy | Rust **1.95.0**, `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` passed. Final Clippy log: `target/verification2-clippy-final.log`. |
| All-platform CI | Source `67f0715` passed [all five jobs in run 44994827768](https://github.com/ninad-k/oxbox/actions/runs/44994827768): Windows **226 core / 94 native**, macOS ARM **220 / 95**, Linux **219 / 95**, zero failed/ignored. The UI suite passed **39 tests** and packaging checks **18**. Native production builds and strict Clippy passed on all three systems. CI build/test success is not other-OS runtime acceptance. |
| Installed-provider check | Actual Windows FFmpeg synthetic conversion/GIF/overwrite-refusal smoke passed; exact scope below. |
| Current native GUI / runtime | The exact `6606f3…412069` Windows EXE opened **49/49 unique canonical default forms**, using Ctrl+K search and accessible field/tile controls, with matching headings, Back controls and named operations. Custom Timer `10` immediately displayed `00:10:00`; generated QR text editing removed the old preview and Save/Copy controls. A generated 120-second provider verified explicit Cancel, preserved active work after hide/restoration, and app/child cleanup on Quit. Separate endpoint windows and their limits are recorded in [runtime privacy](../RUNTIME_PRIVACY.md). This is not full effect, screen-reader or keyboard-only acceptance. |
| Release / clean machine | Release ID `453727915` targets `67f0715`, remains draft/prerelease and has six refreshed assets downloaded and hash-verified. No actual version tag exists. Clean-machine install, upgrade, login opt-out preservation and uninstall remain pending. |

Final source/build/CI and artifact mapping belongs in [VALIDATION.md](../VALIDATION.md). Do not transfer
the earlier production binary's measurements to a newly built executable.

## Reproduced findings

The integrator observed failing tests before the scoped implementations changed. The QR/OCR popup
fixtures initially failed to locate their options; those targeting failures were corrected before
behavioral REDs were accepted. Each final media fixture asserts that the actual control value changed
before checking stale state.

| Problem and resulting behavior | Regression / filter | Evidence boundary |
|---|---|---|
| A disabled file form still handled raw OS drops, allowing an active job's displayed inputs to change. Ignore drops while the input UI is disabled; retain normal idle drops. | `busy_drop_cannot_relabel_an_active_file_job_with_different_inputs`; `idle_drop_still_accepts_a_new_checksum_input_and_clears_old_digest` | Generated `DroppedFileHandle` events and cancellable fake jobs cover checksum, duplicate roots, archive create/extract and encryption inputs. No private file or OS drop was required. |
| Editing an Archive draft retained a preceding completion/error. Bind outcome invalidation to operation, sources, archive, destination, format and budget; preserve a new Start failure. | `archive_changed_input_does_not_keep_an_old_completion_or_error` | Headless generated drop and injected outcomes; no live cleanup. |
| Custom Timer duration changed the draft but left its idle clock preview unchanged until Start. Update an editable idle/completed preview and clear its old completion notice while preserving running/paused time. | `idle_custom_duration_updates_the_timer_preview_before_start`; `custom_duration_does_not_reset_a_running_or_paused_timer` | Real AccessKit numeric edit and fake timer state; no delivered notification. |
| Restoring a hidden or unfocused window reset the active tool and abandoned its work. Restore the existing surface; reserve navigation and cancellation for an explicit search request. | `restoring_a_tool_or_settings_does_not_navigate_or_cancel_its_work`; `explicit_search_navigation_still_leaves_the_previous_surface` | Native navigation-policy tests; actual hidden-job restoration must be recorded separately against the current executable. |
| Editing QR content/type left a previous code exportable and could publish a queued old preview. Invalidate code, pixels, pending generation and old save/copy receipts when text/type/correction/size changes. | `verification2_qr_text_edit_discards_exportable_old_code_and_queued_generation`; `verification2_qr_type_edit_discards_exportable_old_code_and_queued_generation` | Actual multiline TextInput/ComboBox edits, generated QR bytes and injected completions; no clipboard or save dialog. |
| Editing OCR language/engine retained recognition for the previous settings. Discard text and queued recognition while retaining the unchanged input image preview. | `verification2_ocr_language_edit_discards_old_text_and_queued_recognition`; `verification2_ocr_engine_edit_discards_old_text_and_queued_recognition` | Actual TextInput/ComboBox edits and generated result/pixels; no OCR engine/device invoked. All four media regressions passed after meaningful REDs. |
| A settings delegation queued before navigation/Quit could admit a late OS-settings launch. Carry the original permit into the worker and check cancellation under its gate. | `settings_open_captured_before_navigation_or_quit_never_calls_backend` | Missing-helper RED, then fake stale/fresh/cancel admission checks; no settings window opened. Applies to Do Not Disturb, Airplane Mode and Presentation Mode's notification-settings action. |
| Hidden Files could report success when the backend read back the opposite flag. Return a visible error on refused readback. | `hidden_file_change_reports_refused_readback_instead_of_success` | Fake requested/actual/refused state; no Explorer setting changed. |
| Desktop restoration needed to reject a stale/reused native handle. Revalidate the saved window identity and current desktop membership before mutating it. | `restore_validates_desktop_window_identity_before_mutating_a_handle` | Missing-policy RED, then fake invalid/reused/foreign identity checks. Win32 validation and mutation remain separate calls; this is not an atomic guarantee against external destruction/reuse. |
| Testing the retained legacy and native hosts together enabled rfd's `common-controls-v6` feature, but the native test executable had no activation manifest and failed before its harness started. Embed the native package's original Common Controls v6 manifest in app and test executables. | Actual workspace loader failure, followed by the combined-host test and release resource inspection. | Dependency-free build script, MSVC-only linker flags, `asInvoker` and no OS configuration change; final results are recorded in the validation receipt. |

Tests live beside the [native utilities](../../crates/oxbox-native/src/tools/utilities/),
[media](../../crates/oxbox-native/src/tools/media/), [system forms](../../crates/oxbox-native/src/tools/system/)
and [core tools](../../crates/oxbox-core/src/tools/). Useful focused filters are
`tools::utilities::files::input_state_tests`, `tools::utilities::time::timer_input_tests` and
`verification2_`. Final validation also runs the full suites, not just the filters.

### Windows workspace loader check

The first full workspace attempt passed all **226 core tests**, then the native harness exited with
`0xc0000139` before running tests. A native-only build passed **94 tests**. Import comparison isolated
`comctl32.dll!TaskDialogIndirect` in the combined build: the retained legacy dialog dependency enables
rfd's Common Controls v6 feature through Cargo feature unification. The native package now carries its
own [Windows manifest](../../crates/oxbox-native/windows.manifest) and
[build script](../../crates/oxbox-native/build.rs), which apply to its app and test targets.

The Common Controls assembly requires activation through an application manifest; Microsoft's
[side-by-side controls guidance](https://learn.microsoft.com/en-us/windows/win32/controls/cookbook-overview)
describes that requirement. The MSVC
[embedded manifest option](https://learn.microsoft.com/en-us/cpp/build/reference/manifest-create-side-by-side-assembly-manifest?view=msvc-170)
and [manifest input option](https://learn.microsoft.com/en-us/cpp/build/reference/manifestinput-specify-manifest-input?view=msvc-170)
merge the manifest independently of the wrapper's icon/version resource. No DLL was installed or replaced.

## Per-tool evidence

“Reviewed” means the concrete form/state/core path was inspected. “Core/control” means pure logic,
generated files, fake backends or headless input events. Existing tests were cross-checked, not
independently rerun by every reviewer. Unless stated otherwise, the native/effect check in the last
column remains unperformed in this pass. Default-route/name coverage is not expanded-state or
hardware acceptance.

### Utilities — 13 routes

| Tool ID | Reviewed core/control evidence | Native/effect acceptance still needed |
|---|---|---|
| `transform-text` | Nine cases, Unicode, input/case updates, empty hint and Copy enablement. | Actual keyboard, result copy and expanded error/focus cases. |
| `unit-converter` | Category resets, finite/range/temperature validation, instant conversion and Swap; existing round trips. | Full category controls/copy on the current binary; earlier bounded Unit GUI evidence remains revision-specific. |
| `currency-converter` | Default-off networking, fixed-provider refresh, offline import, dated/stale state, cancellable owned runner and commit-before-book replacement. | Generated offline import/restart in native UI; approved opt-in real-provider/runtime checks separately. No ECB request in this pass. |
| `break-time` | Pause/snooze/skip scheduling, persisted settings, independent hidden worker, Stop and joined Quit. | Real idle/fullscreen detection, hidden delivery and notification permissions. |
| `timer-stopwatch` | Presets/custom, exact Space/R shortcuts, pause/resume/reset/laps and monotonic worker; new idle-preview regression. | Native start/pause/reset/laps, hidden completion and notifications. |
| `world-time` | Normalized IANA search, offsets/day-night, add/remove/reorder and save rollback. | Current-binary picker, reorder and restart persistence; prior exact-IANA GUI observation is separately dated. |
| `alarm` | Recurrence/zones/folds, snooze/dismiss, reviewed identity-safe removal and persist-before-notify. | Due/hidden/restart notification delivery and actual permission behavior. |
| `date-countdown` | Signed future/past values, notify-once/past-date rules, restoration and reviewed removal. | Native dated editing, restart and notification behavior. |
| `verify-checksum` | Four streamed hashes, expected-digest comparison, progress/cancel and new busy-drop guard. | Generated-file native pick/drop, digest output and cancellation controls. |
| `find-duplicates` | Size/prefix/full-hash grouping, hidden/overlap rules, selection/kept-copy protection and pre-Trash revalidation; busy-drop guard. | Generated-tree native selection/review; consenting real Trash test separately. |
| `archive` | ZIP/TAR.GZ create, magic-based ZIP/TAR/gzip/7z extract, budgets/exclusive output/cancel; new busy-drop and draft-outcome guards. | Native generated-file round trips, expanded options, conflict/error/cancel and output inspection. |
| `clean-drive` | Fixed allow-list, old-temp rule, scan budgets/category review and fake-root cancellation; no automatic cleanup. | Consenting disposable-account scan/review/Trash, with exact allow-listed paths checked. |
| `encrypt-files` | Streaming age/KDF bounds, exclusive output, passphrase/undo clearing, file-bound receipt, reviewed optional Trash and durable cleanup status; busy-drop guard. | Native generated encrypt/decrypt/wrong-passphrase/cancel; real optional Trash only on a consenting test machine. |

### Media, capture and vision — 14 routes

| Tool ID | Reviewed core/control evidence | Native/effect acceptance still needed |
|---|---|---|
| `screenshot` | Explicit target/delay/output/copy controls; area geometry, HiDPI mapping, filename/cancel fixtures and leave/Quit ownership. | Real display/window/area pixels, negative origins, mixed DPI, Escape, folder and clipboard. |
| `clipboard-history` | Paused default/consent, ordered Start/Pause/Clear, memory/pin budgets, search invalidation and view/service lifetime. | Consenting text/image/dedupe/pin/copy/delete/clear and concealed-format behavior; no live contents collected here. |
| `resize-images` | Percentage/fit/exact, PNG/JPEG/WebP/JPEG quality, EXIF, output preservation and batch cancel; generated fixtures. | Native generated batch, picker/preview, output dimensions/formats and cancellation. |
| `qr-code` | QR/Code128/EAN12/13 validation and decode, exclusive PNG/SVG save; new real-control stale-export regressions. | Expanded type/correction controls, PNG/SVG export bytes and explicit copy; current generated preview/invalidation is not full export acceptance. |
| `mute-microphone` | Explicit Refresh/change/confirmed Unmute wiring and Linux parser; macOS unavailable note. | Actual endpoint/default source, external changes and backend refusal. |
| `record-audio` | Literal local device, bounded WAV plan, consent/confirmation, generation-gated Start and shared Stop/recovery. | Consenting microphone selection, real WAV/duration/Stop and Quit. |
| `record-screen` | Desktop/physical-area/Windows-window plans, bounds/FPS, no audio and independent Stop; platform limits visible. | Real capture/playback/DPI/negative origins and cleanup; macOS/Wayland and Linux window provider limits remain. |
| `make-gif` | Validated segment/filter plan, local input/output guards, cancellable task; real synthetic FFmpeg smoke below. | Native file workflow, animation playback, boundary options and active cancellation with the installed build. |
| `convert-audio-video` | Five fixed presets, manifest/protocol guards, exclusive publishing and owned process cleanup; real WebM smoke below. | Native inputs/outputs, every preset's streams/playback and provider codec availability. |
| `take-photo` | Literal local camera, single-frame/no-audio plan, consent, cancellable capture and path/preview result. | Consenting actual camera selection/frame/permissions and cancellation; no camera opened here. |
| `take-video` | Explicit camera-only/no-audio bounded recording and global/tray Stop/Quit ownership. | Consenting real camera, playable MP4, duration and recovery. |
| `recognize-text` | Image/path/preview, engine/language validation, text/copy result; new language/engine stale-recognition regressions. | Generated text image recognized by installed native OCR/Tesseract languages, plus native cancel/error/copy. |
| `barcode-reader` | Generated QR/EAN13/Code128 decode, bounded text, image-path invalidation and no automatic URL opening. | Native generated-image preview/decode and explicit copy; additional advertised symbologies as applicable. |
| `measure-color` | RGB/HEX/HSL/CMYK/bounds, generated-image invalidation, successful-result error clearing and hidden sample ownership. | Generated-image native sample; real display/cursor/DPI and explicit copy separately. |

### System, windows and applications — 22 routes

| Tool ID | Reviewed core/control evidence | Native/effect acceptance still needed |
|---|---|---|
| `system-monitor` | Visible-only one-second nonoverlapping samples, bounded trace and snapshot invariants. | Actual CPU/RAM/swap/disk/process accuracy and native polling/leave. |
| `network-usage` | Local interface/rate/reset/session model, filters and visible-only sampling. | Actual interfaces/counters and filter changes; this is not network-privacy proof. |
| `cpu-temperature` | Two-second nonoverlapping fake-clock cadence and honest empty-sensor limits; no driver installed. | Real supported sensors, values and missing-sensor state. |
| `do-not-sleep` | Deadlines/display option, dedicated guard thread, Stop/expiry/Quit and stale-Start rejection fakes. | Actual awake lease and release on a consenting machine. |
| `sleep-timer` | Reviewed action/delay, independent deadline/warning worker and immediate cancellation. | Safe warning/cancel/recovery; actual power action only on a prepared test machine. |
| `mouse-activity` | Windows-only stopped default, issued requests, fake idle/input/movement/held-input/Stop/Quit service tests. | Current-binary physical input/duration/monitor/protected-desktop cases; earlier bounded pointer test remains separately dated. |
| `close-apps` | Reviewed HWND/PID selection, all-target validation and ordinary close, without forced termination. | Disposable unsaved-document prompts, refusal and stale-window behavior. |
| `switch-resolution` | Enumerated-mode validation, captured permit, independent timed rollback and Keep/Quit restoration. | Actual GPU/monitor/DPI rollback and recovery. |
| `eject-volumes` | Exact removable-drive review, fresh type check, lock/dismount/eject without force; invalid-drive tests. | Consenting removable/busy media and device refusal. |
| `energy-saver` | Installed profile/active-state parsing, literal GUID and fresh membership validation. | Real profile selection/switch/refusal and platform availability. |
| `hidden-files` | Opposite-state review, unrelated-bit preservation, generation guard and new actual-readback refusal test. | Real Explorer flag/readback and unrelated/protected-file setting preservation. |
| `uninstall-apps` | Fresh registered command review, literal executable/arguments, bounded leftovers scan and separately reviewed Trash. | Disposable installed app, real uninstaller prompts/refusal and exact leftover review. |
| `do-not-disturb` | Honest manual-settings capability and new captured-permit/cancel admission test. | Actual supported OS settings delegation; no automatic Focus toggle promised. |
| `airplane-mode` | Windows manual settings delegation, no radio-state success claim; new admission test. | Actual settings page behavior; other platforms explicitly unavailable. |
| `lock-screen` | Explicit fixed OS command/API and Quit admission; macOS password-on-wake limit. | Actual lock on a prepared machine, with saved work. |
| `hide-desktop` | Original visibility/lease/restore/retry fakes and new native window-identity restoration guard. | Actual visible/hidden initial states, tray/Presentation interaction and Quit restoration. |
| `window-on-top` | HWND/PID validation and original/last-state ownership; conditional Quit restoration. | Actual closed/elevated/reused/user-modified windows. |
| `window-manager` | Literal placement enum, odd-size/negative-origin geometry and target identity. | Actual work area, DPI and application minimum-size constraints. |
| `show-desktop` | Self/system/already-minimized exclusions, accepted-minimize tracking and partial restore fakes. | Actual windows/user restores/stale handles and partial refusal. |
| `focus-window` | Cutout/intersection/scale geometry and independently owned click-through overlays/cleanup. | Real click-through, target movement/minimize/close, mixed DPI, hidden Stop and Quit. |
| `launch-app-sets` | Saved local targets/literal args, transaction/review identity, per-target cancel and gate-free external handler. | Disposable native save/reload/delete-review and intended app launches. An admitted external app has its own lifetime. |
| `presentation-mode` | Awake/desktop composition, rollback/merge fakes, manual Start/Stop and default-off extra-display automation. | Real display detection, lease restoration, disconnect/retry and OS notification-settings limits. |

## Actual installed FFmpeg smoke

The integrator explicitly ran this command on Windows against the current working core source:

```bash
cargo +1.95.0 run -p oxbox-core --locked --example provider_smoke -- --media
```

The preinstalled, user-managed FFmpeg was found through the user's WinGet PATH. The example generated
one second of **160 × 120 orange video at 10 FPS** and a **440 Hz synthetic tone**, encoded an MPEG-4/AAC
fixture, converted it to WebM, refused a second conversion over the existing output, created a GIF
and verified its GIF87a/GIF89a magic header. It reported success. The local log is
`target/verification2-real-media.log`; generated fixtures were retained in the disposable
`oxbox-provider-smoke-44968-1795328827796446000` temporary directory.

This exercised actual provider conversion, not a fake version/sleeping executable. It used no physical
screen, camera or microphone, downloaded nothing and made no intentional network request. It exercised
the **core example**, not a hash-pinned release executable or the native forms. It did not prove every
preset, output playback, codec/build combination, recording path, provider privacy boundary or other OS.
The executable remains unbundled; its licensing depends on the installed FFmpeg build.

## Exact Windows production PE inspection

Static inspection used the installed Windows SDK `mt.exe` and Visual Studio `dumpbin.exe`, plus
PowerShell version/signature metadata. The reviewer did not execute the application or installers.
The inspected executable belongs to source `67f0715452fb601394f45d3296675d3b85df3fb5`:

| Property | Observed result |
|---|---|
| Artifact | `oxbox-native.exe`, **13,577,216 bytes**, matching the build artifact receipt. |
| SHA-256 | `6606f3d4e172e75bbebbe4201d92b5989fca15ab51b2769cac0893dbac412069` |
| PE / version | x86-64 PE32+, Windows GUI subsystem; file/product version **0.1.0**, product **Oxbox**, description **Oxbox native desktop toolkit**. |
| Signature | **NotSigned**, no signer certificate and an empty certificate directory. Clean-machine warnings/signing acceptance remain separate. |
| Manifest | Embedded Common Controls **6.0.0.0**; all six assembly-identity fields match the source manifest. `requestedExecutionLevel` is **asInvoker**, `uiAccess=false`, with no `autoElevate` element. The manifest does not request elevation. |
| Resource coexistence | The same nonzero resource directory contains **6 `RT_ICON` entries**, **1 icon group**, **1 version resource** and **1 manifest**. Directory size is 13,952 bytes; the resource section has 14,336 raw bytes. |
| Direct imports | 24 DLLs; no direct WebView/Edge or VC runtime DLL import appeared in the `dumpbin` dependency report. This is not a complete dynamic-dependency or runtime-network proof. |
| Staged notices | Source and staged notices match SHA-256 `6f22422e0098e8dc70702a06d63a321627d695fd0e9fbb952a137c8a85c792d2`: **355 packages, zero missing notice texts, 3 unresolved upstream attribution reviews**. This is not completed legal review. |

SDK extraction produced manifest SHA-256
`cd613330097d228ae01fce028e50954fe4595249d51edebc4461ec252b514517`, also present in the newly
built combined test host `oxbox_native-18a1400af352e0fc.exe`. That test host has its own nonzero resource
directory; this finding does not rely on the older failed test executable. Icon resource counts were
checked, but icon payload digests and rendered pixels were not revalidated in this inspection.

Local receipts are `target/verification2-pe-metadata.json`, `target/verification2-pe-headers.txt`,
`target/verification2-pe-imports.txt`, `target/verification2-pe-resources.txt`,
`target/verification2-production.manifest` and `target/verification2-test-pe-metadata.json`.
The build/artifact mapping is `target/verification2-artifacts.json`. Static PE inspection does not
establish native GUI correctness, installer execution, hardware behavior or universal zero network use.

## Actual native form and lifecycle checks

The production executable was copied without modification to `target/native-qa/verification2/` and
run with fresh explicit profiles. `forms.jsonl` and `forms-summary.json` contain exactly 49 unique IDs
matching the canonical catalogue. Each record confirms its actual Windows heading, Back control and
interactive-control presence. Screenshots were used for the launcher and selected generated-input
states; the 49-route record is primarily UI Automation evidence, not a screenshot collection. The three
monitoring forms read local status; process names, interface names and other private rows were excluded
from saved form evidence. The isolated profiles do not register installed startup or restore old services.

The Timer duration edit and QR generation/invalidation have separate UI receipts. Currency's opt-in
control was present with **Refresh from ECB disabled**. Single-instance activation preserved the active
Currency form. A later controlled provider run also preserved the actual busy Convert Audio / Video
form, input/output draft, Cancel control and the same child PID after hiding and activation.

The first 30-second no-output fixture completed before a later Cancel click; that click was not available
and **is not a cancellation pass**. The expected missing-output error was visible. A new, dependency-free
120-second waiting fixture then provided unambiguous lifecycle evidence: explicit Cancel removed one
child after about 11 seconds while the app stayed open, and a repeated job survived hide/restoration
before normal Ctrl+Q removed the app and child about 35 seconds after child creation. Neither output
file existed. Fixture source, hashes, PID/creation evidence and UI receipts remain in the ignored QA
directory. This tests lifecycle with a no-effect provider, not media encoding or device behavior.

An extra-modifier Quit attempt was inconclusive because its target window was unavailable. A later
sidebar click also encountered UI Automation geometry outside the current window; normal Ctrl+Q
completed Quit. These automation limitations do not establish a new shortcut or DPI defect and do not
close the existing exact-keyboard, resize/DPI or screen-reader manual gates.

## Remaining gates and next checks

- Keep the exact source/build/CI and bounded GUI/runtime mapping above separate from expanded manual
  acceptance. Proposed generated-file, device and effect cases remain proposals until recorded.
- Complete each route's native keyboard, focus, errors, expanded busy/result states, dialogs,
  copy/export, Back, hide, restart and Quit cases using generated fixtures where possible. Opening
  a default form does not exercise its effect or result paths.
- Use a consenting test machine for live clipboard/concealed formats, camera/microphone, capture,
  recording, monitor/DPI, protected desktop/held input, windows/power/display/eject/uninstall and
  notification permissions. Test unsupported/provider failures as real errors.
- Complete Windows clean-machine install/upgrade/login opt-out/uninstall, macOS/Linux runtime and
  installer acceptance, actual screen-reader checks, and independent human security/license review.
- Keep DNS/packet/OS-broker and actual-provider privacy coverage explicit. Owner-PID endpoint samples
  have timing and attribution limits; neither a native renderer nor this source pass proves universal
  zero network activity. See [privacy acceptance](../PRIVACY_ACCEPTANCE.md).

Follow [TESTING.md](../TESTING.md), [the installer protocol](../INSTALLER_ACCEPTANCE.md) and
[the release checklist](../RELEASE.md). A complete source review or green test suite does not authorize
publishing the draft or marking clean-machine/hardware acceptance complete.

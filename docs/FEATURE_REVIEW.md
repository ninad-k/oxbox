# Functional feature review

Review date: 2026-11-22. Three parallel source audits examined all **49 native routes**: 13 utilities,
14 media/capture/vision tools and 22 system/window/app tools. They traced concrete controls to core APIs,
including parameters, results, copies/exports, errors, confirmation, persistence and navigation/worker cleanup.
They did not perform a manual run of every feature.

Use [FEATURES.md](FEATURES.md) for the generated catalogue and platform/provider notes, and the
[native migration matrix](../tasks/native-migration.md) for each route's verification state. This review
records functional findings rather than maintaining another list of 49 names or marking manual gates complete.

The [second bounded native feature pass](verification/2026-11-22-native-feature-pass.md) records the
follow-up review of all 49 forms, new parameter/drop/admission regressions, actual synthetic FFmpeg
smoke and the separate native/hardware checks still pending. Its final validation receipt must be
completed for the revision being released; proposed GUI cases are not performed evidence.

## What was examined

| Group | Source and checks |
| --- | --- |
| 13 utilities | Typed text/unit/currency forms, file workflows, encryption and independent reminder clocks. Checked result invalidation, reviewed Trash/removal, saved-state transactions and hidden-window ownership. |
| 14 media tools | Capture/area geometry, consent, recording/Stop, clipboard policy, image/code generation, OCR/barcode/color results and provider-backed work. Checked previews, copies/exports and cancellation/leave/Quit boundaries. |
| 22 system tools | Monitoring cadence, explicit OS-action review, window identities, display rollback, application sets, owned effects and independent tray recovery. Checked ephemeral reviews separately from deliberately active controllers. |

Existing headless tests render all 49 default routes and check accessible control names. Selected expanded
state tests supplement them. These tests do not open real cameras/microphones, collect clipboard contents,
capture the desktop, move the pointer, send notifications, change power/display/radio settings, close user
apps or invoke live Trash. Generated provider children and fake clocks/backends test the relevant boundaries.

## Reproduced findings and regressions

The following discrepancies were reproduced before their implementations changed. Some initial fixture
compile/targeting failures were corrected before behavior evidence was accepted. For example, media input
tests target the actual AccessKit **TextInput**, not a text-content child with the same displayed value.

| Finding / resulting behavior | Regression names or filter | Scope |
| --- | --- | --- |
| EAN-13 advertised 12-digit input but core rejected it. Accept 12 digits or a valid 13-digit checksum through the existing barcode library. | `community_ean13_twelve_digits_match_the_checked_thirteen_digit_code` | Generated PNG/SVG equality; invalid checksum still rejected. |
| Editing an OCR, barcode or color-image path retained old results/previews and queued completions. Invalidate the work and pixels belonging to the previous file. | `community_editing_ocr_image_discards_old_result_and_queued_completion`; `community_editing_barcode_image_discards_old_decoded_values`; `community_editing_color_image_discards_old_pixels_and_queued_preview` | Actual headless field edits, generated pixels and injected completions; no file capture or OS clipboard. |
| Successful asynchronous color sampling retained an earlier validation error. Success clears the preceding error. | `community_successful_async_color_clears_an_earlier_validation_error` | Injected color result; no screen sampling. |
| Changing clipboard search left old-filter rows and a queued old snapshot visible. Discard them before refreshing the new query. | `community_search_edit_discards_old_filter_rows_and_queued_snapshot` | Generated in-memory summaries and headless field edit; history stays paused. |
| Currency Cancel could wait for curl's blocking read. Share cancellation with the owned bounded byte runner; preserve the fixed request and strict UTF-8 boundary. | `refresh_cancel_before_start_skips_the_transport`; `refresh_active_cancel_interrupts_generated_provider_without_network`; `bounded_raw_runner_preserves_binary_bytes_and_text_runner_keeps_its_contract` | Generated self-terminating child processes, not an ECB request. The earlier active-Cancel RED took 4.067 seconds. |
| Currency result acceptance lacked a separate atomic cache commit/error boundary. Commit an admitted candidate before replacing active rates; abandon late results. | `successful_cache_acceptance_commits_disk_and_announces_completion`; `failed_cache_commit_preserves_previous_in_memory_and_backed_up_book`; `abandoned_fetch_cannot_publish_a_late_cache_or_result` | Temporary cache stores, save failure and late-result fixtures. |
| Alarm/date removal omitted review and needed stale-identity/Quit checks. Review the exact record without holding the runtime lock, then revalidate before removal. | `cancel_removal_preserves_runtime_and_disk_and_releases_runtime_lock_for_review`; `changed_identity_or_quit_during_confirmation_rejects_late_removal`; `acceptance_removes_only_the_reviewed_record_and_persists_other_reminders` | Fake confirmation and temporary stores; no delivered notification. |
| CPU Temperature lost its two-second live sampling cadence. Schedule only while the view is active and avoid overlapping jobs. | `temperature_auto_sampling_waits_two_seconds_and_stops_off_view` | Fake-clock cadence; no sensor driver installed. |
| App-set cancellation could admit remaining targets, and an external handler could hold the system gate. Check between targets and release the gate before the handler. | `cancellation_after_first_open_prevents_later_launch_targets`; `launch_admission_releases_system_gate_and_honors_cancel_between_targets` | Fake launch callbacks. An already admitted OS request cannot be revoked; intentionally launched applications have their own lifetime. |
| System navigation lacked ephemeral-task/review cleanup. Abandon scans/reviews while preserving deliberately armed controllers. | `leaving_system_tools_cancels_ephemeral_scan_and_clears_review`; `leaving_a_power_form_keeps_an_explicitly_armed_fake_timer` | Generated scan and fake timer; no uninstall, power or desktop effect. |
| New App Set retained reviews for the preceding set. A new draft discards old launch/delete reviews. | `a_new_application_set_discards_reviews_for_the_preceding_set` | Native model fixture; no application launch. |
| World Time did not match pasted IANA IDs containing underscores. Normalize both query and zone labels consistently. | `zone_search_accepts_canonical_iana_and_readable_city_queries` | Pure bundled-zone search; exact IDs, city names, case and whitespace. |
| Post-encryption Trash retained paths without proving that the original and backup were unchanged. Bind a transient receipt to the actual processed/read and output/write streams, file IDs and stamps; revalidate both files after confirmation. | `encryption_trash_receipt_refuses_changed_original_or_missing_changed_backup`; `encryption_trash_receipt_requires_confirmation_and_cancellation_preflight`; `encryption_receipt_hashes_actual_stream_when_source_rewrites_preserve_metadata` | Generated files/fake Trash, including same-size/time rewrites. This preflight is not an atomic lock against a concurrent filesystem change. |
| The shared result poller consumed cleanup success before the encryption view could display it. Keep a matching durable completion and suppress a repeat offer. An errored/cancelled attempt asks the user to check the original/Trash rather than falsely claim it was kept. | `encrypted_original_cleanup_survives_poll_and_does_not_reoffer_trash` | Injected completion, real poll and headless tree; generic cleanup is not misattributed, and reset/cancel/leave clears the transient association. No live Trash. |

Tests live beside the responsible [core tools](../crates/oxbox-core/src/tools/) and native
[utilities](../crates/oxbox-native/src/tools/utilities/), [media](../crates/oxbox-native/src/tools/media/)
and [system](../crates/oxbox-native/src/tools/system/) forms. The names above can be passed as Cargo test
filters. Normal full checks are documented in [CONTRIBUTING.md](../CONTRIBUTING.md#quick-start).

### Archive and encryption scope

An initial archive-extension concern was withdrawn after checking the actual dispatch: bounded magic-byte
detection already supports extensionless archives. `extensionless_created_zip_and_tar_gz_round_trip`
passed as a generated continuity fixture; no unnecessary filename restriction was introduced.

The encryption receipt guard regressions failed to compile before implementation, then passed against
generated file effects. The existing real age round-trip test additionally checks receipt stream hashes
and fake cleanup admission/refusal, while retaining authentication/tamper/overwrite checks. Default
encryption keeps originals; the optional Trash action has its own explicit review and bounded revalidation.
No live Trash call was used. The final integrated run below must include that real round-trip regression.

## Verification record and limits

The integrated Windows run passed **221 core and 80 native tests, zero failed/ignored**, including the real
age receipt round trip and World Time fix. The final native-only run passed **81 tests**, including durable
cleanup status and honest cancelled-outcome wording; strict all-target Clippy and formatting passed with Rust 1.95.0.
The final source, CI and artifact
mapping belongs in [VALIDATION.md](VALIDATION.md). Initial REDs, generated fixture passes, source review and
production runtime observations are different evidence.

The preceding functional source `a7c91bd` passed [all five CI jobs](https://github.com/ninad-k/oxbox/actions/runs/44987974907):
Windows 225 core / 81 native, macOS ARM 219 / 82, Linux 218 / 82, zero failed/ignored. The follow-up also
corrected Darwin exited-group cleanup and replaced the mouse test's fixed scheduling assumption with a
bounded worker acknowledgement. Original raw-byte coverage remains enabled; macOS additionally passes
eight fast provider exits. Strict Clippy and native production builds pass on all three systems.

The exact Windows release binary was observed separately with a fresh isolated profile: exact IANA search
and clock creation, 12-digit EAN-13 preview, and normal Quit passed. No clipboard/export/device/provider or
destructive action was invoked. Its finite 240.073-second endpoint window and hash mapping are recorded in
[runtime privacy](RUNTIME_PRIVACY.md); this bounded subset does not close the 49-tool manual matrix.

The earlier `d8d1d1e` all-platform CI and bounded Windows GUI/provider observations remain evidence for
that exact source/artifact. They are not silently transferred to these newer functional changes.
[Privacy acceptance](PRIVACY_ACCEPTANCE.md) records the hashes, separate windows, completion Status and
controlled no-effect job Quit proof, including an earlier inconclusive attempt.

The second bounded pass is source `67f0715`, with
[all five CI jobs passing](https://github.com/ninad-k/oxbox/actions/runs/44994827768): Windows 226 core / 94
native, macOS ARM 220 / 95 and Linux 219 / 95, zero failed/ignored. Its exact production Windows EXE
opened all 49 default forms through Ctrl+K search and accessible field/tile controls. Timer preview,
generated QR invalidation, actual no-effect-provider Cancel, hidden-job restoration and active Quit
were also checked. The [dated record](verification/2026-11-22-native-feature-pass.md) separates those
cases, actual synthetic FFmpeg smoke and two independently bounded endpoint windows from remaining
effect/hardware/manual acceptance. Earlier measurements are retained with their original artifact hashes.

## Remaining acceptance work

- **Native interactions:** complete each route's keyboard, focus, error, copy/export, dialog, Back, hide,
  restart and Quit cases on a recorded candidate. Default-route headless coverage is not a full UI exercise.
- **OS effects and hardware:** use a consenting test machine for cameras/microphones, live clipboard and
  concealed formats, screenshot/recording, multi-monitor/DPI, held input/protected desktops, actual window,
  power/display/eject/uninstall behavior and notification permissions. Unsupported/provider refusals must
  remain explicit errors, not mock success.
- **Installed providers:** validate actual FFmpeg codecs, output playback and installed OCR languages with
  disposable generated inputs. Fake version and sleeping-job fixtures verify dispatch/lifetime, not media
  decoding or a physical device. Other-platform runtime acceptance remains separate.
- **Privacy and release:** DNS/packet/OS-broker attribution and actual-provider coverage remain open;
  finite owner-PID endpoint samples are not a universal zero-network guarantee. Clean-machine install,
  login/upgrade/uninstall, accessibility, human security and license review also remain open as tracked in
  [the release checklist](RELEASE.md).

For a contribution, pick one bounded case in the [migration matrix](../tasks/native-migration.md), use a
generated fixture/fake first, and record the expected/actual result, exact source, platform and limitations.
Follow [the first-contribution walkthrough](GETTING_STARTED.md). Source audit completion does not mean
“all 49 features manually passed,” and this document does not authorize publishing the draft.

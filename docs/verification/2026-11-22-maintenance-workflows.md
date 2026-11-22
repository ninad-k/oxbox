# Maintenance and workflow verification — 2026-11-22

The expansion preserves the original 49 IDs and adds seven tools, for **56 tools in eight categories**.
See [the user and recovery guide](../MAINTENANCE.md), [generated feature index](../FEATURES.md) and
[source/automation/manual matrix](../../tasks/native-migration.md#maintenance-and-productivity-expansion--2026-11-22).
This record distinguishes generated/fake-backend checks from actual desktop and installer behavior.

## Verification receipt

| Item | Observed evidence |
|---|---|
| Functional source | [`c30a179a573424b178def065a3c5c0f5c9397074`](https://github.com/ninad-k/oxbox/commit/c30a179a573424b178def065a3c5c0f5c9397074). Later receipt/documentation changes do not alter the native source or build inputs. |
| Local native checks | Exact Rust 1.95.0: **279 core + 112 native**, zero failed/ignored; formatting and strict all-target native/workspace Clippy passed. Combined-host compatibility also passed **12 legacy + 112 native** tests. |
| Cross-platform CI | [Run 45011422402](https://github.com/ninad-k/oxbox/actions/runs/45011422402), all five jobs successful: Windows **279/112**, macOS ARM **274/114**, Linux **273/114** core/native, no failures or ignored tests; strict Clippy, native production builds and actual Rust 1.95 checks passed. |
| Legacy/site checks | Svelte 0 errors/warnings, **39 UI tests**, production build, **18 packaging + 3 endpoint-observer** regressions and **4 website tests** passed. The generated feature index matches 56 IDs. The final local link check found **405 references / 76 Markdown files**, none missing. |
| Public website | [Pages run 45011422424](https://github.com/ninad-k/oxbox/actions/runs/45011422424) built and deployed the same source. The public page returned HTTP 200 with the 56-tool catalogue and new workflows/maintenance sections. |
| Production Windows EXE | **14,381,056 bytes**, SHA-256 `006769afce34f73fd4b1b9eee2216356e3f8ac7f39101d8fcf1587a74d8cb841`, built with the selected wrapper and Rust 1.95.0. GUI/profile checks used a byte-matched copy. PE version 0.1.0, Windows GUI, unsigned; native imports, static CRT and Common Controls v6/asInvoker/uiAccess=false manifest were inspected. |
| Local packages | MSI **6,017,024 bytes**, NSIS **4,245,923 bytes**. Static report `target/installer-verification/run-8qZqa7/report.json` passed, including extracted MSI payload hashes, owned-startup cleanup, non-forcing lifecycle and NSIS compiler output. No installer/custom action was executed. |
| Refreshed draft | Release ID **453727915** targets `c30a179`, remains draft/prerelease and has no actual version tag. All six replacements were re-downloaded and hash/size matched; downloaded static report `run-Uy2Bos` passed. Old `67f0715` assets and notes were backed up and verified before replacement. [Artifact hashes](../VALIDATION.md#maintenance-and-workflow-candidate-c30a179). |
| Bounded runtime privacy | Same EXE: **300.103 s / 6,001 samples**, **90.037 s / 1,800**, **60.031 s / 1,200**, no owned TCP/UDP rows, query/marker failures or hash changes. First two end with an app alive; only the final restart/Quit window ends with none. [Exact timing/exclusions](../RUNTIME_PRIVACY.md#current-maintenance-candidate-c30a179--2026-11-22). |

## Tests-first and parallel review details

The integrator ran Cargo serially while three agents implemented separate file, OS-maintenance and
capture/text/site work packages. Missing APIs failed compilation before the initial implementations.
Runtime regression failures were then observed for confirmation, cancellation, stale review and recovery
behavior. All fixtures use generated content or injected effects; no real shredding, Trash, registry edit,
startup disable/restore, private browser scan, live capture or device/power action was used by these tests.

Independent source review reproduced and fixed these additional problems:

| Finding | Check and resulting behavior |
|---|---|
| Resize presets had different limits from actual resize work. | `resize_presets_` first failed both wide/narrow acceptance and excessive-pixel refusal. Presets now use the same core dimension validator, with mode-aware inactive fields. |
| A same-path replacement could inherit cleanup review by retaining length and timestamps. | `cleanup_review_rejects_replaced_file_with_preserved_size_and_timestamps` failed with fake Trash admitted, then passed after volume/file-ID or device/inode stamping and per-item revalidation. The final filesystem check/use race remains a separate limitation. |
| A browser could reopen between scan and cleanup. | Fake activity tests fail without the active-browser guard. Recognized running-browser caches are read-only, and activity is checked again before each browser-cache Trash request. This does not lock the browser. |
| Recovery backup bytes could change after preview. | Immutable parsed startup/registry recovery review binds confirmation to the exact bytes and refuses a changed backup. Destination records are never overwritten during restore. |
| Cancel appeared as an urgent error. | `requested_cancellation_is_a_neutral_polite_status_not_an_error_alert` failed against the prior alert role, then passed with a neutral polite status. Partial destructive changes still require review. |
| A modal could return after tray Quit and persist a late preset/Undo change. | Generated-profile Save, Remove and Discard Undo regressions failed before the shared registry admission check; they now preserve stored bytes and in-memory data after Quit. |
| Startup Manager could bypass Oxbox's saved login preference. | Exact owned entries and recovery are delegated to Settings, with ownership rechecked at action admission; unrelated or repurposed commands remain distinct. |

## Verification boundaries

- Core tests cover bounded parsing/configuration, generated filesystem/image/text data, fake registry/startup/
  overwrite/Trash backends, partial cancellation, no-replace moves and exact recovery guards.
- Native headless tests cover idle routes, accessible control names, selected review branches, flattened
  image export selection, stale draft cleanup and option-only preset round trips. They do not certify
  real screen readers, a GPU, hardware or a desktop environment's effects.
- The retained legacy UI displays a native-required message for the seven new routes. Their real operations
  are in the selected eframe host; Svelte adapter importability is not a second implementation claim.
- The local website was inspected at desktop width and a temporary 375 × 812 viewport. Registry search,
  a macOS empty result, Show all tools reset/focus, mobile workflow text and expanded maintenance FAQ were
  observed. Catalogue/source/anchor/resource checks and all four website tests passed. No installer download
  or published-release claim was added.
- The lockfile and direct dependency manifests are unchanged. Current audits report zero vulnerability
  entries and zero pnpm advisories, plus the existing eight maintenance warnings and GLib 0.18.5
  `RUSTSEC-2024-0429` unsoundness warning. That unresolved Linux binding issue remains a release gate.

## Current-binary Windows GUI checks

The exact EXE above ran with an explicit disposable profile and generated input data. Its fresh Dashboard
showed zero favorites, an optional starter picker and All tools 56. All **seven new default forms** opened
with real named controls and their platform/safety limits. Clean Drive's changed default form also opened.
No live startup/registry/cache scan, Trash, permanent overwrite/delete, uninstaller or OS/device action was run.

- `= 12 * 8` displayed **96**. `timer 10m` opened **00:10:00** with Start available and no session running.
- A generated timer preset saved only `{ seconds: 600 }` and its name/tool/version. Selecting 1 minute then
  Apply restored **10 minutes**, with the explicit “no work was started” status and Start still available.
- Storage Analyzer reported the generated **38-byte** file, one document, one visited path and no skips.
- Browser Review read one generated manifest and reported its name, ID, version and `storage` permission,
  with enabled state explicitly unknown. No live browser profile or account/history data was inspected.
- Batch Rename preview showed `generated.txt` → `qa-generated.txt` for the generated file; Apply was not
  invoked. File Shred remained unarmed, with explicit irreversible/SSD/cancellation limits.
- Clipboard History showed **Paused / zero items**. Clean pasted text produced a **25-byte** trimmed preview
  from generated multiline text. History was never enabled and no OS clipboard read or Copy was requested.
- Screenshot defaulted to memory-only review; raw save/copy controls were disabled. A generated **320 × 160**
  PNG opened in the actual native file picker. An **80 × 40** black redaction committed one edit and saved
  through a second native picker to a new flattened PNG. Independent decoded pixel comparison found exactly
  **3,200 opaque black pixels**, **48,000 unchanged outside pixels**, and **zero unexpected pixels**.
  The output SHA-256 is `352f6c65011790faefb2a2fb995313f61fe63574af6d1db647045b4aa6223009`.
  No screen was captured, clipboard exported or OCR provider started for this test.
- Title-bar Close removed the window while the process remained alive. A second invocation restored the
  same window/process with the annotated preview and saved receipt intact. Explicit Quit then removed the
  window and all processes at that exact QA executable path. This was a quiet editor session, not active
  destructive/provider cancellation or clean-machine login acceptance.

GUI evidence does not replace destructive-effect, full keyboard-only, screen-reader, provider/hardware or
macOS/Linux runtime acceptance. Cropping, arrows and ASCII labels have core/headless coverage; this GUI
pass exercised solid redaction and explicit export.

## Pending acceptance

Clean-machine install/login/upgrade/uninstall remains pending by explicit user instruction. Native effects
for new tools, real provider/hardware behavior, all-OS runtime privacy including DNS/OS brokers,
screen-reader/IME/contrast/DPI checks, independent security/license review, signing and NSIS independent
payload extraction remain open. File overwrite is not SSD/media sanitization or NIST certification.
The existing release must stay an unpublished draft until its required gates are satisfied.

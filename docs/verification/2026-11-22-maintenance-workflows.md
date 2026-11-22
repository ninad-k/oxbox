# Maintenance and workflow verification — 2026-11-22

The expansion preserves the original 49 IDs and adds seven tools, for **56 tools in eight categories**.
See [the user and recovery guide](../MAINTENANCE.md), [generated feature index](../FEATURES.md) and
[source/automation/manual matrix](../../tasks/native-migration.md#maintenance-and-productivity-expansion--2026-11-22).
This record distinguishes generated/fake-backend checks from actual desktop and installer behavior.

## Tests-first and parallel review

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

## Pending acceptance

Clean-machine install/login/upgrade/uninstall remains pending by explicit user instruction. Native effects
for new tools, real provider/hardware behavior, all-OS runtime privacy including DNS/OS brokers,
screen-reader/IME/contrast/DPI checks, independent security/license review, signing and NSIS independent
payload extraction remain open. File overwrite is not SSD/media sanitization or NIST certification.
The existing release must stay an unpublished draft until its required gates are satisfied.

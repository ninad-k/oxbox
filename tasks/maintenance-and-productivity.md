# Maintenance and daily workflows

User-authorized scope, 2026-11-22: implement the five proposed workflow improvements and useful local
CCleaner-style maintenance across Windows, macOS and Linux, plus Windows registry cleanup. Preserve all
49 existing tool IDs, native eframe 0.37/Rust 1.95, favorites-only dashboard and default-off networking.
The current draft and clean-machine gate remain pending until new evidence is recorded.

## Work packages

- Batch Rename: literal replace, prefix/suffix, numbering, exact preview, conflict/identity checks,
  explicit Apply and bounded guarded Undo. Never replace existing files.
- Saved presets: opt-in named resize/checksum/timer/unit configurations, validated bounded storage;
  no passphrases, copied text, file contents, destination paths or active-session restoration.
- Quick Actions: bounded arithmetic, unit conversions and a configured timer from the existing launcher;
  no arbitrary shell execution, hidden destructive operations or automatic clipboard reads.
- Screenshot annotation: crop, arrows, original bitmap text, opaque redaction, flattened preview/export,
  exclusive save and explicit OCR/copy. Original images remain unchanged.
- Text cleanup: explicit paste/type, whitespace/line cleanup, preview and Copy; history monitoring remains
  a separate paused-by-default feature. No implicit cross-app input injection.
- UX: neutral cancellation status, consistent controls, optional first-run favorite selection, guidance
  for close-to-tray/Quit, and native website screenshots with task-oriented feature explanations.
- Maintenance: OS-specific safe cache/temp cleanup, current-user startup review/disable/restore,
  read-only storage/large-file analysis, browser-extension review, selected-file overwrite/delete,
  and Windows current-user registry review/backup/remove/restore.

## Operating-system boundary

Cleanup categories are exact per-OS allowlists resolved from current-user known directories. Downloads,
documents, password/cookie/history databases and system roots are never classified as cache by a broad
directory heuristic. Scans are explicit, bounded and cancellable; links/reparse redirects are refused.
Apply revalidates the reviewed entries and uses Trash. Active/locked/replaced entries must fail safely.

Startup management targets HKCU Run values on Windows, user LaunchAgents on macOS, and user XDG autostart
desktop entries on Linux. Changes are backed up and reversible; system services, elevated startup records
and security software are outside the managed scope. Availability/session restrictions must be visible.

Registry cleanup is Windows-only and opt-in: positively identified obsolete HKCU application paths,
exact value backup before removal, revalidation and guarded restore. No blanket COM, driver, service,
system-hive or empty-key sweep; no performance or corruption-repair promise.

The user explicitly requested implementation of permanent file shredding, which is a scoped exception to
the ordinary Trash default. Each use requires exact file review, typed acknowledgement and confirmation.
It never runs automatically. Streaming overwrite/sync/unlink is not a secure media sanitization guarantee:
SSD wear levelling, copy-on-write storage, snapshots, backups and hard links require explicit limits.
No whole-drive or free-space wipe is part of this file-only feature.

## Evidence and dependencies

Use existing std, image, serde, platform bindings and approved native adapters; any additional direct
dependency needs separate approval. Core/fake-backend RED must be observed before implementation.
Tests use generated files and injected effects, never live registry/startup changes or real user deletion.
Root coordinates serial Cargo; agents own independent modules. New tools require canonical metadata,
native accessible forms, legacy comparison adapters, website/docs and per-platform CI.

CCleaner is a functionality reference only. Use public descriptions, original code/copy/icons and a
feature-by-feature coverage table; do not claim identical coverage or copy proprietary artifacts.
Cloud cleaning, driver/software downloads, antivirus, whole-drive sanitization and automatic unreviewed
deletion are not silently introduced by this local maintenance scope. Document those product differences.

Primary references:
- [CCleaner product functionality](https://www.ccleaner.com/legal/products-policy)
- [CCleaner Mac feature categories](https://www.ccleaner.com/ccleaner-mac)
- [Windows registry configuration guidance](https://support.microsoft.com/en-us/windows/experience/system-configuration-tools-in-windows)
- [NIST media sanitization, revision 2](https://csrc.nist.gov/pubs/sp/800/88/r2/final)

## Source and acceptance tracking

- [x] Implement the seven new catalogue routes and preserve the original 49 stable IDs (56 total).
- [x] Implement preview/recovery, safe presets, Quick Actions, annotation and explicit text cleanup.
- [x] Implement per-OS maintenance scopes with fake-effect tests, exact review and documented limits.
- [x] Add the maintenance/recovery guide, generated feature index and task-led GitHub Pages sections.
- [ ] Complete final source/CI/build and current-binary Windows GUI/runtime verification.
- [ ] Refresh the six verified Windows draft assets and record their downloaded hashes.
- [ ] Complete new per-tool native effect, assistive-technology and other-OS runtime acceptance.
- [ ] Clean-machine install/login/upgrade/uninstall: intentionally pending by user instruction.

Source checkboxes describe implementation only. [The expansion matrix](native-migration.md#maintenance-and-productivity-expansion--2026-11-22)
and [dated validation](../docs/VALIDATION.md) record independent automated/manual/release evidence.

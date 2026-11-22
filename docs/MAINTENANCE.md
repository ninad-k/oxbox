# Maintenance and everyday workflows

Oxbox combines small local workflows with explicit maintenance reviews. Start with a report or preview,
inspect the selected paths or entries, then confirm a specific change. The launcher remains a
favorites-only dashboard; search reaches the full catalogue. New source features do not close the
[native acceptance](../tasks/native-migration.md), [clean-machine](INSTALLER_ACCEPTANCE.md) or
[release](RELEASE.md) gates. Installers remain in an unpublished draft.

## Prepare, preview and reuse

### Batch Rename

1. Select regular local files. Choose literal replacement, prefix/suffix and optional numbering rules;
   extension preservation is enabled by default.
2. Preview the complete old/new name list. Conflicting destinations, swaps and case-only renames are
   refused. Renames stay in the same parent folder and never replace an existing file.
3. Review the exact batch and confirm Apply. Progress, cancellation and partial outcomes are visible.
4. Review Undo for the latest recorded batch. Undo checks file identity, size, timestamps and content
   before moving a file back, and refuses a changed file or occupied original path.

The batch is bounded to 256 files and 128 KiB of combined path text. The local write-ahead recovery
record is `native-batch-rename.json`: pending intent is recorded before an effect and each completed
move is recorded afterward. A partial/cancelled or interrupted batch can be reviewed for recovery;
the record is not an unlimited version history. Replacing or discarding it is explicit. Keep a separate
backup of important files; cancellation does not undo already completed moves.

### Quick Actions and saved presets

Use **Ctrl/Cmd + K** to open launcher search. Examples:

| Input | Enter action |
|---|---|
| `= 12 * 8` | Copy the calculated result. |
| `12 inches to cm` or `convert 12 in to cm` | Open Unit Converter with those values. |
| `timer 10m` or `timer 1h30m` | Open a configured timer; it does **not** start automatically. |

Quick Actions are bounded arithmetic/unit/timer parsing, not a shell. They do not execute arbitrary
commands or automatically inspect clipboard contents. Arithmetic Copy is an explicit action.

Named presets are opt-in for resize options, checksum algorithm, timer duration and unit choices.
Save and Apply are explicit. Validation happens before replacing the current configuration; applying a
resize preset refuses a running resize job. Presets exclude source/destination paths, file contents,
passphrases, pasted text and active sessions. They do not start a timer or file job on restoration.

### Screenshot review and text cleanup

Screenshot capture defaults to **review before export**. Pixels remain in memory while you crop, add
arrows or text, and apply opaque redaction. Inspect the current flattened preview, then explicitly save
a **new PNG**, copy that image, or request offline OCR. Existing image files remain unchanged. Opening a
local generated image provides the same editor without capturing a screen.

Redaction replaces the covered RGB and alpha channels with opaque pixels. PNG export contains the
flattened bitmap, not hidden editable layers or an embedded original. The in-memory original remains
available for Reset. Optional **direct-save mode** exports the unedited capture immediately; later
redaction cannot change those earlier files, system clipboard contents or another application's copies.

Text labels use an original 5×7 bitmap font: printable ASCII letters, digits and punctuation/newlines,
at most 256 characters and 12 lines, with scale 1–8. Unicode labels are explicitly unsupported. OCR is
separate and uses installed Windows language packs or local Tesseract; it does not download a provider.
Its private temporary flattened PNG is removed before recognition text is returned, with cleanup
failures reported for review.

In **Clipboard History**, choose **Clean pasted text** to paste/type plain text, select line trimming,
horizontal whitespace collapse, empty-line removal or newline normalization, then preview and Copy.
The input is limited to 1 MiB. This does not enable clipboard history or automatically read the system
clipboard. Switch to the destination and paste yourself; automatic cross-application paste is not
implemented. Drafts/results stay in memory and clear on leaving the view; they are not logged or saved
in presets. Explicitly copied text can remain in the OS clipboard or another clipboard manager.

## Choose the maintenance tool

These are specific local capabilities, not a general health score, repair service or speedup promise.
**Maintenance Review** brings local findings and links to separately confirmed actions together. Review
reminders use Alarm; a reminder does not scan-and-delete, change settings or apply a report unattended.

| Task | Tool and behavior | Boundary |
|---|---|---|
| Inspect known temporary/cache files | **Clean Drive** scans exact per-OS allowlists, shows counts/bytes and reviews selected Trash operations. | No broad home/Documents/Downloads sweep. Partial or limited scans remain read-only. |
| Inspect browser cache | Known cache leaves for supported browsers can appear in Clean Drive. | History, saved passwords, cookies, sessions and account databases are excluded. Recognized active-browser cache results are read-only. |
| Find matching file contents | **Find Duplicates** groups content matches and reviews selected copies before Trash. | This is not similarity scoring for blurry photos, an automatic keep/delete decision or cloud analysis. |
| Understand large files | **Storage Analyzer** explicitly scans a chosen local root for metadata, logical sizes/categories and a bounded largest-file list. | No content upload or deletion action. Skipped entries, mounts and limits affect totals. |
| Review login entries | **Startup Manager** inventories supported current-user records and offers backed-up disable/guarded restore. | It does not stop a running process, change a service or promise a faster login. |
| Review browser extensions | **Browser Review** reads bounded extension metadata from known local profile locations. | It does not edit browser preferences or remove extensions. Unknown enabled state must be checked in the browser. |
| Remove an application | **Uninstall Apps** uses an explicitly reviewed Windows registered command; new user-owned macOS/Linux paths are described below. | No silent uninstall, package-manager mutation, leftover auto-selection or downloaded remover. |
| Review obsolete Windows application paths | **Registry Review** checks a narrow HKCU App Paths value scope with backup and guarded restore. | No whole-registry cleaning, COM/service/driver scan or system-hive repair. |
| Permanently process selected files | **File Shred** is a separate advanced overwrite/sync/delete workflow with typed review. | Irreversible; not a media-sanitization guarantee. No drive/free-space wipe. |

### Cache and storage checks

Cache/temp candidates must meet the supported location and age policy: the latest available access or
modification timestamp is at least 24 hours old. Known browser cache, thumbnail/shader and package-cache
locations are specific leaves, not permission to traverse every profile. Close the relevant browser and
rescan before considering its cache; a recognized active browser makes those findings read-only, and
process state is checked again before a Trash request. A change in process/path state, an unfamiliar
browser build or unreliable access timestamps can still limit the report. It is not a lock on the browser.

Use **Review exact file paths…** beside a Clean Drive category to inspect the individual candidates.
After selecting categories, the final inline review shows an immutable list of the files sent to Trash;
confirm that list explicitly or cancel it. Changing the selection discards the pending confirmation.

Clean Drive's scan is bounded per category to 50,000 entries and depth 20. A partial result is not eligible
for cleanup. Apply rechecks the reviewed paths, age and identity before Trash; locked, replaced or linked
entries fail safely. Review the actual paths even when a category name looks familiar. Cleanup can remove
rebuildable cached data; it is not a promise to repair an application or improve measured performance.

Storage Analyzer defaults to 250,000 entries, depth 32 and 100 largest files, with maxima of 1,000,000,
128 and 1,000 respectively. It skips links and other mounts and reports unreadable/limited results.
Logical sizes are not necessarily allocated disk space, unique physical blocks or reclaimable bytes.
The analyzer itself has no cleanup operation.

### Current-user startup: disable with recovery

| OS | Managed scope | Disable / restore behavior |
|---|---|---|
| Windows | Exact `REG_SZ`/`REG_EXPAND_SZ` values in **HKCU Run**. | Save an exclusive verified Oxbox JSON backup, then remove the reviewed value. Restore only if that value is absent. HKLM, StartupApproved, services and loaded processes are untouched. |
| macOS | Current user's legacy `~/Library/LaunchAgents/*.plist`. | Park the exact file under a non-`.plist` name in the same folder. Restore requires matching parked bytes and an absent original path. Loaded jobs and modern Login Items are not managed. |
| Linux | Current user's XDG autostart `.desktop` entries. | Set `Hidden=true` within the Desktop Entry group while preserving other groups and overrides. Restore requires the exact disabled bytes. Do not remove an override and accidentally reactivate a system entry. |

These changes affect later logins; they do not start/kill processes now. Review the immutable parsed
backup before confirming recovery. If a backup or current entry has changed, restore refuses the
stale review instead of overwriting a newer choice. Keep backups until recovery is no longer needed.
Recovery records contain the reviewed entry data, including commands and paths; treat them as private
local files rather than posting them with a public issue.
This tool is separate from **Oxbox's own launch-at-login setting**; installed app defaults and upgrade
opt-out preservation remain described in [USAGE.md](USAGE.md).

### Windows Registry Review

The supported candidate is an unambiguous **default `REG_SZ` value in HKCU App Paths** whose `.exe`
target is missing on an available fixed local drive. An unavailable/network drive or uncertain target
is not evidence that an application is obsolete. Review the exact entry and exclusive backup before
removal. Only the default value is removed; its key and other values remain. Restore is guarded and
requires the destination value to be absent.

There is no HKLM sweep, COM/driver/service/uninstaller heuristic, empty-key purge, performance score or
corruption-repair claim. Microsoft's [Registry Editor guidance](https://support.microsoft.com/en-us/windows/experience/system-configuration-tools-in-windows)
warns about the impact of incorrect changes and recommends backups. Oxbox's narrow scope and backup do
not make arbitrary registry edits safe or establish that a missing application entry caused a problem.

### Browser and application review

Browser Review supports known Chrome/Chromium, Edge, Brave and Firefox profile layouts. The inventory
shows bounded names/IDs/versions and available permission/state metadata. Chromium-family enabled state
may be unknown; verify it in the browser. Review does not read history, cookie/password/account databases,
run extension code, edit the profile or claim that an extension is malicious. Use the browser's own
controls to disable or remove an extension.

The existing Windows Uninstall Apps path preserves the registered-uninstaller review and separate
leftover-path review. The new macOS/Linux source paths have these limits; native inventory/removal
acceptance is still pending:

- macOS inventories immediate `.app` children of `/Applications` and `~/Applications`. Reviewed Trash
  is limited to current non-root user-owned, unprotected bundles; Apple/system names, links and root-owned
  applications are refused. Loaded jobs, saved application data and signatures are not managed.
- Linux inventories standard `.desktop` metadata read-only, without executing `Exec`. Only current-user
  regular, single-link `.AppImage` files directly within `~/Applications` are eligible for reviewed Trash.
  System packages remain the package manager's responsibility; no package command or download is run.

Discovery is nonrecursive and bounded to 4,096 entries, selected removal to 256 items and desktop-entry
text to 64 KiB. Apply preflights the complete selection, then checks each item again. Cancellation stops
later requests; earlier items may already be in recoverable Trash. These checks do not make the final
filesystem check/use interval atomic against a concurrent change.

Source/platform and native acceptance are recorded separately in [VALIDATION.md](VALIDATION.md).

### File Shred: selected files, permanent consequences

Ordinary cleanup uses Trash. File Shred is the explicitly requested exception: review every selected
path and byte count, acknowledge the storage limits, type **DELETE**, then confirm. It makes one streamed
zero-overwrite pass, synchronizes the file and removes that selected file. It refuses folders, recursive
targets, protected roots, links/reparse redirects, hard links and unsupported/remote filesystems.

**Cancellation cannot restore overwritten bytes or files already processed.** There is no Undo, automatic
run, elevated operation, full-drive overwrite or free-space wipe. SSD/flash wear levelling, copy-on-write
filesystems, snapshots, journaled data, backups and other copies mean a file-only operation cannot
guarantee erasure. Windows uses an owned delete handle; Unix validation/unlink is not atomic against
a concurrent filesystem change. This is not certified secure media disposal. NIST's
[media-sanitization guidance](https://csrc.nist.gov/pubs/sp/800/88/r2/final) describes a separate program of
media-appropriate techniques and controls; Oxbox does not claim compliance or certification from this tool.

## Comparison scope and exclusions

CCleaner's public [Windows product policy](https://www.ccleaner.com/legal/products-policy) describes
cache/browser cleanup, startup and extension management, disk analysis, application removal and other
maintenance categories. Its [Mac product page](https://www.ccleaner.com/ccleaner-mac) also describes
duplicates, startup and application-management functions. Those primary pages informed the category
comparison below. Oxbox is an independent clean-room implementation; no proprietary code, artwork,
interface or copy is reused, and equivalent category names do not imply identical coverage.

| Public product category | Oxbox mapping | Difference or exclusion |
|---|---|---|
| Temporary/browser cleanup | Clean Drive and Maintenance Review | Known cache leaves and explicit review/Trash; no history/password/cookie-database cleaning. |
| Duplicate and disk analysis | Find Duplicates and Storage Analyzer | Local content matches/metadata reports; no cloud-drive or email access, photo-quality scoring or automatic deletion. |
| Startup management | Startup Manager | Current-user records with recovery only; no services/system-wide disabling or performance guarantee. |
| Browser add-on management | Browser Review | Read-only supported metadata; changes stay in browser controls. |
| Application removal | Uninstall Apps | Reviewed OS-specific/user-owned scope; no universal remover or package-manager mutation. |
| Registry cleaning | Registry Review on Windows | Conservative HKCU App Paths default values only; no broad repair/optimization sweep. |
| Scheduled maintenance | Review reminders through Alarm | Reminder only; no unattended deletion, background cleaner or browser-close cleanup trigger. |
| Wiping | File Shred | Selected regular local files, explicit permanent confirmation; no whole-drive/free-space wipe or guaranteed media erasure. |
| Driver/software updates, cloud features, antivirus and app-suspension optimization | Not implemented | No updater downloads, cloud integration, antivirus engine or automatic app sleeping is introduced. |

## What remains to verify

Generated-file, fake-backend and headless tests cover models and guarded effects. They do not certify
real cache cleanup, startup/registry changes, irreversible file processing, live browser profiles,
native dialogs or macOS/Linux application removal. Test those on a consenting disposable machine,
recording the exact source/build, OS, selected paths, expected/actual outcome and recovery result.
No synthetic health score or blanket “everything works” claim replaces that evidence.

Follow [TESTING.md](TESTING.md), [SECURITY.md](../SECURITY.md), the
[implementation task](../tasks/maintenance-and-productivity.md) and the pending release checklist.
Publishing the documentation or website does not publish the installer draft or close its clean-machine,
hardware, accessibility, privacy, signing or independent human-review gates.

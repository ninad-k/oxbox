# Security review — 2026-11-22

This is a source and regression-test review by an AI agent working separately from the native UI,
privacy and installer lanes. It is **not an independent human security attestation**, penetration test,
whole-dependency proof, or approval to publish a release. The required human review remains open.
All 49 catalogue routes are preserved. No live user Trash, uninstall, device, display or power operation
was performed; process fixtures never access devices or the network. Read-only upstream metadata and
advisory lookups are research, not production runtime-network acceptance.

## Scope and method

### Maintenance expansion `c30a179`

Three parallel implementation/review lanes and the integrator added generated/fake-effect coverage for
rename/recovery, permanent file processing, current-user startup/registry changes, bounded local reports,
image flattening, text cleanup and presets. Reproduced fixes cover real cleanup file identity, per-item
browser activity, immutable recovery bytes, shared resize bounds, exact own-startup delegation on all
three OSes and post-Quit persistence admission. No new direct dependency or application network path
was added. Windows **279 core / 112 native** and all-platform strict CI passed; live destructive effects,
adversarial final filesystem races and independent human review remain unaccepted. The
[maintenance guide](MAINTENANCE.md) documents exact scopes, backups and SSD/cancellation limitations;
the [dated receipt](verification/2026-11-22-maintenance-workflows.md) distinguishes tests from actual GUI evidence.

### Preserved baseline review

Inspected shared filesystem/path guards, duplicate scanning and cleanup, ZIP/TAR/7z extraction limits,
cache cleanup, exclusive media outputs, image resizing, encryption boundaries, local provider discovery,
FFmpeg/Tesseract argument construction and child ownership. Inspected the native file confirmation and
persistence boundaries, release/build scripts, frozen dependency inputs and draft-release restrictions.
The separate privacy lane owns the encryption, passphrase-widget and local-provider path changes
described in [PRIVACY_ACCEPTANCE.md](PRIVACY_ACCEPTANCE.md).

For new demonstrated logic defects, tests were written and observed failing before implementation.
The integrator runs shared Cargo commands to avoid Windows executable/link locks. Evidence names below
refer to ignored local `target/` logs, not portable release artifacts. Final integrated checks and actual
production-binary observations belong in [VALIDATION.md](VALIDATION.md) and [RUNTIME_PRIVACY.md](RUNTIME_PRIVACY.md).

## Findings and remediations

Severity describes the demonstrated Oxbox behavior, not a published vulnerability classification.

| Finding | Demonstrated trigger and impact | Remediation and evidence |
|---|---|---|
| Medium: stale duplicate cleanup could remove the last matching copy | Scan a duplicate group, then remove or change its unselected keeper before confirming the old selection. The previous native cleanup sent the selected paths to generic Trash without checking the keeper again. Selecting all copies or a path outside the reviewed group was also accepted by the original core effect boundary. | `duplicates::trash_selected` now validates the complete selection, requires an unselected member in each affected group, rechecks regular-file metadata and streams every selected/kept member through the original full BLAKE3 fingerprint before any Trash call. A final metadata pass and cancellation check precede the effect. The native form passes the original in-memory scan and only the confirmed paths. Initial tests: two failures/one pass in `security-duplicates-red.log`; all four final Trash regressions pass in the integrated suite. |
| High: a provider descendant could outlive the timeout and hold shutdown/cancellation | A local wrapper starts a self-terminating descendant inheriting its stdout/stderr, then exits or exceeds a 350 ms timeout. Killing/reaping only the wrapper left pipe-reader joins waiting about 4.12 seconds; a nonterminating descendant could wait indefinitely. | One managed-process wrapper now covers text providers, FFmpeg jobs and recordings. Windows creates a suspended child, assigns it to a non-breakaway kill-on-close Job Object, then resumes its uniquely identified primary thread. Assignment/resume failures kill and reap the suspended child. Unix creates a process group before exec and uses typed `waitid(WNOWAIT)` so the leader stays unreaped until its group is signalled; group identity is consumed before any later reaping. Ordinary descendants are stopped before pipe-reader joins on success, timeout, cancellation and Drop. Both real inherited-pipe tests failed first (`security-process-red.log`), then passed together in 0.58 seconds on Windows (`security-process-green.log`), each asserting under two seconds. macOS/Linux execution remains a CI/runtime gate. |
| Medium: provider path validation could contact a Windows remote filesystem | Canonicalizing an explicit UNC or mapped-network input/output/provider path could reach a remote filesystem before enforcing the offline provider boundary. No live remote path was accessed to reproduce this. | The privacy lane added syntactic local-drive checks before filesystem access, drive-type checks, reparse-component rejection and validation after canonicalization. Provider lookup, media inputs, staging and final output parents use that shared guard. Pure/fake regressions and remaining Unix mounted-filesystem limits are documented in the privacy record. |
| Resource exhaustion: untrusted age work factors | A syntactically valid age scrypt header can request an excessive work factor before password authentication. Header-byte limits alone do not bound KDF cost. | The privacy lane selected fixed encryption work factor 18 and a decryption maximum of 20 through the supported age APIs, with a clear excessive-work error. Its complete header regression was observed failing before the cap. Factors above the policy are intentionally unsupported; a permitted KDF still needs substantial memory/CPU. See the privacy record and final integrated checks. |
| Secret retention: password widget undo history | Masked egui password fields still stored plaintext undo snapshots after the application's draft string was cleared. | The privacy lane clears and stores the widget undo state each frame for both passphrase fields. A headless sentinel regression observed the old restorable value before the fix. Clearing UI drafts is not a memory-forensic zeroization guarantee; the selected age secret type protects its owned passphrase separately. |

### Residual filesystem and provider limits

The duplicate preflight is not an atomic transaction across multiple files. A concurrent process can still
replace or modify a pathname after its last validation and before the OS Trash operation. Generic path
guards and canonicalization have similar check/use limits. They prevent ordinary stale-scan mistakes and
reject known symlink/reparse paths; they do not establish hostile concurrent-filesystem isolation.
Trash remains separately confirmed and never becomes permanent deletion.

Provider process ownership is lifecycle management, not an executable sandbox or a promise of provider
network silence. A user-managed provider runs with the user's privileges. A deliberately hostile Unix
provider can leave its process group; OS access/termination failures can prevent cleanup. Windows Job
Objects manage associated CreateProcess descendants and do not cover every external broker/service.
Trust and update FFmpeg/Tesseract installations separately. The reviewed fixed demuxer/protocol arguments
reduce remote/manifest exposure; they do not replace codec/parser security updates or production observation.
[Windows Job Object semantics](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

The media exclusive-copy fallback can leave a partial **new** destination when cancellation or an I/O error
interrupts publication. The original is preserved and an existing output is never overwritten. This is an
existing documented recovery behavior, not a completed atomic-publication guarantee.

## Archive, output and persistence checks

No additional demonstrated bypass was found in this review of the existing archive guards. ZIP central
directory/count preflight precedes parser allocation, with a 64 MiB directory and 100,000-entry ceiling.
TAR processing rejects traversal, links, special/sparse entries and nonempty directory payloads, and bounds
extended metadata. 7z stored and decoded table metadata is capped at 128 KiB; its validated plain table is
frozen for the final reader, with a cumulative 128 MiB decoder-model budget, limited coder graph and
single-thread decoding. Extraction also enforces a total output/expansion budget and creates a new output
directory. These are conservative compatibility policies, not proof against every third-party decoder bug.
Existing malicious-header, traversal, symlink and bounded-reader fixtures remain relevant regression checks.

New archive/image/encryption/media outputs use shared filename/path guards and exclusive creation.
Windows alternate streams, device names and reparse paths are rejected at the applicable boundaries.
Native cleanup confirmations retain reviewed selections; no new automatic Trash or uninstaller execution
was introduced. Shared persistence remains size-bounded/atomic and preserves legacy import sources. This
inspection found no further concrete persistence-loss defect; it did not fuzz every storage schema.

## Dependency and supply-chain assessment

A fresh `cargo audit --json` invocation on 2026-11-22 exited successfully with **zero vulnerability
entries, eight unmaintained warnings and one GLib unsoundness warning** across the locked workspace.
The fetched advisory database contained 1,555 advisories at commit
`55e1c3dfbd834237a05bbbf7b50ba3b5be27b662`, last updated 2026-11-21. Local outputs are
`target/security-cargo-audit.json` and `target/security-cargo-audit.stderr.txt`. Successful exit and a zero
vulnerability-entry count do not close the informational unsoundness or maintenance findings.
[DEPENDENCIES.md](DEPENDENCIES.md) retains their target paths and follow-up; no ignore list was added.

**GLib remains unresolved for Linux.** On 2026-11-22 the primary crates.io API still listed 0.18.5 as the
newest published 0.18 release. GTK 0.18.2's published dependencies require GLib/GIO `^0.18`; newer GLib
families cannot replace that binding by an ordinary compatible update. Inspection of the locked 0.18.5
source and official 0.18 branch still found the affected immutable out-pointer. The upstream fix is merged
as `05dff0ee696f9bcd8617cd48c4b812d046d440cb` and is published in GLib 0.20.0 and later. No compatible
published patch was found. Sources: [GLib metadata](https://crates.io/api/v1/crates/glib),
[GTK dependency metadata](https://crates.io/api/v1/crates/gtk/0.18.2/dependencies),
[upstream correction](https://github.com/gtk-rs/gtk-rs-core/pull/1343),
[RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).

Concrete choices are a coordinated GTK-family/provider migration with platform behavior retesting, or an
explicitly approved narrow upstream-source backport with pinned provenance, notices, regression tests and
transparent audit treatment. Neither was performed here. A second direct newer GLib dependency does not
repair the existing binding. No direct call to the affected iterator was identified in Oxbox; that narrows
known exposure without proving framework-wide unreachability. Linux acceptance remains open.

The only new direct package in this fix is the explicitly user-approved Unix-only `libc =0.2.191`, already
present transitively in the lockfile, for typed `waitid`/process-group APIs. The lock change adds that core
dependency edge without upgrading resolved packages. Windows reuses the existing windows-sys version with
additional process Job/Threading/ToolHelp features. Approval is recorded in
[ADR 0004](adr/0004-runtime-offline-boundary.md).

The release/build scripts use frozen inputs, inspect the native graph/resource layout and retain an
unpublished-draft boundary. All 25 existing GitHub Action references are now pinned to exact commits resolved
from the respective upstream GitHub repositories, with their original version comments and jobs preserved.
Weekly GitHub Actions Dependabot checks track future reviewed updates. Source inspection cannot attest to
hosted runner integrity, signing identity or every fetched build tool. Third-party notices
and human legal/security review remain separate acceptance gates.

## Reproduction and completion status

Run the focused regressions from the repository root with the selected Rust 1.95 toolchain:

```text
cargo test -p oxbox-core --locked tools::duplicates::tests::trash_
cargo test -p oxbox-core --locked tools::media_provider::tests::process_ -- --nocapture
cargo test -p oxbox-core -p oxbox-native --locked
cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo audit --json
```

The integrator's post-fix Windows run with exact Rust 1.95 passed **208 core tests and 64 native tests**,
with zero failures or ignored tests, strict all-target Clippy and formatting checks. The core suite took
106.16 seconds and the native suite 0.31 seconds; those are test-run observations, not production
performance measurements. Local evidence is `target/review-rust-tests.log` and
`target/review-clippy.log`. This includes all four duplicate cleanup regressions, both process containment
regressions and the privacy/accessibility lane's merged tests. Cross-platform CI, production provider
cancellation/recording finalization, full runtime privacy/DNS coverage, clean-machine installer behavior
and independent human security/legal attestations remain open until their actual evidence is recorded.
No manual 49-tool acceptance row is closed by this agent review or its fake/headless fixtures.

## Post-transform original-file safety follow-up

The functional review found that optional encryption/decryption Trash retained only paths: an original
could be edited after the successful transform, or the saved output could disappear, before its removal
review. The new transient receipt binds BLAKE3 hashes of the **actual read/write streams**, file IDs, sizes
and modification stamps. After explicit review, cancellable streaming checks compare both files and recheck
their stamps before the shared Trash helper is called. Receipts contain no passphrase and are not serialized.

Three generated/fake-effect guards cover changed/replaced originals, changed/missing output, confirmation,
cancellation and a same-size/time rewrite of an already-read prefix. The existing real age test additionally
verifies both hashes and valid/refused fake cleanup. All pass in the 221-core-test Windows run. No live Trash
was invoked. This narrows ordinary stale-backup races; it is not an atomic lock against concurrent filesystem
mutation and does not close the independent adversarial filesystem/human review gate.

## Bounded second feature pass

Source `67f0715` adds captured-permit checks for delayed settings requests, actual Hidden Files readback
validation and a desktop restoration guard bound to HWND, PID, thread, class and current desktop membership.
The Win32 identity check and mutation are separate calls, so concurrent external destruction and handle
reuse remain possible. Generated/fake regressions passed on Windows, macOS ARM and Linux; no live
OS settings or desktop effect was performed. File-drop and QR/OCR result changes also prevent stale
input/result associations in the native forms. A dependency-free Common Controls manifest retains
`asInvoker` and `uiAccess=false`; it does not request elevation.

[CI 44994827768](https://github.com/ninad-k/oxbox/actions/runs/44994827768) passes all five jobs, including
strict Clippy and production builds on all three systems. The
[dated 49-tool verification](verification/2026-11-22-native-feature-pass.md) distinguishes actual Windows
default-form/selected lifecycle observations from fake effects and open manual cases. The dependency
audit still reports the documented Linux GLib unsoundness advisory and maintenance warnings. Human
security/license review, adversarial filesystem, hardware/provider and clean-machine gates remain open;
the release stays unpublished.

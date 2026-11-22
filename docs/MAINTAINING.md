# Maintaining and reviewing Oxbox

Use this workflow for contributions to the native migration branch, `codex/native-renderer`. It does not
grant publication permissions or replace the [release checklist](RELEASE.md).

## Triage

Check whether a report concerns the native app, retained Tauri host, browser preview, static site or installer.
Ask for a commit and OS/session before inferring a current native regression from a legacy screenshot.
Use the canonical tool ID and a small generated fixture. Redirect security details to
[private vulnerability reporting](../SECURITY.md), and keep clipboard contents, recordings, passphrases,
usernames and personal paths out of public logs. Apply the [code of conduct](../CODE_OF_CONDUCT.md) consistently.

Good initial contributions include a bounded reproducible bug, a safer error path, missing lifecycle coverage,
an incorrect guide, or a documented native acceptance case. Do not advertise acceptance gaps as missing
implementations or ask contributors to rebuild an already implemented tool.

## Review a pull request

1. Confirm the intended base branch and focused scope. Read the relevant specification and before/after
   behavior. Require original code/assets and appropriate third-party notices; contributions use MIT OR Apache-2.0.
2. Review boundaries: core logic stays toolkit-neutral; forms use typed inputs and real actions; no unapproved
   dependencies/network access, hidden effects or generic mock forms. A dependency change needs explicit
   approval and the maintenance/license/size/privacy assessment in [DEPENDENCIES](DEPENDENCIES.md).
3. Inspect user-data safety and lifetime: exclusive outputs, bounded streaming, confirmed Trash, transactional
   storage, stale-result invalidation, cancel checks and owned worker/child cleanup. Check Back, hide and Quit
   independently; stopping an explicit background session is different from abandoning a view task.
4. Inspect meaningful regression evidence, strict clippy/fmt and required CI. Do not accept disabled checks,
   ignored regressions or tests that touch real power, desktop, private clipboard or devices without consent.
5. Review visible labels, focus, errors and announcements. Headless names/contrast checks supplement actual
   keyboard, screen-reader, DPI and hardware observations; record outstanding cases explicitly.
6. Verify canonical catalogue, documentation, generated feature index and site agree. Check current evidence
   links and avoid turning finite runtime samples into an unconditional offline/security claim.

Use the PR template to record checks that apply and explain ones that were not run. A green build is necessary
but does not close manual release gates. Update the changelog for user-visible behavior. Coordinate shared
files and Cargo builds when several contributors work in parallel.

## Prepare a candidate without publishing it

Follow [RELEASE](RELEASE.md), [installer acceptance](INSTALLER_ACCEPTANCE.md) and
[validation](VALIDATION.md). Freeze the functional source, use Rust 1.95 and locked dependencies, generate
notices, build native bundles and record hashes. Validate actual downloaded draft assets against local
artifacts and the exact executable source. Preserve the distinction between source inspection, payload
extraction and real clean-machine install/login/upgrade/uninstall tests.

Keep the release a draft while any required gate remains open. Do not silently replace an installer with one
from a different source commit or reuse old privacy/manual evidence as proof for a changed binary. Retain
artifact/source mappings and explain the coverage limits. After an authorized publication, update release
links and website download calls to action to the verified public assets; the current site intentionally
has no public installer download. See the existing release workflow rather than inventing another pipeline.

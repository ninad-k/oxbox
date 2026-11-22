# Security policy

Oxbox runs locally, handles user files and can trigger OS actions (lock, sleep, shutdown), so security reports matter.

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private
[security advisory form](https://github.com/ninad-k/oxbox/security/advisories/new).
Include steps to reproduce, affected version and OS. We aim to reply within 7 days.
Use the exact source commit for development builds and identify the native or retained legacy host.
Share a minimal generated fixture where possible; do not post passphrases, clipboard contents or private
recordings in an issue. The current native candidate is a draft, not a supported public stable release.

## Scope

In scope: anything that lets an untrusted file or other process make Oxbox read, write, delete or execute
something the user did not ask for; unsafe archive handling (path traversal, decompression bombs); crypto,
local activation, persistence, provider-process ownership and OS-action authorization weaknesses.
The legacy Tauri host also has web-content/IPC boundaries. State which host your report concerns.

## Design commitments

- Tool commands are local by default and implement no Oxbox app telemetry. Currency refresh is a separately opted-in, fixed-URL HTTPS download
  of public ECB rates, tracked in [issue #1](https://github.com/ninad-k/oxbox/issues/1). No amounts, selected
  currencies, clipboard data, recordings, passphrases, or files are uploaded by Oxbox commands.
- The selected native eframe host embeds no browser and calls core APIs directly. CSP and minimal Tauri
  capabilities apply to the retained legacy comparison host (`src-tauri/capabilities/`), not native widgets.
- Destructive actions require confirmation and use the Recycle Bin/Trash by default.
- Outputs use exclusive creation; archive entries cannot escape a newly-created destination.
- Extraction enforces byte/entry/metadata limits; 7z has bounded metadata and conservative per-block model budgets.
- Cleanup uses internal location allow-lists and revalidates reviewed budgets before moving files.
- Clipboard recording requires an explicit start, is bounded in memory, and is cleared on Quit.
- Passphrases are not intentionally logged or persisted. Core uses zeroizing secret storage; native password
  drafts and their edit undo history clear on leave/hide. UI strings/allocators and OS memory do not provide
  a forensic erasure guarantee. Age passphrase decoding uses a documented work-factor ceiling; see
  [privacy acceptance](docs/PRIVACY_ACCEPTANCE.md).
- Media providers are explicit local executables. File conversions restrict FFmpeg input protocols/demuxers;
  recording starts require confirmation and managed processes are stopped/reaped on Quit. Oxbox does not download providers.
- Window effects retain ownership, display changes have timed rollback, and app-closing/eject/uninstall actions
  require review. Registered uninstall commands are re-read before execution and are never passed through a shell.

These are implementation commitments, not a claim of an external audit. Human file-tool review and platform
release validation remain tracked in [docs/VALIDATION.md](docs/VALIDATION.md).
Earlier legacy Windows QA observed independent WebView2 HTTPS traffic whose purpose was not identified.
The selected native dependency graph removes that embedded browser. Bounded native Windows production
observations saw no owned TCP/UDP endpoints in their recorded intervals; this is not a universal network
guarantee. [Runtime privacy findings](docs/RUNTIME_PRIVACY.md) identify hashes, scenarios, sampling gaps,
DNS/broker exclusions and pending cross-platform/provider checks. Dependency advisories, human security
review and other [release gates](docs/RELEASE.md) remain explicit rather than being treated as certification.

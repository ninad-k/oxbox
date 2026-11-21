# Security policy

Oxbox runs locally, handles user files and can trigger OS actions (lock, sleep, shutdown), so security reports matter.

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private
[security advisory form](https://github.com/ninad-k/oxbox/security/advisories/new).
Include steps to reproduce, affected version and OS. We aim to reply within 7 days.

## Scope

In scope: anything that lets a web page, file or other process make Oxbox read, write, delete or execute something
the user did not ask for; IPC permission bypasses; unsafe handling of untrusted archives (path traversal, zip bombs);
weaknesses in Encrypt Files.

## Design commitments

- Tool commands are local by default and implement no Oxbox app telemetry. Currency refresh is a separately opted-in, fixed-URL HTTPS download
  of public ECB rates, tracked in [issue #1](https://github.com/ninad-k/oxbox/issues/1). No amounts, selected
  currencies, clipboard data, recordings, passphrases, or files are uploaded by Oxbox commands.
- Strict Content Security Policy and minimal Tauri capabilities (`src-tauri/capabilities/`).
- Destructive actions require confirmation and use the Recycle Bin/Trash by default.
- Outputs use exclusive creation; archive entries cannot escape a newly-created destination.
- Extraction enforces byte/entry/metadata limits; 7z has bounded metadata and conservative per-block model budgets.
- Cleanup uses internal location allow-lists and revalidates reviewed budgets before moving files.
- Clipboard recording requires an explicit start, is bounded in memory, and is cleared on Quit.
- Passphrases are not intentionally logged or stored; Rust uses zeroizing secret storage. JavaScript strings
  cannot provide a guarantee of immediate memory erasure.
- Media providers are explicit local executables. File conversions restrict FFmpeg input protocols/demuxers;
  recording starts require confirmation and managed processes are stopped/reaped on Quit. Oxbox does not download providers.
- Window effects retain ownership, display changes have timed rollback, and app-closing/eject/uninstall actions
  require review. Registered uninstall commands are re-read before execution and are never passed through a shell.

These are implementation commitments, not a claim of an external audit. Human file-tool review and platform
release validation remain tracked in [docs/VALIDATION.md](docs/VALIDATION.md).
Windows release QA observed independent WebView2 HTTPS traffic. Its purpose was not identified, and the strict
whole-process offline requirement remains unmet; [runtime privacy findings](docs/RUNTIME_PRIVACY.md) distinguish
observations, SDK documentation and the public-release acceptance gap.

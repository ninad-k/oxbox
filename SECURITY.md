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

- No network access and no telemetry.
- Strict Content Security Policy and minimal Tauri capabilities (`src-tauri/capabilities/`).
- Destructive actions require confirmation and use the Recycle Bin/Trash by default.

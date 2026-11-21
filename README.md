<p align="center">
  <img src="static/oxbox.svg" width="96" height="96" alt="Oxbox logo" />
</p>

<h1 align="center">Oxbox</h1>

<p align="center">
  A free, open-source toolbox of one-click desktop utilities for Windows, macOS and Linux.<br/>
  Built in Rust with Tauri. Local tools. No ads, no subscriptions, no Oxbox app telemetry.
</p>

<p align="center">
  <a href="https://github.com/ninad-k/oxbox/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/ninad-k/oxbox/actions/workflows/ci.yml/badge.svg" /></a>
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" />
  <img alt="Status: early development" src="https://img.shields.io/badge/status-early%20development-orange" />
</p>

---

Oxbox lives in your system tray (menu bar on macOS). Click it, search or browse by category, and run the tool
you need: keep the PC awake, hash a download, find duplicate files, make a QR code, check what is eating your CPU.

> **Status: implementation, awaiting release acceptance.** The 20-tool v0.1 catalogue and promoted Phase 5 tools have core logic, command adapters,
> typed APIs and launcher views. Automated checks and manual release gates are recorded in
> [VALIDATION.md](docs/VALIDATION.md). “Ready” below means the implementation is available in source;
> it does not imply a published or independently audited release. Contributions welcome.

> **Windows runtime privacy:** release QA observed background HTTPS from the embedded WebView2 runtime.
> Strict zero-network acceptance remains open; see the [runtime privacy findings](docs/RUNTIME_PRIVACY.md).

## Tools

| Category | Tool | What it does | Status |
|---|---|---|---|
| Productivity | Transform Text | UPPER, lower, Title, Sentence, camelCase, PascalCase, snake_case, kebab-case, CONSTANT_CASE | **Ready** |
| | Unit Converter | Length, mass, temperature, data size, time | **Ready** |
| | Clipboard History | Recent copied text and images, paste any again | **Ready** |
| Time | Timer & Stopwatch | Countdown and stopwatch with laps | **Ready** |
| | World Time | Current time in cities you choose | **Ready** |
| | Break Time | Reminders to take regular breaks | **Ready** |
| Files | Verify Checksum | SHA-256, SHA-1, MD5 and BLAKE3; compare with a published hash | **Ready** |
| | Find Duplicates | Identical files, even with different names | **Ready** |
| | Archive & Unarchive | Create zip/tar.gz; extract zip, tar, 7z | **Ready** |
| | Clean Drive | Temp and cache files, moved to the Recycle Bin/Trash | **Ready** |
| Screen Capture | Screenshot | Full screen, window or area to file and clipboard | **Ready** |
| Images & Media | QR & Barcode | Text or link to QR code / barcode image | **Ready** |
| | Resize Images | Batch resize and format conversion | **Ready** |
| System | System Monitor | CPU, memory, disk and top processes | **Ready** |
| | Network Usage | Live upload/download speed and totals | **Ready** |
| | Do Not Sleep | Keep the computer and display awake | **Ready** |
| | Sleep Timer | Sleep or shut down after a countdown | **Ready** |
| Privacy & Security | Lock Screen | Lock the computer instantly | **Ready** |
| | Hide Desktop | Hide desktop icons for presentations | **Ready** |
| | Encrypt Files | Passphrase encryption with [age](https://age-encryption.org) | **Ready** |

The user-promoted Phase 5 implementation expands the launcher to **48 tools**, including window/app controls,
presentation tools, alarms, media recording/conversion, OCR, barcode reading, color sampling, and currency.
See [the expanded tool guide](docs/ROADMAP_TOOLS.md) for platform support and local-provider requirements.

<p align="center"><img src="docs/images/launcher-dark.png" width="900" alt="Oxbox native Windows launcher in dark appearance with 48 tools" /></p>
<p align="center"><img src="docs/images/launcher-light.png" width="900" alt="Oxbox native Windows launcher in light appearance with 48 tools" /></p>

## Build from source

Prerequisites: [Rust](https://rustup.rs) 1.93+, [Node.js](https://nodejs.org) 22.17+, [pnpm](https://pnpm.io),
and the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/ninad-k/oxbox.git
cd oxbox
pnpm install --frozen-lockfile
pnpm tauri dev      # run with hot reload
pnpm tauri build    # build installers into target/release/bundle
```

Prebuilt installers will be published on the [Releases](https://github.com/ninad-k/oxbox/releases) page from v0.1.
Early builds are not code-signed, so Windows SmartScreen and macOS Gatekeeper will warn on first launch.

## How it is built

```
┌──────────────────────── Oxbox app (Tauri 2) ────────────────────────┐
│  Svelte 5 UI (src/)            Rust shell (src-tauri/)              │
│  launcher, search, tool views  tray, windows, #[tauri::command]s    │
│            │   typed IPC (src/lib/api.ts)   ▲                       │
│            └────────────────────────────────┘                       │
│                               │                                     │
│                 oxbox-core (crates/oxbox-core/)                     │
│      all tool logic, pure Rust, unit-tested, no UI dependencies     │
└─────────────────────────────────────────────────────────────────────┘
```

- [SPEC.md](SPEC.md): goals, scope, boundaries
- [tasks/plan.md](tasks/plan.md): phased roadmap with acceptance criteria
- [AGENTS.md](AGENTS.md): conventions and the step-by-step recipe for adding a tool (for humans and AI agents)
- [docs/adr](docs/adr): architecture decisions
- [docs/USAGE.md](docs/USAGE.md): every tool, shortcuts, safety limits and platform notes
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): IPC groups, background services and cancellable jobs
- [docs/TESTING.md](docs/TESTING.md): automated commands and manual acceptance matrix
- [docs/RELEASE.md](docs/RELEASE.md): installer and publication checklist
- [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md): dependency review and primary references
- [docs/ROADMAP_TOOLS.md](docs/ROADMAP_TOOLS.md): Phase 5 tools, providers and network opt-in
- [CHANGELOG.md](CHANGELOG.md): user-facing changes

## Contributing

Pick a planned tool from [tasks/todo.md](tasks/todo.md), follow the recipe in [AGENTS.md](AGENTS.md),
and open a pull request. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Privacy

Oxbox implements no app telemetry. Tool commands work locally; currency rates are the sole intentional network feature:
after a separate explicit opt-in, a manual refresh downloads public ECB reference data. Amounts and selected
currencies stay local. The reviewed exception is [issue #1](https://github.com/ninad-k/oxbox/issues/1).
The embedded Windows WebView2 runtime was observed contacting Microsoft independently. Its request purpose
was not identified; strict whole-process offline acceptance remains open. See [runtime privacy findings](docs/RUNTIME_PRIVACY.md).
Clipboard recording starts paused, stays in memory, and is cleared on Quit. Saved settings and tool outputs
stay on your computer. File cleanup goes to the Recycle Bin/Trash after confirmation; outputs preserve originals.
See the [usage guide](docs/USAGE.md) for limits and the [security policy](SECURITY.md) for private reports.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Trademarks

Oxbox is an independent open-source project inspired by all-in-one utility suites such as Parallels Toolbox.
It is not affiliated with, endorsed by or sponsored by Parallels International GmbH. "Parallels" and
"Parallels Toolbox" are trademarks of their respective owner. No proprietary code or assets are used in Oxbox.

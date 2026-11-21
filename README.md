<p align="center">
  <img src="static/oxbox.svg" width="96" height="96" alt="Oxbox logo" />
</p>

<h1 align="center">Oxbox</h1>

<p align="center">
  A free, open-source toolbox of one-click desktop utilities for Windows, macOS and Linux.<br/>
  Built in Rust with Tauri. Offline. No ads, no subscriptions, no telemetry.
</p>

<p align="center">
  <a href="https://github.com/ninad-k/oxbox/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/ninad-k/oxbox/actions/workflows/ci.yml/badge.svg" /></a>
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" />
  <img alt="Status: early development" src="https://img.shields.io/badge/status-early%20development-orange" />
</p>

---

Oxbox lives in your system tray (menu bar on macOS). Click it, search or browse by category, and run the tool
you need: keep the PC awake, hash a download, find duplicate files, make a QR code, check what is eating your CPU.

> **Status: early development.** The app framework (tray, launcher, search, favourites, tool views) and the first
> two tools are done. The rest are planned in phases; see [the roadmap](tasks/plan.md). Contributions welcome.

## Tools

| Category | Tool | What it does | Status |
|---|---|---|---|
| Productivity | Transform Text | UPPER, lower, Title, Sentence, camelCase, PascalCase, snake_case, kebab-case, CONSTANT_CASE | **Ready** |
| | Unit Converter | Length, mass, temperature, data size, time | Planned (phase 1) |
| | Clipboard History | Recent copied text and images, paste any again | Planned (phase 4) |
| Time | Timer & Stopwatch | Countdown and stopwatch with laps | Planned (phase 1) |
| | World Time | Current time in cities you choose | Planned (phase 1) |
| | Break Time | Reminders to take regular breaks | Planned (phase 3) |
| Files | Verify Checksum | SHA-256, SHA-1, MD5 and BLAKE3; compare with a published hash | **Ready** |
| | Find Duplicates | Identical files, even with different names | Planned (phase 2) |
| | Archive & Unarchive | Create zip/tar.gz; extract zip, tar, 7z | Planned (phase 2) |
| | Clean Drive | Temp and cache files, moved to the Recycle Bin/Trash | Planned (phase 2) |
| Screen Capture | Screenshot | Full screen, window or area to file and clipboard | Planned (phase 4) |
| Images & Media | QR & Barcode | Text or link to QR code / barcode image | Planned (phase 1) |
| | Resize Images | Batch resize and format conversion | Planned (phase 4) |
| System | System Monitor | CPU, memory, disk and top processes | Planned (phase 3) |
| | Network Usage | Live upload/download speed and totals | Planned (phase 3) |
| | Do Not Sleep | Keep the computer and display awake | Planned (phase 3) |
| | Sleep Timer | Sleep or shut down after a countdown | Planned (phase 3) |
| Privacy & Security | Lock Screen | Lock the computer instantly | Planned (phase 3) |
| | Hide Desktop | Hide desktop icons for presentations | Planned (phase 4) |
| | Encrypt Files | Passphrase encryption with [age](https://age-encryption.org) | Planned (phase 4) |

More tools (window manager, OCR, screen recording, colour picker, and others) are listed in the roadmap.

## Build from source

Prerequisites: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+, [pnpm](https://pnpm.io),
and the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/ninad-k/oxbox.git
cd oxbox
pnpm install
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

## Contributing

Pick a planned tool from [tasks/todo.md](tasks/todo.md), follow the recipe in [AGENTS.md](AGENTS.md),
and open a pull request. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Privacy

Oxbox works offline and collects nothing. It does not make network requests.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Trademarks

Oxbox is an independent open-source project inspired by all-in-one utility suites such as Parallels Toolbox.
It is not affiliated with, endorsed by or sponsored by Parallels International GmbH. "Parallels" and
"Parallels Toolbox" are trademarks of their respective owner. No proprietary code or assets are used in Oxbox.

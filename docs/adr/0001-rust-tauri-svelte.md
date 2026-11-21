# ADR 0001: Rust + Tauri 2 + Svelte 5, with logic in a separate core crate

- Status: Accepted
- Date: 2026-11-21

## Context

Oxbox must run on Windows, macOS and Linux, ship as a small installer, start fast into the tray, and call many
OS-level APIs (power, screen capture, clipboard, file system). It is open source and will be built largely by
contributors and AI coding agents, so the structure must be easy to follow and hard to misuse.

Options considered:

| Option | Pros | Cons |
|---|---|---|
| **Rust + Tauri 2** | Native installers for 3 OSes, ~5–10 MB, mature crates for nearly every tool, memory safe, tray/shortcut/updater plugins | Web UI layer (needs Node at build time) |
| Rust + egui/iced/Slint | Single binary, no JS | Plainer UI, more custom widget work, weaker accessibility |
| Go + Wails/Fyne | Simple language | Thinner OS-API ecosystem; more cgo |
| C# + Avalonia | Good cross-platform UI | Bundles .NET runtime (much larger); macOS/Linux APIs need hand-written interop |

## Decision

- **Tauri 2** for the shell (tray, windows, IPC, bundling).
- **Svelte 5 + TypeScript** (SvelteKit in SPA mode) for the UI: small bundles and little boilerplate for ~60 views.
- **`oxbox-core` crate** holds all tool logic with no Tauri dependency, so it is unit-testable and reusable (e.g. a future CLI).
- `src-tauri` contains only thin command adapters, tray and window code.

## Consequences

- Each tool follows the same 4-part shape (core module → command → `api.ts` wrapper → Svelte view + registry entry), documented in `AGENTS.md`.
- Building requires Node + pnpm in addition to Rust.
- WebView differences (WebView2 / WKWebView / WebKitGTK) must be considered for UI features; we use standard CSS only.

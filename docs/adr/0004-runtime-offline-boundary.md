# ADR 0004: Separate the offline requirement from the embedded browser boundary

- Status: **Accepted and implemented — initial native shell feasibility observed; release acceptance remains open**
- Date: 2026-11-21
- Scope: native migration authorized by the user on 2026-11-21, using exact eframe 0.37.0 and Rust 1.95

The user explicitly approved this migration and its listed dependencies, required all 49 tools to be
preserved, and required parallel agents and an unpublished release until verification. The implementation
branch is `codex/native-renderer`; [the migration checklist](../../tasks/native-migration.md) records evidence.
This approval permits the native implementation and its dependency graph; it does not waive any acceptance
gate or authorize global OS/security changes. The legacy Tauri implementation remains available for comparison.

## Finding

The current supported WebView2 controls do **not** establish zero network activity for the application and
its browser descendants. Keep the release a draft and the strict acceptance gate open. The explicit native approval supersedes the runtime choice in [ADR 0001](0001-rust-tauri-svelte.md) for the
selected entrypoint. The old host and its validation remain available for comparison.

The [runtime investigation](../RUNTIME_PRIVACY.md) records working SDK resource interception alongside
browser-owned ECS, component-update and SmartScreen requests. Microsoft documents required diagnostics
outside the developer's overall control, plus narrower controls for crash reporting and SmartScreen.
Retain security protections; disabling reputation checks is not the selected solution.
[Microsoft privacy documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy).

## Historical embedded-browser controls and their limits

| Approach | Assessment |
|---|---|
| SDK web-resource filters, including worker sources | Useful for app content. The documented boundary is requests for web resources, not every browser-owned service or speculative connection. Keep the existing filter; do not promote its counters to a process-wide guarantee. |
| Isolated profile, extensions/SSO controls, custom crash reporting | Useful supported controls already implemented. They control specific behavior; a dedicated profile does not disable the browser's service processes. |
| Per-app proxy / network-disable option | No documented supported whole-process disable API was found in the reviewed environment/API references. The documented `proxy-server` mechanism is a browser flag and affects HTTP/HTTPS only. It is not an enforceable all-protocol boundary. |
| Fixed Version WebView2 | Controls runtime distribution/update timing. It does not document elimination of required diagnostics, ECS or SmartScreen. It also adds over 250 MB of runtime binaries and a separate security-update responsibility. |
| DevTools offline emulation or browser flags | Page/network diagnostics are not a production process-isolation boundary. Microsoft explicitly advises against shipping browser flags. No flags or diagnostic controls are proposed for production. |

These conclusions are bounded to the documented SDK surface reviewed on this date, not a claim about every
future WebView2 release. Sources: [request-management scope](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/webresourcerequested),
[environment options](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environmentoptions),
[browser flags and proxy limitations](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags),
[process model](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model),
[Fixed Version distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).

Global Edge/Windows policy, firewall changes, security-disable switches, an app-wide proxy assumption and
replacing one embedded browser with another are not accepted shortcuts to this requirement.

## Selected decision

Implement the user-approved **eframe/egui 0.37.0 native host with Rust 1.95** on codex/native-renderer.
The application runtime is crates/oxbox-native, reusing oxbox-core without an embedded webview. The legacy
Svelte/Tauri implementation stays as a parity reference; no hidden browser is retained in the new host.
All 49 source routes and concrete native forms are implemented, the local native test suite has passed,
and a live Windows decorated launcher with AccessKit names has been observed. These are bounded source,
automation and shell feasibility facts, not full native manual or release acceptance.

Removing the WebView2 service dependency does not prove that the new binary, drivers, OS integrations or
explicitly launched providers never communicate. Production privacy observations, all-platform/hardware
behavior and native installer acceptance remain required. [Upstream egui](https://github.com/emilk/egui).

## Approved dependency selection and historical alternatives

Primary crates.io metadata checked on 2026-11-21 reports **egui 0.37.0 and eframe 0.37.0**, both non-yanked,
MIT OR Apache-2.0, with declared **Rust 1.95**. The 0.36 line also requires 1.95. Version **0.35.0 declares
Rust 1.92**, making it the newest published line compatible with Oxbox's then-current 1.93 minimum before resolving
and checking transitives. During that pre-approval comparison no packages were downloaded or built. The subsequently approved
0.37 native graph has now been built locally; current commands and manifests supersede the unselected alternatives.
[eframe metadata](https://crates.io/api/v1/crates/eframe), [egui metadata](https://crates.io/api/v1/crates/egui),
[0.35 manifest](https://github.com/emilk/egui/blob/0.35.0/crates/eframe/Cargo.toml),
[0.37 workspace minimum](https://github.com/emilk/egui/blob/0.37.0/Cargo.toml).

The selected native renderer configuration is:

```toml
eframe = { version = "=0.37.0", default-features = false, features = ["accesskit", "clipboard", "default_fonts", "system_fonts", "glow", "wayland", "x11"] }
```

Use eframe::egui; no second direct egui dependency is needed. The reviewed 0.35 minimum-preserving option
was not selected. Inspection/debug servers, HTTP clients, remote image loaders and automatic external links
are disabled. Native widgets/charts and core-produced raster previews use the existing APIs; there is no new
SVG parser or egui_extras dependency. Existing Lucide vectors are flattened into catalog/icons.json.

The user also approved promoting the existing locked host packages below. Their versions/features and the
complete resolved graph are recorded in the manifests/lockfile; audit/notices and platform verification remain
release work. The packaging wrapper verifies the native graph excludes browser-host crates and uses the
existing bundler only for distribution, with WebView installation and updater artifacts disabled.

| Approved direct package | Purpose |
|---|---|
| rfd 0.17.2 | Explicit native open/save/confirmation dialogs |
| tray-icon 0.25.1, global-hotkey 0.8.0 | Tray lifecycle and configurable launcher key |
| auto-launch 0.6.0, open 5.4.5 | Login registration and explicit user-requested open actions |
| tauri-winrt-notification 0.8.1 | Windows notification adapter; does not embed a browser |
| mac-notification-sys 0.6.15, notify-rust 4.19.0 | macOS/Linux notifications |
| gtk 0.18.2 (Linux) | Native tray provider/event-loop integration; existing GLib advisory remains separately tracked |
| windows-sys 0.61.2, raw-window-handle 0.6.2 | Native windows, instance coordination and capture identities |
| libc =0.2.191 (Unix only; additionally approved 2026-11-22) | Typed waitid(WNOWAIT) and process-group cancellation for owned local media providers; the resolved version was already locked transitively |

Existing workspace serialization, paths, core image/clipboard operations and standard-library workers remain
reusable. Single-instance locking can use standard-library file locking plus platform-local activation IPC.
Native host implementations now exist; that fact does not certify every event-loop, device or packaging path.
Any additional dependency, version change, renderer backend or installer tool must return for approval.

The user additionally approved the Unix-only direct libc pin on 2026-11-22 for the provider-child
ownership fix. It keeps the group leader unreaped until the group has been signalled, preventing a
PID/PGID reuse gap between observing provider exit and cancelling its descendants. Windows uses the
existing windows-sys package with a kill-on-close Job Object established while the child is suspended.
These mechanisms manage ordinary provider descendants; they are not a sandbox for hostile executables.

## Migration implementation and acceptance

1. **Preserve the service boundary.** Reuse `oxbox-core` and its tests. Extract Tauri-managed app services into
   a toolkit-independent host layer: jobs/progress/cancellation, settings transaction/rollback, reminders,
   power/desktop leases, media children, clipboard and Mouse Activity. Keep matching stop/join/restore behavior.
   Port UI-only search/favorites, countdown/stopwatch and preference models with their regressions; Rust core
   reuse does not automatically preserve behavior currently implemented in TypeScript.
2. **Verify the risky host pieces.** The square favorites launcher and live native shell are observed. Tray,
   single instance, login, hidden notifications, previews, file dialogs and multi-monitor/DPI selectors still
   need their recorded runtime checks. Do not treat initial feasibility as completion of those checks.
3. **Port every view.** Maintain a 49-row parity matrix for forms, errors, confirmations, progress, cancellation,
   result copy/export, providers and unsupported-platform notices. Use the canonical
   catalog/tools.json shared by native, legacy and Pages; typed adapters contain no duplicate final metadata. Migrate existing
   settings/favorites/reminders without losing data. Website hosting can remain static HTML.
4. **Revalidate presentation and accessibility.** Preserve the graphite/ember identity and native decorations.
   Test keyboard traversal, focus restoration, screen-reader names and announcements, contrast, IME/Unicode,
   high contrast/scaling and reduced motion on each OS. AccessKit availability is not proof of accessibility.
   Validate GPU/remote-session compatibility; the selected Glow/OpenGL backend is not a universal fallback.
5. **Repackage and measure.** Verify the native build wrapper and installer toolchain against the selected
   entrypoint. Recheck size/startup, fresh install/uninstall,
   notices, MSRV, supported-tool hardware behavior and all Quit cleanup. Existing draft measurements do not transfer.
6. **Measure the actual offline boundary.** Test production builds from clean profiles through startup, idle,
   every tool and Quit, including owned child/provider processes, DNS and TCP/UDP over bounded observation
   windows. Document coverage and remaining uncertainty. Check all three OSes; Windows results do not certify others.

First agree what the strict boundary includes: app-owned host/renderer/provider processes, externally launched
app sets, remote filesystem paths and OS services are distinct cases. The approved manual ECB refresh is an
explicit network exception; an unconditional never-network product would need import-only currency behavior.
Neither a native rewrite nor a finite packet sample warrants a universal zero-network guarantee.

## Approval and release consequence

Architecture/dependency approval is complete: eframe 0.37.0, Rust 1.95 and the listed native adapters are selected.
Current source and route/automation coverage are recorded in [the 49-tool migration matrix](../../tasks/native-migration.md).
Use the native package/check/build commands in AGENTS.md and the build-native wrapper; the Tauri runtime is
retained only for comparison. No additional renderer, dependency, global OS policy or security bypass is authorized.

Keep the release an unpublished draft until native production privacy observations, current cross-platform CI,
per-tool/hardware/accessibility behavior, notices/security review and native clean-machine installer acceptance
are recorded. Prior Tauri installers and finite packet samples do not certify the replacement host or a universal
zero-network guarantee. The runtime/release records distinguish historical and current evidence.

# ADR 0004: Separate the offline requirement from the embedded browser boundary

- Status: **Proposed — architectural and dependency approval required**
- Date: 2026-11-21
- Scope: research only; no renderer, dependency, lockfile, OS policy or security-setting changes

## Finding

The current supported WebView2 controls do **not** establish zero network activity for the application and
its browser descendants. Keep the release a draft and the strict acceptance gate open. This proposal does
not supersede [ADR 0001](0001-rust-tauri-svelte.md) without an explicit decision.

The [runtime investigation](../RUNTIME_PRIVACY.md) records working SDK resource interception alongside
browser-owned ECS, component-update and SmartScreen requests. Microsoft documents required diagnostics
outside the developer's overall control, plus narrower controls for crash reporting and SmartScreen.
Retain security protections; disabling reputation checks is not this proposal's solution.
[Microsoft privacy documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy).

## Supported controls and their limits

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

## Proposed next decision

If strict default offline behavior remains mandatory, approve a **separate native-renderer feasibility branch**,
followed by a full migration only if its gates pass. The concrete candidate is egui through native `eframe`,
without a webview. The upstream project supports Windows, macOS and Linux. This removes the particular
WebView2 service dependency; it does **not** prove that a new binary, its drivers, OS integration or launched
providers never communicate. [Upstream egui](https://github.com/emilk/egui).

The existing Svelte views cannot run inside egui. All **49 tool views and their interactions must be ported**;
keeping a hidden webview would retain the same boundary. Keep the current implementation available during
the migration, and preserve each tool's current platform availability rather than claiming all 49 work on every OS.

### Exact dependency proposal

Primary crates.io metadata checked on 2026-11-21 reports **egui 0.37.0 and eframe 0.37.0**, both non-yanked,
MIT OR Apache-2.0, with declared **Rust 1.95**. The 0.36 line also requires 1.95. Version **0.35.0 declares
Rust 1.92**, making it the newest published line compatible with Oxbox's current 1.93 minimum before resolving
and checking transitives. No packages were downloaded or built for this investigation.
[eframe metadata](https://crates.io/api/v1/crates/eframe), [egui metadata](https://crates.io/api/v1/crates/egui),
[0.35 manifest](https://github.com/emilk/egui/blob/0.35.0/crates/eframe/Cargo.toml),
[0.37 workspace minimum](https://github.com/emilk/egui/blob/0.37.0/Cargo.toml).

The proposed minimum-preserving spike adds one genuinely new direct renderer dependency:

```toml
eframe = { version = "=0.35.0", default-features = false, features = ["accesskit", "default_fonts", "glow", "wayland", "x11"] }
```

Use `eframe::egui`; a second direct `egui` dependency is unnecessary. Enable no inspection/debug server,
HTTP client, remote image loader or automatic external links. Draw charts/tables with existing widgets and
upload core-produced raster previews; no `egui_extras` or SVG-rendering dependency is required. The alternative
is exact 0.37.0 with a separately approved Rust 1.95 minimum and reviewed native features, including clipboard.
The selected version and complete new transitive graph still need audit, notices and locked compiler checks.
[eframe native renderer/accessibility features](https://docs.rs/eframe/0.35.0/eframe/).

Replacing Tauri plugins also needs explicit approval to promote these **already-locked** packages into the native host:

| Planned direct package | Purpose |
|---|---|
| `rfd = 0.17.2` | Native open/save/confirmation dialogs |
| `tray-icon = 0.25.1`, `global-hotkey = 0.8.0` | Tray lifecycle and configurable global launcher binding |
| `auto-launch = 0.6.0`, `open = 5.4.5` | Login registration and explicit open-folder/document actions |
| `tauri-winrt-notification = 0.8.1` (Windows) | Native reminder notifications |
| `mac-notification-sys = 0.6.15` (macOS), `notify-rust = 4.19.0` (Linux) | Platform notification adapters |
| `gtk = 0.18.2` (Linux) | Existing tray provider's GTK event-loop integration; its GLib advisory remains a separate unresolved review item |
| `windows-sys = 0.61.2` (Windows), `raw-window-handle = 0.6.2` where needed | Native window/instance integration and overlay/capture identities |

Existing workspace serialization, paths, core image/clipboard operations and standard-library workers remain
reusable. Single-instance locking can use standard-library file locking plus platform-local activation IPC.
This is a planned host surface, not an assertion that event-loop integration or packaging already works.
Any additional dependency, version change, renderer backend or installer tool must return for approval.

## Migration and acceptance plan

1. **Preserve the service boundary.** Reuse `oxbox-core` and its tests. Extract Tauri-managed app services into
   a toolkit-independent host layer: jobs/progress/cancellation, settings transaction/rollback, reminders,
   power/desktop leases, media children, clipboard and Mouse Activity. Keep matching stop/join/restore behavior.
   Port UI-only search/favorites, countdown/stopwatch and preference models with their regressions; Rust core
   reuse does not automatically preserve behavior currently implemented in TypeScript.
2. **Prove the risky host pieces first.** Build the square favorites launcher, search/shortcuts, one file-dialog
   tool and a timer. Prove tray/single-instance/login behavior, hidden notifications, image previews and a
   multi-monitor HiDPI selection/focus overlay. Stop the spike if required host integration cannot match behavior.
3. **Port every view.** Maintain a 49-row parity matrix for forms, errors, confirmations, progress, cancellation,
   result copy/export, providers and unsupported-platform notices. Move catalog data into a shared neutral
   manifest or generated interface so the native launcher and Pages catalog cannot diverge. Migrate existing
   settings/favorites/reminders without losing data. Website hosting can remain static HTML.
4. **Revalidate presentation and accessibility.** Preserve the graphite/ember identity and native decorations.
   Test keyboard traversal, focus restoration, screen-reader names and announcements, contrast, IME/Unicode,
   high contrast/scaling and reduced motion on each OS. AccessKit availability is not proof of accessibility.
   Validate GPU/remote-session compatibility; OpenGL is a proposed first backend, not a universal fallback.
5. **Repackage and measure.** Adapt the existing installer toolchain to the new entrypoint; verify this during
   the spike rather than assume Tauri packaging remains unchanged. Recheck size/startup, fresh install/uninstall,
   notices, MSRV, supported-tool hardware behavior and all Quit cleanup. Existing draft measurements do not transfer.
6. **Measure the actual offline boundary.** Test production builds from clean profiles through startup, idle,
   every tool and Quit, including owned child/provider processes, DNS and TCP/UDP over bounded observation
   windows. Document coverage and remaining uncertainty. Check all three OSes; Windows results do not certify others.

First agree what the strict boundary includes: app-owned host/renderer/provider processes, externally launched
app sets, remote filesystem paths and OS services are distinct cases. The approved manual ECB refresh is an
explicit network exception; an unconditional never-network product would need import-only currency behavior.
Neither a native rewrite nor a finite packet sample warrants a universal zero-network guarantee.

## Approval and release consequence

Keep the existing release a draft. Request a decision on the native feasibility branch, the exact proposed
dependency promotions and either retaining Rust 1.93 through eframe 0.35 or raising it to 1.95 for current eframe.
Do not replace all 49 views, add dependencies or relax the privacy requirement merely by accepting this research.
Acceptance of a final migration requires evidence, not this proposed ADR.

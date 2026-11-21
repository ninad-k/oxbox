# Windows runtime privacy follow-up

The user authorized app-scoped fixes and network observation on 2026-11-21, and explicitly approved direct
Windows SDK bindings already in Cargo.lock: `webview2-com = 0.39.1` and `windows = 0.62.2`
(`Win32_Foundation`, `Win32_System_Com`). This work does not authorize global browser, privacy, firewall,
security or update changes. Keep the release a draft until the acceptance gates pass.

- [x] Diagnose existing versus isolated app profiles with local, per-child diagnostic logs.
- [x] Identify vendor configuration and component-update endpoints without publishing user data or raw logs.
- [x] Write failing request-policy and fake-startup tests, then implement fail-closed filtering.
- [x] Create an app-only SDK environment; disable OS-account SSO, own crash reporting, retain security defaults.
- [x] Share the environment across main, capture and focus windows; install filters before app navigation.
- [ ] Compile, lint and run integrated tests on all supported platforms.
- [ ] Verify a production Windows build with the supported controls, including startup and steady-state TCP/UDP observations.
- [ ] Verify app navigation/IPC, overlays, normal Quit and descendant shutdown.
- [ ] Meet the strict default whole-process offline requirement, or record an explicit unresolved blocker without publishing.

Implementation: `src-tauri/src/runtime_privacy.rs` and its pure policy tests. The scope of SDK interception and
the measured evidence are documented in [RUNTIME_PRIVACY.md](../docs/RUNTIME_PRIVACY.md).

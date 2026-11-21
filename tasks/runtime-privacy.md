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
- [x] Compile, lint and run integrated tests on all supported platforms: [final CI](https://github.com/ninad-k/oxbox/actions/runs/44917613996).
- [x] Run bounded production Windows network observation with supported controls; vendor traffic remains (strict gate failed).
- [x] Verify SDK rejection counters, local status IPC and app-command Quit with all seven observed processes gone.
- [x] Add fake STA-before-environment and default-document route regressions; nine pure policy/startup/delivery tests pass.
- [x] Verify final production launcher UI, local IPC and normal Quit without diagnostic flags; all seven recorded processes exited.
- [ ] Verify capture/focus overlays in the supported monitor/DPI matrix.
- [x] Record the unresolved vendor-traffic blocker and retain an unpublished draft.
- [ ] Meet the strict default whole-process offline requirement.

The blocker is now concrete: app-only SDK controls leave browser-owned configuration, SmartScreen and
component-update requests. The draft remains unpublished; no global policy, protection or update setting
was changed. See the measured scope and limitations in the evidence record.

Implementation: `src-tauri/src/runtime_privacy.rs` and its pure policy tests. The scope of SDK interception and
the measured evidence are documented in [RUNTIME_PRIVACY.md](../docs/RUNTIME_PRIVACY.md).

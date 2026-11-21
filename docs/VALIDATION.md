# Validation record

This file records observed evidence, not planned results. Release gates remain open until explicitly recorded.

## 2026-11-21 implementation session

- Host: Windows, Rust 1.100.0; workspace minimum Rust 1.93.
- Initial scope: the 20-tool v0.1 catalogue and detailed Phases 1–4. The user subsequently promoted the
  Phase 5+ roadmap; those additional implementations and their verification are recorded separately below.
- Tests-first stage: missing API failures were observed for new core modules and the monotonic-clock tests
  before implementations were added.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: passed; 75 core tests and both crates' test/doc-test targets, 0 failures.
- `pnpm check`: passed, 0 errors and 0 warnings.
- `pnpm test`: 27 passed across 8 root test files; the unrelated nested copy is excluded.
- `pnpm build`: passed, including all 20 lazy tool views and the area overlay route.
- Actual minimum-toolchain check: installed Rust 1.93.0, then ran `cargo +1.93.0 check --workspace --all-targets --locked`
  with an isolated target directory. Passed on Windows. This covers the v0.1 implementation before Phase 5 additions.
- `pnpm tauri dev`: compiled and launched the native Windows binary. An accessibility snapshot exposed the
  20-tool catalogue, sidebar, search, favorites, and Settings. The desktop screenshot showed the Windows lock
  screen; UI interaction stopped, and visible functional testing/screenshots remain blocked until it is unlocked.
- `pnpm tauri build`: passed for the initial v0.1 implementation. MSI: 3,227,648 bytes (3.08 MiB);
  NSIS: 2,269,690 bytes (2.16 MiB). Both are below 15 MB. These are unsigned local builds, not published releases.
- The desktop was subsequently unlocked; the native launcher was observed with all 20 tools and Settings.
  A real dark-theme launcher screenshot was saved. Default shortcut conflict was surfaced visibly rather than
  crashing (an additional development process was running). Expanded Phase 5 work paused deeper UI checks until integration.
- At this initial stage, no public release/tag had been created.

## Expanded Phase 5 verification

- All 48 catalogue entries have unique IDs, platform metadata and importable views. Svelte check: 0 errors/0 warnings.
- UI unit tests: 28 passed across 9 files. Production frontend build passed with the 48 views and both overlay routes.
- Final workspace tests: 122 core tests plus 2 Tauri rollback tests passed, including the clipboard-after-Quit regression.
  No tests were disabled or ignored to obtain this result. Clippy with warnings denied and format checks passed.
- Actual Rust 1.93.0 locked workspace/all-target check passed with the new rxing and Windows COM/WinRT features.
- External-provider smoke: generated one-second synthetic video/audio, converted through core to WebM and GIF,
  verified the GIF header and refused an existing output. No physical camera, microphone or screen was opened.
- Explicit, user-authorized ECB provider smoke: HTTPS download parsed 30 currencies dated 2026-11-20.
  A stricter balanced-document/date-context regression was observed failing, then passed with the parser fix.
- Network proposal posted with explicit user approval as [issue #1](https://github.com/ninad-k/oxbox/issues/1).
  App preferences were not opted in by the provider smoke command.
- Native Windows release QA copy launched and displayed all 48 tools. Light appearance persisted across restart;
  real light/dark launcher screenshots were captured. Unit Converter returned 0.001 km for 1 m and 1000 m after
  swapping to 1 km through native IPC. Theme updates reported saved successfully.
- Final expanded installer build passed, including the Quit guards: MSI 3,969,024 bytes (3.79 MiB),
  NSIS 2,841,534 bytes (2.71 MiB). Both are below the 15 MB target; neither installer was installed or published.
- The original System appearance preference was restored after screenshot checks, and the QA app was left
  at the 48-tool launcher. No live recording, microphone change, cache cleanup, uninstallation, drive eject,
  resolution/power change, or lock/shutdown effect was invoked during UI QA.
- Cargo reports a future-compatibility notice for transitive `proc-macro-error2` 2.0.1 on the host compiler.
  Current builds, Rust 1.93 checks and project clippy checks pass; dependency upkeep should track that upstream notice.

## Final release review — 2026-11-21

- Three parallel review agents and the integrating agent reproduced and fixed regressions in recording shutdown
  and failed-output preservation, delayed capture cancellation, post-Quit file-job registration, settings rollback,
  notification generations, presentation cleanup and monotonic Sleep Timer display.
- TAR GNU/PAX preprocessing is bounded and cancellable before the library accumulates metadata. ZIP 9.8.1
  uses bounded ZIP64/footer preflight and strict local/central header checks. 7z uses frozen metadata limited
  to 128 KiB stored/decoded, a conservative coder profile and cumulative model estimate; see [ADR 0003](adr/0003-bounded-7z.md).
- Windows alternate-stream/device output names are rejected. The age header parser is bounded to 64 KiB
  while file payloads continue streaming. Focused regression failures were observed before their fixes.
- Final `cargo fmt --all -- --check`, workspace/all-target clippy with `-D warnings`, and locked workspace tests
  passed: **148 core + 3 desktop tests**, no failures or ignored tests. Locked workspace/all-target checks on
  actual Rust **1.93.0** passed after the final archive changes.
- Patched SvelteKit 3.4/adapter-static 4 configuration and explicit package subpath imports passed `pnpm check`
  with 0 errors/0 warnings, **31 UI tests across 9 files**, and a production frontend build. Frozen install passed.
- `cargo audit --json`: 0 vulnerability findings, 9 informational warnings, no ignored advisories.
  `pnpm audit --json`: 0 advisories. Scope and remaining upstream warnings are recorded in [DEPENDENCIES.md](DEPENDENCIES.md).
- Offline third-party notice verification passed: 497 packages, no missing notice texts after attributed,
  revision-pinned supplements, and 3 explicitly unresolved upstream attribution reviews. The notices and
  project licenses are configured as installer resources. Source scans found no credential-pattern matches,
  broken local documentation links or project files above 5 MB; the pre-existing nested copy was excluded.
- Actionlint and `git diff --check` passed. Final packaging, native smoke evidence and public CI are recorded
  below; earlier installer sizes above describe prior builds.
- First public CI run [44908023075](https://github.com/ninad-k/oxbox/actions/runs/44908023075) passed UI,
  Windows clippy/tests/notices and the Rust 1.93 check. It exposed a non-Windows `needless_bool` lint and
  Ubuntu 22.04's incompatible stock SPA headers for `libspa 0.9.2`. The link guard was simplified and Linux
  CI/packaging moved to Ubuntu 24.04. A current-source Rust 1.93 clippy pass also prompted an equivalent
  boolean simplification in the cache-age guard; locked workspace/all-target clippy then passed on 1.93.
  The initial failures are not counted as acceptance passes; the final run below supersedes them.

## Verified Windows draft artifacts and CI

- Functional source commit: `5117e6d53943f050b6120e6abfddae17d51d5a9e`, pushed on `codex/complete-v0-1`.
  Subsequent release/privacy documentation changes do not change Rust, frontend or bundle configuration.
- [CI run 44909470452](https://github.com/ninad-k/oxbox/actions/runs/44909470452) completed **successfully**:
  UI check/test/build, Rust 1.93 locked workspace/all-target check, and Windows/macOS/Ubuntu 24.04 clippy,
  tests and native application builds. Windows also reproduced the notices under Node 22.
- `pnpm tauri build` completed successfully from that source. Both x64 installers are unsigned:

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `Oxbox_0.1.0_x64_en-US.msi` | 4,272,128 | `49c717fa5eee20a0ee62b049d36ea3da65a304fc454c26f9646a3a8ccbf50049` |
| `Oxbox_0.1.0_x64-setup.exe` | 2,965,568 | `fa970ed0a6d23d68da38d7d17eadd498b2b4340652ddd0a206ab8052ca95e78c` |

- Read-only MSI database inspection confirmed product/version `Oxbox`/`0.1.0`, x64 template and exactly the
  application plus MIT/Apache-2.0 licenses and `THIRD_PARTY_NOTICES.txt`. Authenticode reported NotSigned for
  both installers and the application. No installer execution/install/uninstall was performed on this host.
- Final `pnpm tauri dev` launched successfully. Its native file picker selected a generated 22-byte fixture;
  SHA-256 matched `e2c43e235038b53c8df6cbf12b47277e2efa25ee3ef86e0887bc8f1a0f0f8f2d` from the independent
  file hash, and matching/mismatching comparison messages were observed. Keyboard search/Enter and live
  UTC/Dubai/London/New York clocks worked. Rust output was excluded from Vite watching after a concurrent
  Windows linker exposed EBUSY warnings; the corrected concurrent run stayed free of those watcher errors.
  Development runs were intentionally stopped with Ctrl+C, which is not a manual tray-Quit acceptance test.
- A separate copy of the final production binary displayed 48 tools and lazy-loaded Unit Converter. Native
  IPC returned 0.001 km for 1 m and 1000 m after swapping to 1 km. The default Ctrl+Alt+Space was occupied by
  another application; the conflict was reported. A temporary Ctrl+Alt+Shift+O saved and hid the launcher.
  Relaunching the same executable reopened the original process, and the title-bar close hid it to the tray
  while that process remained alive. The original shortcut preference was restored afterward.
- The production QA app was restarted at the 48-tool launcher with the original System appearance. No
  physical recording, mic/cache/uninstall/eject/display/power effect was invoked by these native checks.
- Runtime network acceptance did **not** pass: WebView2 public HTTPS was observed without a currency request.
  A second bounded sample normalized mapped addresses and confirmed a Microsoft public peer, with no Rust-host
  socket observed. Purpose/payload remain unresolved; [RUNTIME_PRIVACY.md](RUNTIME_PRIVACY.md) records the scope.
  A successful CI run or unsigned draft asset is not a substitute for resolving that privacy acceptance gap.

## Launcher, Mouse Activity and website revision — 2026-11-21

- Current source has **49 tools**, including the explicitly started Windows-only Mouse Activity MVP.
  Tests-first regressions cover idle scheduling, synthetic input, Stop/Quit cleanup, blocking-pool ordering,
  the asynchronous IPC dispatch boundary, and issued single-use action-bound request tokens.
- Current locked workspace checks passed: **165 core + 12 desktop tests**, no failures or ignored tests;
  workspace/all-target clippy with warnings denied and Rust formatting passed.
- Svelte check passed with 0 errors/0 warnings; **33 UI tests across 10 files** passed. The production frontend
  build passed. The dashboard defaults to favorites only, and Alt+1–9 opens favorite slots.
- Browser preview at 800 × 800 verified empty-dashboard onboarding, library pinning, exactly three selected
  favorite cards, Alt+1 opening Unit Converter, and favorite persistence after reload. This is browser UI
  evidence, not native IPC or pointer acceptance.
- Website build verified all 49 registry-derived tools, seven categories, local resources and fragment targets.
  Desktop and 375px mobile browser QA found no horizontal overflow or console errors. Search/platform filters,
  empty state, Reset and FAQ disclosure worked after fixing the observed reset-default-action regression.
  Both focused website regression tests passed; actionlint and staged whitespace checks passed.
- Rust 1.93 locked workspace/all-target checking passed with the new SDK bindings and native input features.
  Current audits found 0 Cargo vulnerability findings and 0 pnpm advisories; the same eight maintenance and
  one Linux dependency soundness informational warning remain. The regenerated 497-package notices verify;
  three upstream attribution reviews remain open.
- Native startup testing caught an uninitialized COM apartment after disabling automatic main-window creation.
  An owned STA guard and ordering/failure regressions fix that prerequisite. Local default-route normalization
  now matches Tauri's handling of `App("index.html")`; final native checks must use the subsequent frozen build.
- An intermediate production SDK probe confirmed configured request filtering and a locally blocked external
  document, while recording remaining Microsoft configuration and SmartScreen requests. This does **not**
  satisfy strict whole-process offline acceptance. See [RUNTIME_PRIVACY.md](RUNTIME_PRIVACY.md).
- GitHub Pages is configured for workflow deployment, including the implementation branch's environment policy.
  Public deployment, current native screenshots and rebuilt draft asset hashes are recorded after verification.

## Verified revised Windows draft — 2026-11-21

- Final functional source: `d4e50b762cb796f511281c17e3327de0776b2458`, on `codex/complete-v0-1`.
  [CI 44917613996](https://github.com/ninad-k/oxbox/actions/runs/44917613996) passed all five jobs: UI check,
  **35 UI tests**, frontend build, actual Rust 1.93 check, and Windows/macOS/Ubuntu 24.04 clippy, tests and
  native builds. Earlier [b449806 CI](https://github.com/ninad-k/oxbox/actions/runs/44915821430) also passed;
  it is superseded by the final run. The intermediate shortcut-only run was cancelled after the disclosure fix.
- `pnpm tauri build` produced the following final unsigned Windows x64 artifacts. Read-only MSI inspection
  confirmed `Oxbox` / `0.1.0`, x64 template, the application and exactly the three license/notice resources.
  No installer was executed or installed on this host.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `Oxbox_0.1.0_x64_en-US.msi` | 4,329,472 | `386586677a442355c6d48856768a3d6572cc9bd60681d59f875a66211ce94989` |
| `Oxbox_0.1.0_x64-setup.exe` | 3,000,767 | `efbf843d24a5d8dba202d49872cdf66f71f647635edd4fb5a7f7c619ee9dd24f` |

- Native Windows QA verified the normal square frame, empty favorites-only dashboard, four temporary pinned
  tools, theme persistence and real [light](images/launcher-light.jpg)/[dark](images/launcher-dark.jpg)
  screenshots. The original System appearance and empty favorite selection were restored and persisted.
  Unit Converter returned 0.001 km for 1 m, then [1000 m for 1 km](images/unit-converter.jpg) through native IPC.
- Native Ctrl+K focused search; typing and Enter opened a lazy tool. The initial automated Alt+number chord
  failed on the physical-code-only implementation. A tested fallback for absent/Unidentified codes fixed it;
  native Alt+1 and Alt+4 subsequently opened the selected favorites. Known numpad/other physical codes and
  extra modifiers remain excluded. No cause beyond the observed input behavior is assumed.
- Native Ctrl+Alt+O hid the app while its process stayed alive, then showed it when pressed from an inert,
  locally built test window. The disposable fixture exited afterward. Ctrl+Q also exited the earlier native
  QA copy and all seven recorded processes.
- Mouse Activity native Start, Waiting, zero movement and banner Stop passed with idle 3600 seconds and a
  one-minute cap. Restart was stopped. The final source's native Quit button was tested with a session enabled
  and zero movements; all seven host/WebView2 processes exited. No physical pointer-motion/input-return,
  held-input, protected-desktop, duration-expiry or multi-monitor claim is made from these no-motion checks.
- The final Settings view displayed the Microsoft Defender SmartScreen data-transmission notice. Native UI QA left
  launch-at-login and currency consent unchanged and did not change user privacy/security settings or invoke
  camera/mic, desktop visibility, display, drive or power effects.
- [GitHub Pages deployment 44915821604](https://github.com/ninad-k/oxbox/actions/runs/44915821604) succeeded.
  The public [site](https://ninad-k.github.io/oxbox/) returned HTTP 200 for its HTML, CSS, JS, tools JSON and logo;
  each response matched the verified local build byte-for-byte. The repository homepage points to that site.
- Strict WebView2 offline acceptance remains failed as documented above. [ADR 0004](adr/0004-runtime-offline-boundary.md)
  proposes a native-renderer migration and names the dependencies requiring approval; no renderer dependencies
  or minimum-Rust change have been applied.
- The existing [Windows draft](https://github.com/ninad-k/oxbox/releases/tag/untagged-cd5edee4ae6189e4d9fc)
  was updated with both installers and `SHA256SUMS.txt`. All three GitHub-uploaded byte counts and SHA-256
  digests match the local files; `isDraft: true`, prerelease status and the exact functional source target were
  verified afterward. No version tag was pushed and no public installer release was published.

## Open acceptance gates

- Full Windows manual tool matrix, including multi-monitor 100%/150% capture and destructive/power effects in a test account/VM.
- Independent human security checkpoint for the file tools and the documented attribution/license review.
- macOS/Linux supported-tool runtime acceptance and macOS Intel packaging acceptance.
- Clean-machine installer smoke test, cold startup measurement, and resolution of observed WebView2 runtime traffic.
- Published release assets and complete tool-by-tool OS acceptance. Light/dark launcher screenshots are now present.

Use [TESTING.md](TESTING.md) and [RELEASE.md](RELEASE.md) to close these gates. Do not infer a pass from the
presence of code, a ready launcher entry, or an unchecked checklist.

## Native renderer migration — 2026-11-21

- User approved ADR 0004, exact eframe 0.37.0 and minimum Rust 1.95; branch `codex/native-renderer`.
- All 49 stable tool IDs route to native forms, shared with the Pages/legacy catalogue through JSON.
- Tests first observed failures for launcher/preferences/atomic storage, stopwatch, clipboard accessor,
  monitor coordinate selection, abandoned jobs, hidden progress, shortcut transactions and instance activation.
- Windows native/core clippy with all targets and warnings denied passed. Combined tests passed **179 core +
  23 native**, none failed/ignored. Utility13 and media14 routes render headlessly without starting devices;
  system22 routes have compiled source/lifecycle regressions, not 22 manual passes.
- Real debug GUI opened with native decorations, AccessKit named controls, Segoe UI from the host, local
  outlined icons, an empty favorites-only Dashboard and the 49-tool library. Live inspection found/fixed
  inherited horizontal card layout, Enter search losing focus, missing star glyphs and missing navigation highlight.
- Actual Unit Converter displayed 0.001 km for 1 meter. Pinning four tools showed exactly four on Dashboard;
  Alt+1 opened the first. Ctrl+Alt+O hid the window; launching the same profile again exited the newcomer and
  restored the existing PID/window. Quit removed the first debug host and its console child.
- A bounded first debug privacy observation saw no owned TCP rows/UDP bindings; exact scope and sample gaps
  are in [runtime privacy](RUNTIME_PRIVACY.md). Production, DNS/OS-broker and other-OS coverage is separate.
- Native source, compiler, packaging and release verification are still in progress. The previous historical
  Tauri results above do not transfer to the native binary. Clean-machine, real hardware, screen-reader/IME/HiDPI,
  human security/license review and complete per-tool manual acceptance remain open. The release remains draft.

- Additional Windows debug UI smoke: a native file picker selected an isolated local fixture; SHA-256
  exactly matched PowerShell `Get-FileHash` (`5f6dbde1effd671b5daad02f4a2ae049dd7b3e5f6682392484a043dff9445820`).
  Inspection found unparented dialog focus restoration needing a follow-up fix before final packaging.
- Mouse Activity smoke used a 3,600-second idle threshold: Start reported Waiting, the independent banner
  Stop reported Stopped and zero movements. No synthetic pointer movement, clicks, typing or scrolling was
  triggered by that smoke. Physical idle/resume movement acceptance remains separate.

- Final review reproduced five native failures for exact shortcut modifiers and delayed clipboard Start versus
  Pause, plus two core failures for post-shutdown starts. Fixes order Pause/Clear/consent withdrawal after
  pending Start, block reopening under the worker mutex, stop clipboard before recording finalization on Quit,
  and match each key event exactly. Parented dialogs retain the actual eframe window through accept/cancel;
  a paired Slash text regression was observed and fixed. Legacy tray toggles and Escape navigation are restored.
- Exact **Rust 1.95.0** combined run passed **181 core + 33 native tests**, none failed or ignored. The new
  regressions use fake tasks/windows and never record the real clipboard or invoke native device/power/input effects.
- The first branded native production package has icon/version resources linked by the existing Windows SDK,
  with no added Rust dependency. Its extracted MSI executable matched the build hash; PE imports had no
  WebView2Loader or VC runtime redistribution DLLs. Final post-fix packaging/measurement supersedes these preliminary assets.

## Final native automated and packaging checks — 2026-11-21

- Exact Rust1.95.0 strict Core/native all-target clippy, formatting and locked tests pass: **191 core +52
  native**, no failures/ignored tests. UI37/37, Svelte0errors/0warnings, legacy comparison build and13
  packaging/notice regressions pass. The website still derives49tools/7categories and passes3 regressions.
- Tests first reproduced canceled jobs resuming, missing exact modifiers, clipboard starts after Pause/Quit,
  blocked-modal recovery, late recording/mouse/power admission, registry growth, and startup transaction/quoting/
  uninstall ownership errors. Fake workers/windows/backends were used; no live startup registration or effects.
- Native recovery uses bounded/coalescing independent system/media workers. Callback-entry barriers stop pending
  admissions and cancellation independently of egui polling during a synchronous file dialog. Quit requests
  close jobs/reminders/notifications and both cleanup lanes; normal shutdown joins all owned workers.
- Final Windows native production executable was built with Rust1.95.0, static CRT, original icon/version resource:
  **13,480,960bytes**, SHA256 `6fb64ef38e8a5725c72ef614cf78fc304775d8ed8310606ac72f45a447ba91b5`.
  MSI/NSIS builds succeeded, with an installed-copy marker and exact-owned startup uninstall cleanup. No generated
  installer has been installed on this workstation. Final payload inspection/runtime results follow separately.
- Startup defaults are tested as policy/fake transactions, including saved/OS-disabled choice preservation,
  development/isolated-profile guards, quoted Windows paths and preserving a repurposed Run command. Actual
  install/login/upgrade/uninstall behavior still requires a clean Windows account/VM.

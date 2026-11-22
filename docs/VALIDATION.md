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
- Preceding source-A Windows native production executable was built with Rust1.95.0, static CRT, original icon/version resource:
  **13,480,960bytes**, SHA256 `6fb64ef38e8a5725c72ef614cf78fc304775d8ed8310606ac72f45a447ba91b5`.
  MSI/NSIS builds succeeded, with an installed-copy marker and exact-owned startup uninstall cleanup. No generated
  installer has been installed on this workstation. Final payload inspection/runtime results follow separately.
- Startup defaults are tested as policy/fake transactions, including saved/OS-disabled choice preservation,
  development/isolated-profile guards, quoted Windows paths and preserving a repurposed Run command. Actual
  install/login/upgrade/uninstall behavior still requires a clean Windows account/VM.

## Native production candidate and installer inspection — 2026-11-21

The final functional source is `7132daa231627925359410132e754673f68905cc`. Its only change after
`efdab7435bfe6d89b93ff6198779d031ab7f8826` is a non-Windows duplicate-file clippy correction; the Windows
branch remains unchanged. A fresh exact-Rust-1.95.0 production build changed 24 PE timestamp/CodeView GUID
bytes; executable code, data, unwind tables and resources matched the preceding Windows build byte-for-byte.
The final installers were repackaged from this exact executable without another Rust compilation.

[CI run 44941250679](https://github.com/ninad-k/oxbox/actions/runs/44941250679) completed successfully for
that source: Windows, macOS ARM and Linux native strict clippy/tests/production builds, Windows Rust 1.95
minimum-toolchain check, and legacy UI/packaging regressions. macOS Intel runtime/package acceptance remains open.

| Final Windows artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `oxbox-native.exe` | 13,480,960 | `257b5b20f92724cc18dcf4e2167917d4d53385974f1521c87940ee1d72f9840c` |
| `Oxbox-native_0.1.0_x64_en-US.msi` | 5,689,344 | `21da99c977675a47ae1862688e2ca842da08f53be788778e7a6d532863221498` |
| `Oxbox-native_0.1.0_x64-setup.exe` | 4,024,779 | `81e5f5ed1ebfcc9d1c3e54e1774bac1e64f49e053714ad164c53178f2eabc781` |

- Independently extracted MSI executable hash matches the final executable. Its installed marker, MIT/Apache
  licenses and checked-in Windows notices match their inputs. Windows notice inventory: 355 packages,
  zero missing texts, three outstanding human attribution reviews. The notices SHA-256 is
  `c9edae1d9895ce866549691a9c5209976e81328a65d363b49a46a4a25701a7d0`.
- PE imports contain only Windows system DLLs, with no WebView2 loader or VC redistributable dependency.
  Icon resources match the original Oxbox icon; app/installer versions are 0.1.0. All artifacts are unsigned.
- MSI invokes ownership-checked native startup cleanup before RemoveFiles, only for full uninstall outside
  an upgrade. Generated NSIS source and successful compiler output verify branded install/uninstall icons,
  no WebView installation, guarded native cleanup before executable removal, and no unconditional Run deletion.
  NSIS was not independently extracted or installed; clean-machine acceptance remains open for both installers.
- Actual native Windows GUI checks on the preceding byte-equivalent-code build covered empty/four-favorite
  dashboards, light/dark appearance, Alt+1, search, conversion/swap, local QR preview and parented file-picker
  Cancel/Accept with foreground restoration. Fixture checksum matched
  `5f6dbde1effd671b5daad02f4a2ae049dd7b3e5f6682392484a043dff9445820`.
- Mouse Activity produced two pointer-only movements with 30-second idle/interval settings, paused on Tab
  input and stopped from its banner. Quiet `--autostart` produced a tray-only process; second launch restored
  the original window and direct Quit removed all owned processes. No installed startup registration was changed.
- The final executable independently opened an empty Dashboard and 49-tool library, displayed 0.001 km for
  1 meter, and exited through its Quit button. A manual negative Ctrl+Shift+Q check (extra Shift should reject
  the Ctrl+Q binding) was inconclusive after UI automation reported an unusable window. Exact-modifier unit
  tests pass; this negative check and full manual keyboard-Quit acceptance remain open.
- A separate exact-final-executable endpoint observation completed normally over 360.154 s / 7,201 samples,
  before launch through normal startup, brief idle, Unit Converter and direct Quit, ending with zero owned
  processes. No owned TCP rows (including listeners/loopback) or UDP bindings were observed; aggregate checks
  of all rows confirmed zero endpoint sums/maxima, and query/marker errors were zero.
  p95/maximum sample gaps were 62.62/81.82 ms; approximately 283.1 s followed the last owned sample.
  These are bounded Windows snapshots, not DNS/packet/provider or all-platform acceptance.
- Cross-target notices were generated without overwriting Windows output: macOS ARM 372 packages / 0 missing /
  29 reviews; macOS Intel 374 / 0 / 29; Linux x64 541 / 0 / 4. Pinned supplements have verified source/text hashes.
  Reproducible notices do not close the human attribution or installer-toolchain review.
- GitHub Pages desktop and 375-pixel mobile checks passed: actual native hero image, no horizontal overflow,
  Mouse search, platform empty state, reset to 49 tools and startup FAQ. The site does not offer public draft downloads.
- The [existing release draft](https://github.com/ninad-k/oxbox/releases) (release ID `453727915`)
  targets functional source `7132daa`; its native MSI/NSIS, checksum manifest, notices and two Oxbox license
  files were downloaded after upload and all six hashes matched their local inputs. Superseded Tauri installer
  assets were removed. Remote postcondition: draft true, prerelease true, six native review assets, no version
  tag pushed and no release published. The checksum manifest SHA-256 is
  `a18f6c149d95897bca7c7dcaf349b1b276efc504682f152af7b8764aaea6a428`.

Full per-tool/hardware/monitor/accessibility acceptance, real install/login/upgrade/uninstall, independent
security/license review, DNS coverage and other-OS runtime observation remain open. See [RELEASE.md](RELEASE.md).

## Parallel release review fixes — 2026-11-22

- User requested parallel installer/privacy/accessibility/security checks, fixes, commit/push and a draft-only
  release. No disposable Windows VM was available; the user explicitly kept clean-machine acceptance pending.
- Observed failing regressions for stale duplicate keeper/selection cleanup, inherited provider pipes delaying
  timeout/exit, password undo retention, ambiguous row controls, archive/capture labels and unannounced errors.
  The process regressions previously took about 4.12 seconds each; both now pass in 0.58 seconds combined.
- The user approved promoting already-locked `libc = 0.2.191` to a Unix-only direct dependency for typed
  waitid/WNOWAIT ownership. No resolved dependency version changed; the lockfile only adds the core edge.
- Exact Rust 1.95.0 all-target strict Clippy and formatting passed. Integrated tests passed **208 core + 64 native**,
  zero failed/ignored; core 106.16 seconds, native 0.31 seconds. Generated fixtures avoid real Trash, device,
  clipboard or network effects. The real provider-process tests use bounded self-reexecuting test fixtures.
- AccessKit name checks now cover all 49 default forms, with the 22 system forms denying task admission before
  rendering; expanded/two-row/error/password fixtures are separate regressions. This is not screen-reader acceptance.
- Packaging/notice/installer regressions passed 18/18, and Windows endpoint-observer argument guards passed 2/2.
  Native Windows notices regenerated/verified: 355 packages, zero missing texts, three human attribution reviews.
- Fresh audit invocation retained zero vulnerability entries, eight maintenance warnings and one GLib unsoundness
  warning using advisory DB `55e1c3dfbd834237a05bbbf7b50ba3b5be27b662`. Primary metadata still has no compatible
  published GLib 0.18 patch. Human review and Linux remediation remain open; no advisory was suppressed.
- Existing GitHub Actions versions are pinned to verified upstream commits. Checks above precede rebuilt
  installer/runtime verification, which must be recorded for the actual replacement executable and source.

### First review candidate `54dd593`

[CI run 44981134584](https://github.com/ninad-k/oxbox/actions/runs/44981134584) passed all five jobs with
exact Rust 1.95.0: Windows 208 core/64 native tests, macOS ARM and Linux 201 core/65 native tests each,
zero failed/ignored, strict Clippy and native release builds. Both real inherited-pipe/process cleanup
regressions passed on all three systems. Minimum Rust and legacy UI/packaging checks also passed.

The Windows production EXE (13,523,456 bytes, SHA-256
`428b28603ec2c16d825e833250fba1cb1a1dd06d2d9d6ef1f78b6e4253c94273`) and MSI/NSIS passed the new
read-only installer verifier. [Installer evidence](INSTALLER_ACCEPTANCE.md) records its exact package hashes.
The Windows accessibility tree confirmed named Appearance/search/chooser controls, literal Unicode retention,
keyboard search/Enter conversion (1 meter → 0.001 km), an alert for invalid input without moving focus,
safe generated-provider results and Ctrl+Q shutdown. No installed startup registration was changed.

The published endpoint observer completed a separate **360.095 s / 7,199 samples** window on that exact
EXE, before launch through UI use, explicit generated-provider checks and Quit. TCP rows (including listeners/
loopback) and UDP bindings had zero maxima and sums; query/marker errors were zero and the final EXE hash
was unchanged. First/last ownership was at 33.158/203.955 s, leaving 156.140 s after the last owned sample.
Observed names were the native app, generated `ffmpeg.exe`/`tesseract.exe` and their `conhost.exe`; at most
three owned processes appeared. p95/maximum gaps were 62.815/136.954 ms. Generated providers only printed
fixed version text; this is not actual FFmpeg/Tesseract, DNS, packet or other-platform traffic acceptance.

This UI pass found provider results lost completion semantics after their task result was consumed.
A narrow follow-up adds polite completion and contextual provider errors; its actual-tree regression failed
first, then all **65 Windows native tests** and strict Clippy passed. The reusable observer's validity function
also now rejects phase-marker failures, with three guard/behavior tests passing. Candidate `54dd593` remains
historical evidence; replacement artifacts and observations must identify the follow-up source/hash separately.

### Final review candidate `d8d1d1e`

[CI run 44982236463](https://github.com/ninad-k/oxbox/actions/runs/44982236463) passed all five jobs:
Windows **208 core + 65 native**, macOS ARM and Linux **201 core + 66 native** each, zero failed/ignored,
strict Clippy, native release builds, Rust 1.95 minimum and legacy UI/packaging checks. Both real process-tree
regressions and the consumed-provider completion/error regression explicitly pass on all three systems.

Exact Rust 1.95.0 Windows production artifacts:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Native EXE | 13,524,480 | `af363cd668e412f111bdfb37de39738ff35089e993585b19725932ba2de74b26` |
| `Oxbox-native_0.1.0_x64_en-US.msi` | 5,709,824 | `e2f9ab781123b37957bbc6ab2ed50edd30cdae296b52698ba3db86ccaafe3c1d` |
| `Oxbox-native_0.1.0_x64-setup.exe` | 4,024,370 | `38f8eb7e5c72770d25c8e99bbeef3b0f5eb7457a5bbcc790dc99215d40e28360` |

- The no-install verifier passed all five extracted MSI payload hashes, lifecycle properties/cleanup scope,
  generated NSIS guards and actual compiler-output identity. Original icons/version and 24 Windows-system
  DLL imports were verified, without WebView2 or VC redistributable imports. All artifacts remain unsigned.
- Final Windows notices SHA-256 is `6f22422e0098e8dc70702a06d63a321627d695fd0e9fbb952a137c8a85c792d2`;
  inventory remains 355 packages / zero missing texts / three human reviews. Manifest SHA-256 is
  `fb7142dcea1b5effad32ab62547edaf8ee15d0aca3340e11a1d1eeac716d87c3`.
- Two separate final-EXE endpoint windows completed: **240.050 s / 4,800 samples** for generated-provider
  completion and normal Quit; **240.064 s / 4,799 samples** for Quit during a controlled inert job. All owned
  TCP/UDP sums and maxima were zero, query/marker failures were zero, hashes unchanged and final process
  counts zero. Exact gaps/timings/exclusions are in [runtime privacy](RUNTIME_PRIVACY.md).
- Windows UI Automation observed the provider completion Status, Working state and named fields. The long
  generated child disappeared over 15 seconds before its natural exit, with input unchanged and no output/
  staging files left. This is a bounded app lifecycle check, not actual media/provider or screen-reader acceptance.
- Release ID `453727915` now targets `d8d1d1e`. All six uploaded assets were independently downloaded and
  hash-matched to their final inputs. Postcondition: draft true, prerelease true, exactly six native review
  assets, no version tag pushed and no release published.

Clean-machine installation/login/upgrade/uninstall remains pending at the user's request. Independent NSIS
extraction, full devices/monitor/IME/screen-reader/runtime matrix, real-provider/DNS coverage, human security/
attribution review and the documented Linux GLib finding remain open. No final universal privacy or release
acceptance is inferred from these bounded checks.

## 2026-11-22 community and functional follow-up

Three parallel source audits traced all **49 native routes** (13 utilities, 14 media, 22 system) through
their concrete controls and core APIs. [The functional review](FEATURE_REVIEW.md) lists reproduced gaps,
test names and remaining limits. No full native manual row is closed by this source review.

- Fixed confirmed reminder removal, currency cancellation/atomic cache admission, temperature polling,
  app-set cancellation/review cleanup, system navigation, stale media/search results, advertised EAN-13
  input and pasted IANA search. Optional original-file Trash now binds to streamed input/output hashes,
  file identities and stamps; generated-effect guards and the real age round trip pass.
- The exact Rust 1.95.0 integrated Windows run passed **221 core + 80 native tests**, zero failed/ignored;
  strict all-target Clippy and formatting passed. The core suite took 164.04 seconds, dominated by the
  existing real passphrase test; native tests took 0.46 seconds. Later native-only status checks are recorded
  with the final source/artifact mapping below rather than silently included in these counts.
- The final native-only run passed **81 tests**, zero failed/ignored (0.48 seconds), after the independent
  review caught consumed cleanup status. Its actual poll/headless regression now preserves success, avoids
  repeated Trash offers and reports uncertainty after a cancelled/errored OS cleanup attempt. Strict Clippy
  and formatting passed again; core source was unchanged after its 221-test run.
- Svelte/TypeScript check: zero errors/warnings. All **39 UI unit tests** and the production legacy build pass.
  All 49 lazy view imports remain covered. Packaging/notice/observer regressions: **21 passed**; website:
  **3 passed**, with 49 canonical tools and 7 categories.
- The new generated community feature index matches canonical metadata. A local check verified **156 file
  references across 14 contributor/architecture documents**; a separate review checked heading anchors and
  corrected fork instructions, preview-only advice and Focus overlay terminology.
- The reported `127.0.0.1:4174` module load failure was reproduced as a refused connection to a stopped
  preview server. The existing chunk remained on disk; after restoring the server, its exact URL returned
  HTTP 200 JavaScript and the Unit Converter form loaded in the browser. A regression-tested explicit reload
  action now explains unavailable modules and warns about unsaved input. Desktop commands still require a host.
- Windows notices remain 355 packages, zero missing texts and three human attribution reviews. No dependency
  or lockfile change was needed for this follow-up. The Linux GLib finding and other human reviews stay open.

This section records source/test evidence. Replacement production artifacts, all-platform CI and any new
runtime observation must identify their own source/hash before replacing the draft. The earlier `d8d1d1e`
artifacts and endpoint windows remain historical. Clean-machine testing remains pending at the user's request;
full manual/hardware/provider/DNS/accessibility and independent human acceptance remain release gates.

### Cross-platform correction after `1ae368c`

[CI run 44986859533](https://github.com/ninad-k/oxbox/actions/runs/44986859533) passed Windows
(221 core / 81 native), Linux (214 / 82), minimum Rust and UI/packaging. macOS ARM passed strict Clippy,
the encryption receipts/real age round trip, World Time and inherited-pipe cleanup cases, but failed two
core tests: a fast provider's cleanup returned EPERM, and a Mouse Activity fixture assumed a worker had
published its paused state after a fixed 75 ms sleep. The overall run failed and its artifacts were not
promoted to the draft. Pages deployment 44986859480 succeeded for that source (49 tools, 7 categories).

The mouse fixture now waits for the published phase with a bounded deadline. Darwin cleanup now includes
a narrow exited-group check: Apple's pinned kernel source filters zombies in the group signal path and
can return EPERM when no signalable member is found. The process list API includes live and zombie members
under its list lock. See [XNU signal implementation](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/kern_sig.c#L1612)
and [process census](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/proc_info.c#L377).
The interpretation matches the observed failure; actual macOS CI must verify the correction. Live,
unreadable, truncated, changing or mismatched groups must still fail closed while the leader remains unreaped.

The four portable census-proof tests first produced three passes and the expected zombie-only failure,
then passed after implementation. The scoped Windows follow-up passed **24 provider tests** and the mouse
worker acknowledgement regression, plus strict all-target Clippy and formatting. macOS additionally has an
eight-fast-exit regression, retaining the original raw-byte fixture without delay or suppression. Final
cross-platform results must be recorded against the follow-up source rather than calling the failed run green.

GitHub's private vulnerability reporting was found disabled during the community check. It was enabled and
the repository API returned `enabled: true` on 2026-11-22, making the existing private-report policy actionable.

### Community review candidate `a7c91bd`

[CI run 44987974907](https://github.com/ninad-k/oxbox/actions/runs/44987974907) passed all five jobs:

| Platform | Core tests | Native tests | Clippy / production build |
| --- | ---: | ---: | --- |
| Windows x64 | 225 | 81 | Passed |
| macOS ARM | 219 | 82 | Passed |
| Linux x64 | 218 | 82 | Passed |

All have zero failed or ignored tests. Rust 1.95 minimum, Svelte zero errors/warnings, 39 UI tests, 18 pure
packaging regressions and the 49-tool generated index also pass. Logs explicitly confirm receipt/real age,
durable cleanup status, IANA search, raw-byte/text, mouse acknowledgement and inherited-pipe cleanup tests.
macOS additionally passes the eight-fast-exit case. The earlier failed run remains documented above.

The production wrapper rebuilt source `a7c91bd696af0f80c86029336593e3d37d64f78b` with exact Rust 1.95.0.
All unsigned Windows artifacts are pinned separately from the preceding `1ae368c` and `d8d1d1e` builds:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Native EXE | 13,581,312 | `1c8715a5b9739ad1142bc91475d552ea0384fbd538651704d4f687da6756c7c3` |
| `Oxbox-native_0.1.0_x64_en-US.msi` | 5,734,400 | `16a396fe20f6b89feb1e86dde34c71c88a4c7a947cb92ab81bab703d5ef529b5` |
| `Oxbox-native_0.1.0_x64-setup.exe` | 4,040,336 | `e30fda0d81985fe8c5ade354c97aa14448d58ec4c7a1703f3cc5d9bc147ca3a9` |
| `SHA256SUMS.txt` | 452 | `fab42eef64f1b9d1315ff725cc39b46bccd1a91dfe7a2b1fb7965ff905830e53` |
| `THIRD_PARTY_NOTICES.txt` | 3,696,014 | `6f22422e0098e8dc70702a06d63a321627d695fd0e9fbb952a137c8a85c792d2` |

Static checks passed for the local and downloaded installers: all five extracted MSI payloads match the
reviewed inputs; lifecycle/upgrade properties and generated NSIS guards/compiler identity match. Version
metadata is Oxbox 0.1.0, signature NotSigned, and 24 Windows system DLL imports have no WebView or VC runtime
redistributable matches. The original icon source is unchanged; its historical six-resource comparison was
not repeated for this binary. No installer or MSI custom action was executed. Evidence is
`target/community-native-final-artifacts.json` and verifier reports `run-PHwlIn` / `run-xW3O75` below
`target/installer-verification/`.

The exact EXE copy was run with a fresh explicit profile. Windows UI Automation and visible rendering
confirmed the empty favorites Dashboard, 49-tool library, keyboard search/Enter, exact `America/New_York`
selection, a saved clock showing EST / UTC−05:00, and an EAN-13 preview from `590123412345`. Save/Copy were
not clicked. Normal sidebar Quit closed the process; no installed startup choice was changed.

A separate **240.073-second / 4,800-sample** endpoint window began before launch and ended after Quit.
All owned TCP/UDP sums/maxima, query/marker failures and final process counts were zero; app/script hashes
were unchanged. First/last ownership was 25.451 / 155.318 seconds, leaving 84.755 seconds after last ownership.
p95/maximum gaps were 62.715 / 99.123 ms. An independent all-row check agrees with the summary. Only the native
EXE appeared. `target/native-privacy-qa/run-community-native/` retains pins, original rows/summary and strict
assessment; `target/native-qa/community-review/` retains generated-input UI/Quit proof. No providers, devices,
clipboard collection, file export, OS/destructive effect or ECB refresh was invoked. DNS, brokers, packets,
short flows and other-platform/provider acceptance remain outside this observation.

Release ID `453727915` targets `a7c91bd`, remains **draft / prerelease**, and contains exactly six native
assets. All six were downloaded to `target/remote-community-final/` and byte-size/SHA-256 matched before
the downloaded MSI/NSIS checks passed. No version tag or public release was created. The public Pages
deployment from `1ae368c` remains valid for its unchanged 49-tool catalogue and source-branch links.

Clean-machine install/login/upgrade/uninstall remains pending at the user's request. Full per-tool manual,
hardware/providers, screen-reader/IME/DPI, DNS/broker and other-OS runtime, independent NSIS extraction,
signing and human security/attribution/dependency gates remain open. These results do not certify every feature
on every device or authorize publishing the draft.

The final documentation pass verified 267 local file references across 22 contributor/architecture/
acceptance documents and checked the generated feature index again. New issue forms remain staged in the
native branch until its reviewed merge into the default branch; private vulnerability reporting itself is
enabled now. Documentation-only evidence updates do not change the functional source or installer mapping.

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

## Open acceptance gates

- Full Windows manual tool matrix, including multi-monitor 100%/150% capture and destructive/power effects in a test account/VM.
- Independent human security checkpoint for the file tools and the documented attribution/license review.
- macOS/Linux supported-tool runtime acceptance and macOS Intel packaging acceptance.
- Clean-machine installer smoke test, cold startup measurement, and resolution of observed WebView2 runtime traffic.
- Published release assets and complete tool-by-tool OS acceptance. Light/dark launcher screenshots are now present.

Use [TESTING.md](TESTING.md) and [RELEASE.md](RELEASE.md) to close these gates. Do not infer a pass from the
presence of code, a ready launcher entry, or an unchecked checklist.

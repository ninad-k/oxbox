# Preparing a native v0.1 release

The native migration uses exact eframe 0.37.0 and Rust 1.95, with 56 tool routes preserving the original 49. Version 0.1.0 is a
release target. The existing release must stay an **unpublished draft** until the acceptance gates pass.
Historical Tauri evidence in [VALIDATION.md](VALIDATION.md) does not certify the native application.

## Current dependency and accessibility candidate `8a62ec1`

Source `8a62ec1ad7e65ad9b3378dbcb495b897ef725c47` passed all five jobs in
[CI 45039126970](https://github.com/ninad-k/oxbox/actions/runs/45039126970), including strict Clippy,
three-platform production builds, minimum Rust 1.95, localization diagnostics and optimized Linux GLib.
All 56 tools remain. The [current receipt](verification/2026-11-22-dependency-accessibility.md) records
exact test counts, source provenance, final executable and bounded Windows interactions/endpoint windows.

Release ID **453727915** now targets that source and remains **draft/prerelease**, with no actual version
tag. Verified backup of the preceding A9 assets/notes completed before replacement; all six downloaded
assets matched and passed static inspection. MSI **6,062,080 bytes**, NSIS **4,324,931 bytes**, unsigned.
[Current hashes](VALIDATION.md#dependency-and-accessibility-follow-up-8a62ec1) identify the binaries.
No installer/custom action executed. Clean-machine install/login/upgrade/uninstall is intentionally
pending; full manual/hardware/accessibility/privacy and independent human/signing gates stay open.
Five maintenance findings and the separately accounted GLib source advisory are documented. No publication is authorized.

## Current 56-tool follow-up `a9b6e1c`

Historical receipt, superseded by `8a62ec1` above.

Checked source `a9b6e1c13a2d15c0a87fa1fa42b364209e915310` passed all five jobs in
[CI 45027690468](https://github.com/ninad-k/oxbox/actions/runs/45027690468): Windows **285 core / 135 native**,
macOS ARM and Linux **280 / 137** each, zero failed/ignored, strict Clippy and production builds.
The [56-tool receipt](verification/2026-11-22-56-tool-pass.md) records each route and the exact EXE's bounded
generated checksum/color, tray/restoration/Quit and 300.070-second endpoint checks.

Release ID **453727915** targets that exact source and stays **draft/prerelease**, with no real version tag.
Six prior `c30a179` assets/notes were backed up and verified; all six replacement downloads matched and
passed static checks. MSI **6,033,408 bytes**, NSIS **4,260,245 bytes**, unsigned.
[Artifact hashes](VALIDATION.md#complete-56-tool-follow-up-a9b6e1c) distinguish this candidate from older builds.
Clean-machine install/login/upgrade/uninstall remains intentionally pending, as do real native-effect,
hardware/accessibility/privacy, independent extraction, signing and human review gates. No publication is authorized.
Earlier candidate sections below retain historical evidence for their own source and executable.

## Current maintenance/workflow candidate `c30a179`

Historical receipt, superseded by `a9b6e1c` above.

Source `c30a179a573424b178def065a3c5c0f5c9397074` passed all five jobs in
[CI 45011422402](https://github.com/ninad-k/oxbox/actions/runs/45011422402): Windows **279 core / 112 native**,
macOS ARM **274 / 114**, Linux **273 / 114**, zero failed/ignored tests, strict Clippy and production builds.
The exact Windows EXE and generated-data GUI/runtime scope are recorded in the
[maintenance verification receipt](verification/2026-11-22-maintenance-workflows.md).

Release ID `453727915` targets `c30a179` and remains **draft / prerelease**, with no actual version tag.
All six replacement assets were downloaded, byte/hash-matched and statically inspected after verified
backup of the preceding `67f0715` assets and notes. MSI **6,017,024 bytes**, NSIS **4,245,923 bytes**, both
unsigned. The user explicitly keeps clean-machine install/login/upgrade/uninstall pending. Other manual,
hardware/accessibility/privacy and independent human security/attribution/signing gates below remain open.

## Previous 49-tool candidate `67f0715`

The preceding 2026-11-22 review candidate was source `67f0715`, including the
[second bounded 49-tool verification](verification/2026-11-22-native-feature-pass.md). All five CI jobs pass:
Windows 226 core / 94 native, macOS ARM 220 / 95 and Linux 219 / 95, with zero failed or ignored tests,
plus strict Clippy, native production builds, Rust 1.95, UI and packaging. Its exact Windows EXE opened all
49 default native forms; Timer/QR state checks and generated-provider Cancel, hide/restoration and active
Quit also passed. Two separate bounded endpoint observations are recorded in
[VALIDATION.md](VALIDATION.md). These checks do not establish full native effect/hardware acceptance.
At that verification, release ID `453727915` targeted `67f0715` and remained **draft / prerelease**. All six refreshed assets
were downloaded, byte/hash-matched and statically inspected. Earlier `a7c91bd` startup, IANA/EAN-13 and
Quit measurements remain historical evidence for their own executable.
Earlier `7132daa`, `54dd593`, `d8d1d1e` and the failed-CI `1ae368c` candidate remain historical evidence.
The user explicitly left the clean-machine gate pending because no disposable Windows environment is available.

## Native acceptance gates

- [x] Preserve all 49 original stable IDs within 56 concrete native routes; keep shared JSON metadata in sync with Pages.
- [x] Pass local core/native tests, all-target clippy with warnings denied, and formatting.
- [x] Record actual decorated Windows launcher interactions. Earlier candidates cover favorites, conversion
  and favorite/global hotkeys; `67f0715` covers all 49 default forms, Timer/QR state changes, generated-provider
  Cancel, active-job hide/restoration and Quit. Each remains scoped to its source and executable.
- [x] Complete review-fix Windows/macOS ARM/Linux native CI and Rust 1.95 locked checks:
  [run 45039126970](https://github.com/ninad-k/oxbox/actions/runs/45039126970), source `8a62ec1`;
  Windows 294 core / 5 golden / 150 native, macOS ARM 289 / 5 / 152, Linux 289 / 5 / 153,
  with zero failed/ignored tests in the full suites.
- [x] Complete the parallel agent source/regression review and fix reproduced installer, file, provider,
  passphrase and accessibility issues; [security review](SECURITY_REVIEW.md), [privacy](PRIVACY_ACCEPTANCE.md),
  [accessibility](ACCESSIBILITY.md), [functional review](FEATURE_REVIEW.md) and [installer protocol](INSTALLER_ACCEPTANCE.md) distinguish scope and open gates.
- [ ] Complete native per-tool manual acceptance, including progress, cancellation, destructive confirmations,
  provider failures, power/desktop/recording restoration and hidden notifications.
- [ ] Complete production runtime observations including owned providers and DNS coverage on all three OSes;
  record observation bounds, rather than claim universal zero traffic from finite samples.
- [x] Record bounded Windows native production endpoint observations, including a quiet-tray/activation/Quit loop.
  Current `8a62ec1` has a 300.103-second / 5,998-sample generated-check/tray-restoration window ending
  with an app alive, plus a separate 90.033-second / 1,800-sample startup/Quit window ending with zero
  owned processes. Earlier measurements retain their own scope; no all-provider/DNS claim transfers.
  See [runtime privacy](RUNTIME_PRIVACY.md) for exact intervals and snapshot/DNS limitations.
- [x] Extract/hash-match the current `8a62ec1` draft MSI payload; inspect sizes/hashes and native packaging boundaries.
  Earlier icon/resource pixel and digest observations retain their historical executable scope.
  Verify generated NSIS source and successful compiler output; independent NSIS extraction is still open.
- [x] Pass local and downloaded Windows static installer checks for `8a62ec1`; record current-binary
  bounded GUI/runtime validation separately from installer execution and full manual acceptance.
- [ ] Test actual native MSI/NSIS clean install/uninstall and independently inspect NSIS payloads.
- [ ] Verify first-installed-launch startup default, quiet login-to-tray, opt-out/upgrade preservation, paths with spaces,
  OS-disabled choices and ownership-checked uninstall cleanup on a clean Windows account.
- [ ] Verify macOS Intel/Apple Silicon and Linux packages and supported hardware/session behavior.
- [ ] Verify keyboard/screen-reader/IME/Unicode, DPI/monitor changes, high contrast and remote/GPU sessions.
- [ ] Resolve/review dependency findings and outstanding attribution reviews with a human reviewer.
- [ ] Complete independent human security acceptance and remaining adversarial filesystem/real-provider runtime checks.
- [x] Record rebuilt review-fix source `8a62ec1`, installer checksums and all six downloaded remote asset hash
  matches in [VALIDATION.md](VALIDATION.md). Release ID `453727915` remains draft/prerelease;
  no version tag has been pushed or release published.

## Native packaging

```bash
pnpm install --frozen-lockfile
cargo fetch --locked --target x86_64-pc-windows-msvc
node scripts/build-native.mjs --bundle --target x86_64-pc-windows-msvc
```

Use Rust 1.95.0 or newer; release validation records the actual selected compiler. Windows builds use an
explicit target and static CRT. Native artifacts go beneath `target/native-dist/<target>/release/`.
The existing Tauri CLI is used only as an installer packaging tool with isolated metadata and WebView
installation/updater artifacts disabled. It does not add a Tauri/browser runtime to the native executable.
Native Linux distribution uses a portable tar archive, not a WebKit-dependent AppImage/deb assumption.
See [packaging/native/README.md](../packaging/native/README.md) for platform commands and installed tooling.

Optional FFmpeg/Tesseract providers are user-installed and never bundled or downloaded at app startup.
Bundler tooling downloads, when needed at build time, are separate from application runtime behavior.
Generate notices for the selected target; resolve missing upstream texts with revision-pinned supplements.
The checked-in notices inventory is Windows-specific. macOS ARM/Intel and Linux generation has zero missing
texts, with target-specific human attribution reviews still open; see [provenance](third-party-licenses/README.md).

## Draft workflow

Do not push a version tag merely to prepare a Windows draft: tag pushes start the four-platform release
workflow. After approval of a reviewed version tag, `.github/workflows/release.yml` builds native Windows,
macOS arm64/x64 and Linux artifacts, computes checksums and attaches outputs only to a draft. It refuses
to modify a published release and never publishes one. A successful workflow is build evidence, not manual signoff.

A maintainer may update the existing Windows-only draft from a pushed reviewed native commit, attach the
actual MSI, NSIS and checksums, and list every outstanding gate. Verify its target and `isDraft: true` both
before replacing assets and afterward. Do not mix historical Tauri installers with native installers under
ambiguous filenames. Record remote asset hashes after uploading. No automatic updater is configured.

Builds remain unsigned. Document OS warnings without telling users to disable protections. Never commit
signing certificates, credentials or tokens. Publish deliberately only after every relevant gate is accepted.

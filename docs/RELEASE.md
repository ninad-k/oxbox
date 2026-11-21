# Preparing a native v0.1 release

The native migration uses exact eframe 0.37.0 and Rust 1.95, with all 49 tool routes. Version 0.1.0 is a
release target. The existing release must stay an **unpublished draft** until the acceptance gates pass.
Historical Tauri evidence in [VALIDATION.md](VALIDATION.md) does not certify the native application.

## Native acceptance gates

- [x] Preserve all 49 stable tool IDs and route native forms; keep shared JSON metadata in sync with Pages.
- [x] Pass local core/native tests, all-target clippy with warnings denied, and formatting.
- [x] Observe the actual decorated Windows native launcher, favorites, search, a conversion, favorite hotkey,
  global shortcut hide, single-instance restoration and Quit. This is a bounded subset of manual acceptance.
- [ ] Complete current-commit Windows/macOS/Linux native CI and Rust 1.95 locked checks.
- [ ] Complete native per-tool manual acceptance, including progress, cancellation, destructive confirmations,
  provider failures, power/desktop/recording restoration and hidden notifications.
- [ ] Complete production runtime observations including owned providers and DNS coverage on all three OSes;
  record observation bounds, rather than claim universal zero traffic from finite samples.
- [ ] Inspect actual native MSI/NSIS payloads, version, icon, PE imports, size and hashes; test clean install/uninstall.
- [ ] Verify first-installed-launch startup default, quiet login-to-tray, opt-out/upgrade preservation, paths with spaces,
  OS-disabled choices and ownership-checked uninstall cleanup on a clean Windows account.
- [ ] Verify macOS Intel/Apple Silicon and Linux packages and supported hardware/session behavior.
- [ ] Verify keyboard/screen-reader/IME/Unicode, DPI/monitor changes, high contrast and remote/GPU sessions.
- [ ] Resolve/review dependency findings and outstanding attribution reviews with a human reviewer.
- [ ] Complete independent file-tool path/deletion/archive and media-provider security review.
- [ ] Record final source commit, installer checksums and remote draft-only postcondition.

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
The checked-in notices inventory is Windows-specific; other-target supplements are separate acceptance gates.

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

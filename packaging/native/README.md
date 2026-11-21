# Native packaging

The user approved the native renderer and existing host packages in
[ADR 0004](../../docs/adr/0004-runtime-offline-boundary.md). This directory reuses the already installed
Tauri **CLI as an installer generator**. The native executable depends on eframe and `oxbox-core`;
it does not embed Tauri, Wry or a WebView. The legacy renderer is retained untouched for comparison.
The script checks this distinction in the resolved native runtime/build dependency graph.

Use Rust 1.95+, the existing frozen pnpm install, and the platform build prerequisites used in
[CI](../../.github/workflows/ci.yml). Node 22.17+ is sufficient; the scripts add no dependencies.

```sh
# Regression tests and read-only packaging/graph validation; no compilation or OS effects.
node --test scripts/build-native.test.mjs
node scripts/build-native.mjs --check --target x86_64-pc-windows-msvc

# Build the native release binary only (use the host's target on macOS/Linux).
node scripts/build-native.mjs --target x86_64-pc-windows-msvc

# Build and create unsigned native MSI + NSIS installers on Windows.
node scripts/build-native.mjs --bundle --target x86_64-pc-windows-msvc

# Package an existing release binary produced with the same settings.
node scripts/build-native.mjs --bundle --skip-build --target x86_64-pc-windows-msvc

# Native macOS and GNU/Linux artifacts, built on the matching OS/architecture.
node scripts/build-native.mjs --bundle --target aarch64-apple-darwin
node scripts/build-native.mjs --bundle --target x86_64-apple-darwin
node scripts/build-native.mjs --bundle --target x86_64-unknown-linux-gnu
```

The default output root is `target/native-dist`; `CARGO_TARGET_DIR` overrides it. An explicit Rust
target is always passed, including for the default host. MSVC builds append `+crt-static` only to the
target build so proc-macro/build-script host binaries keep their normal compiler settings. The binary
is at `<output>/<target>/release/oxbox-native[.exe]`; Windows/macOS bundles are under that directory's
`bundle/`. Linux archives are beside the binary. `--bundles msi` or `--bundles nsis` narrows Windows
packaging; macOS accepts `app,dmg` and Linux accepts `tar.gz`.

On Windows, the existing SDK `rc.exe` compiles Oxbox's original ICO and version metadata into a
content-addressed `.res` under the output root. `cargo rustc` links that resource into the native
EXE, giving Explorer and installed shortcuts the same brand as the launcher/tray. Changing the icon
or version changes the resource argument so Cargo rebuilds the host. This adds no Rust dependency
and does not modify a signed executable after linking. Use the script's build path for production;
plain `cargo build` is a development binary without this packaging resource step.

Windows/macOS packaging creates a dependency-free, metadata-only project under the output root,
then runs the existing CLI's `bundle` command against the previously built executable. That project
is not compiled. No legacy Cargo manifest or frontend assets are used or rewritten. The bundle
command disables binary patching, signing and updater artifacts. Its Windows config explicitly
sets `webviewInstallMode: skip` with no minimum WebView version, so neither a WebView bootstrapper nor
a VC Redistributable prerequisite belongs in the installer. Inspect the generated MSI tables/NSIS
script and actual executable imports as part of release acceptance. The existing bundler may fetch
its normal WiX/NSIS build helpers on a fresh build machine; those are build tools, not app prerequisites.

The inspected Tauri CLI 2.12.4 source injects `libwebkit2gtk` dependencies into Linux `.deb`/`.rpm`
settings unconditionally. Native Linux therefore uses the system `tar` utility to package the executable and
licenses into a portable archive. It needs the native GTK, X11/Wayland, graphics and capture libraries
provided by a compatible system. This archive does not establish distribution portability or replace
clean-machine testing. Native `.deb`/`.rpm`/AppImage packaging requires a separately reviewed solution.

Each bundle includes `docs/THIRD_PARTY_NOTICES.txt` and both project licenses. Regenerate the native
target notice with the existing generator before distribution. FFmpeg/Tesseract remain user-installed.
Windows/macOS installed bundles also include `oxbox-installed.json` for the approved first-installed-launch
login policy. The standalone build/QA executable has no marker beside it; Linux portable archives explicitly
omit it, so running a development or portable build does not silently register login startup.

Both Windows installers invoke the native `--uninstall-cleanup` path before executable removal on a
full uninstall. That path opens no GUI/profile and removes the current user's Run value only when it
exactly matches this executable's quoted `--autostart` command. Upgrades preserve startup policy.
MSI links the small [WiX fragment](uninstall.wxs). For NSIS, the script narrowly patches the generated
stock uninstall section to replace its unconditional Run deletion, then recompiles with the same
cached `makensis.exe`. An unknown template layout fails the build instead of silently bypassing
the ownership guard. Compiler/source inspection is separate from clean-machine uninstall acceptance.

[The release workflow](../../.github/workflows/release.yml) runs only on `v*` tags or explicit manual
dispatch for an existing tag whose version matches Cargo. It builds Windows x64, macOS arm64/x64 and
Linux x64, uploads target artifacts, and attaches them to an unpublished prerelease draft. Only its
final draft job has repository write permission; it refuses to alter a published release. Asset
names have a `native-` prefix to distinguish them from historical legacy draft installers.

The macOS matrix uses explicit ARM and Intel runner labels so tests run on the matching architecture;
GitHub documents these labels in its [hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
Installer execution, signing/notarization, platform/hardware checks, accessibility, network measurement,
license review and publication remain explicit release gates. A successful package build does not
mark those gates passed. Nothing in these scripts installs or launches the generated application.

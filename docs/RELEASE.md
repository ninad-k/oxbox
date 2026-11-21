# Preparing a v0.1 release

The source implements the detailed v0.1 plan and user-promoted Phase 5 tools. Version `0.1.0` is the
release target; a draft is a review artifact and does not mean public acceptance or publication.

## Before public publication

- [ ] Review the full source diff and resolved dependency/license changes.
- [x] Complete current-revision local automated checks and [CI at d4e50b7](https://github.com/ninad-k/oxbox/actions/runs/44917613996).
- [ ] Complete and record Windows manual acceptance, including power-tool cancellation and Quit cleanup.
- [ ] Review file-tool path/deletion/extraction security with a human reviewer.
- [ ] Verify macOS Apple Silicon + Intel and Linux builds and supported-tool behavior.
- [x] Capture current-revision launcher screenshots in light/dark and a native tool example, without private data.
- [ ] Resolve observed [WebView2 runtime traffic](RUNTIME_PRIVACY.md), complete ECB opt-in UI acceptance, and measure cold tray startup.
- [ ] Smoke-test Windows installers on a clean machine/VM and confirm uninstall behavior.
- [x] Measure current Windows installers: MSI 4.13 MiB and NSIS 2.86 MiB, below the 15 MB target.
- [x] Inspect current MSI x64/version/payload, verify unsigned status, and prepare SHA-256 checksums for both installers.
- [x] Confirm version agreement in workspace Cargo.toml, package.json, and tauri.conf.json (`0.1.0`).
- [x] Move the reviewed Unreleased changelog into the versioned draft release section.

## Local packaging

```bash
pnpm install --frozen-lockfile
pnpm tauri build
```

Bundles appear in `target/release/bundle/` (or the configured `CARGO_TARGET_DIR`). MSI/NSIS builds may need
build-time downloads of installer tooling and WebView2 bootstrapper resources. This is separate from runtime
offline behavior. Do not install or publish a locally built installer without reviewing it first.

## Draft release workflow

After the checklist is complete, a maintainer can create and push the reviewed version tag, for example `v0.1.0`.
The release workflow builds Windows, Linux, macOS arm64 and x64 installers and attaches them to a **draft**
GitHub release. The manual workflow accepts an existing version tag. Verify that all four matrix jobs succeed,
download and smoke-test the actual assets, then publish the draft deliberately.
Each matrix job regenerates notices for its target; omitted upstream notices must be resolved with attributed,
revision-pinned supplements before that target's packaging can pass. Windows CI verifies its checked-in inventory.
Linux packaging uses Ubuntu 24.04 for compatible PipeWire/SPA headers. Test its runtime-library baseline on
the target distribution; an AppImage does not establish compatibility with older glibc installations.

A maintainer may also prepare a Windows-only draft before platform/manual signoff. Build from the pushed,
reviewed commit, compute SHA-256 checksums, and attach the MSI, NSIS installer and checksum file using
`gh release create v0.1.0 --draft --prerelease --target FULL_COMMIT_SHA --notes-file NOTES_FILE ASSETS...`.
Name every outstanding gate in the notes. A draft must remain unpublished while those gates are open.
Do not push a version tag merely to prepare local Windows assets: a tag push starts the four-platform workflow.
If an existing draft is rebuilt, verify its target commit before replacing assets and record the new hashes.

Builds are unsigned until code signing is configured. Document OS warnings truthfully; do not instruct users to
disable security software. Never put signing certificates, passwords, or release tokens in the repository.
The setup must not claim an updater or signing capability that has not been configured.

## After publication

Add release links/screenshots to README, update the changelog/task checkpoints only after acceptance, and
triage reports with OS/tool details. Promoted Phase 5 task files document the current expanded implementation.
Future additions still need specified safety/platform/dependency criteria; new network features need approval and an issue.

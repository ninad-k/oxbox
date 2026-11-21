# License supplement provenance

Published Rust/npm archives sometimes omit their workspace's license files. `sources.json` maps exact
package versions to checked-in notice texts, immutable upstream URLs and SHA-256 hashes. The generator
verifies every file's containment, hash and UTF-8 content before incorporating it into the inventory.
Existing historical supplements may remain even when their legacy package is no longer a native dependency.

The native Windows inventory is rooted at `oxbox-native` and its selected normal/build target graph.
Lucide-derived vectors retain the conservative `@lucide/svelte` source/dependency/peer notice inventory.
The native executable contains no Tauri host or Svelte browser UI. This inventory is not a legal audit.

## Native additions, 2026-11-21

- AccessKit 0.25.2 / consumer 0.39.2 / Windows 0.35.2 / winit 0.34.2 use the workspace's Apache, MIT
  and Chromium attribution texts at published revision `f3026222559c7f5f46d8f25cb7404de68eb149f0`.
- eframe and egui_system_font_provider 0.37.0 use the published
  `3c6eb8261c3293ad9c7ea1ed58487e038860ebef` workspace licenses.
- Other egui-family 0.37.0 crates recorded `7c0b40402271e8d40cda69f87cf3c2617c2e4382` in their
  published `.cargo_vcs_info.json`. GitHub returned 422 for that object during this review. The exact
  upstream **0.37.0** release tag, annotated tag object `418687fa18fdac8a9b1b6c8f4a2eb1afd2df4c47`,
  resolves to commit `841a328ba2dd8f6996beb7189ebb7fc00e668a2a`. Its workspace license texts are retained
  with this immutable revision and explicit `packageRevision`/`releaseTag` provenance in `sources.json`.
  These files are version-pinned release notices, not a claim to recover the unavailable commit.
- epaint_default_fonts retains Hack's MIT/Bitstream notices, OFL, Ubuntu Font License and the
  emoji-icon-font MIT notice. These four texts were verified byte-for-byte against the published
  0.37.0 crate. Including optional font-source terms conservatively does not assert all fonts are linked.
- gl_generator 0.14.0, khronos_api 3.1.0 and profiling 1.0.18 retain the license text from their published
  VCS revision. Generated API definitions, system fonts and installer/runtime redistributables still
  require the maintainer's corresponding-source and notice review.

Only package sources and primary upstream repositories were used for these additions; no copyright
owners or dates were invented. The generator preserves the existing unresolved attribution review for
`io_tee`, `is-reference` and `locate-character`. A successful generation/check establishes reproducibility
and presence of notice texts; it does not close those human review gates or the GLib security finding.

## Cross-target verification, 2026-11-21

The same locked native dependency graph was inventoried for both macOS architectures and Linux. Outputs
were written beneath `target/`; the checked-in Windows inventory was not regenerated or replaced.

| Rust target | Packages inventoried | Missing notice texts | Explicit upstream reviews |
|---|---:|---:|---:|
| x86_64-pc-windows-msvc | 355 | 0 | 3 |
| aarch64-apple-darwin | 372 | 0 | 29 |
| x86_64-apple-darwin | 374 | 0 | 29 |
| x86_64-unknown-linux-gnu | 541 | 0 | 4 |

Forty exact package/version supplements fill the files omitted from the published macOS/Linux archives:

- AccessKit's macOS, Unix and AT-SPI adapters share the existing published workspace revision and its
  Apache/MIT/Chromium notices. Each entry retains that exact revision and the original notice bytes.
- Older objc2-family packages retain their published revision's complete MIT `LICENSE.txt`. Newer
  revisions provide `LICENSE.md`, which declares available license options and expressly discusses
  uncertainty around Apple SDK-derived works. That complete declaration is preserved, together with
  pinned canonical reference terms. Apache-2.0 is selected where offered; MIT-only crates remain MIT.
  These are **explicit attribution/SDK review items**, not recovered copyright notices or legal clearance.
- `dispatch` and `drm-fourcc` provide MIT declarations in their pinned manifests but no standalone
  copyright/license text. Their manifests and canonical MIT reference terms are retained. Attribution
  remains open; the generated DRM definitions also require upstream header/source review.
- macOS notification bindings and Linux GBM, OpenGL, app-indicator and wayshot bindings retain complete
  license texts from their exact published VCS revisions. The app-indicator repository was verified
  against its pinned `sys/Cargo.toml`, including package name and version.

The 29 macOS reviews comprise the existing three plus 26 newly exposed upstream declaration/SDK cases.
Linux adds the DRM attribution case to the existing three. Zero missing texts means every inventory entry
has a retained source notice or clearly labeled declaration/reference supplement; it does **not** resolve
those review items. No copyright owner or year was inferred from an author list.

Reproduce a target inventory without overwriting Windows notices:

```sh
cargo fetch --locked --target aarch64-apple-darwin
node scripts/generate-third-party-notices.mjs --target aarch64-apple-darwin --output target/native-notices-aarch64-apple-darwin.txt
node scripts/generate-third-party-notices.mjs --target aarch64-apple-darwin --output target/native-notices-aarch64-apple-darwin.txt --check
node scripts/generate-third-party-notices.mjs --check
```

Substitute either other triple and its matching output filename. Fetching/metadata and notice generation
do not compile the application or establish macOS/Linux runtime, installer or licensing acceptance.

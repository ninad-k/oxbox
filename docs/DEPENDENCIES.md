# Dependency policy

`Cargo.lock` and `pnpm-lock.yaml` pin the resolved builds. Use `pnpm install --frozen-lockfile` and Cargo's
`--locked` flag in CI/release work. Ask before adding dependencies outside the tool's approved plan. Check
maintenance, license, platform requirements, binary impact, and runtime network behavior in a dependency PR.
Do not commit downloaded packages, `node_modules`, `target`, installers, or credentials.

| Area | Direct dependencies and purpose |
|---|---|
| Desktop | Tauri 2, dialog/opener/store/autostart/global-shortcut/single-instance/notification plugins |
| UI | Svelte 5, SvelteKit, TypeScript, Vite; Lucide icons imported individually |
| Hashing | sha2, sha1, md-5, blake3 |
| Codes/time | qrcode, barcoders, chrono, chrono-tz; rqrr decodes generated QR codes in tests |
| Files | walkdir, trash, dirs, zip 9.8 (patched parser), tar, flate2, sevenz-rust2 |
| System | sysinfo, keepawake, target-specific windows-sys |
| Capture/media/privacy | xcap, arboard, image (PNG/JPEG/WebP), age, rqrr/rxing; Windows COM/WinRT via windows |
| Shared/test | serde, serde_json, thiserror, tempfile, vitest, svelte-check |

The full transitive graph and third-party licenses should be reviewed before each public release. Package
manifests are authoritative for versions/features and licenses; do not assume every transitive dependency shares
Oxbox's dual license. Generate a machine-readable graph with `cargo metadata --locked --format-version 1` and
inspect UI licenses with `pnpm licenses list`. Do not paste local absolute-path metadata into public release notes.

No HTTP library is added. The opted-in currency provider invokes system curl with a fixed HTTPS URL, no curl
configuration file, no redirects, a timeout and a response-size ceiling. Other tools do not access remote services.
FFmpeg/Tesseract are user-managed optional local providers; their versions, codecs, device support and licenses
belong to those installations. No provider binary is copied into Oxbox bundles. Transitive desktop/build tooling
may contain network support; source review and an observed default-offline runtime check remain release gates.

Useful primary references: [Tauri plugins](https://v2.tauri.app/plugin/), [image](https://docs.rs/image/),
[age](https://docs.rs/age/), [sysinfo](https://docs.rs/sysinfo/), [xcap](https://docs.rs/xcap/),
[chrono-tz](https://docs.rs/chrono-tz/), and [sevenz-rust2](https://docs.rs/sevenz-rust2/).

## RustSec assessment — 2026-11-21

This assessment records findings rather than suppressing them. It used `cargo audit --json` with advisory
database commit `55e1c3dfbd834237a05bbbf7b50ba3b5be27b662`, reverse dependency trees from the locked workspace,
and source inspection of the resolved crates. Run the audit again after every lockfile update. No advisory
ignore list or audit exit-code override was added. ZIP remediation is a separate release dependency change;
an informational advisory assessment does not turn a failing vulnerability audit into a passing one.

### Unsoundness findings

**GLib 0.18.5 — [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).** The affected
`VariantStrIter` iterator passes an immutable pointer as a mutable C out-argument, which can cause undefined
behavior and null-pointer crashes in optimized builds. The resolved path is
`oxbox → tauri 2.12.7 / tauri-runtime-wry 2.12.5 → wry 0.57.3 / gtk 0.18.2 / webkit2gtk 2.0.2 → glib 0.18.5`;
tray/menu components also share this GTK family. `cargo tree --target x86_64-pc-windows-msvc -i glib@0.18.5`
and the macOS equivalent contain no path. It is a Linux desktop dependency in Oxbox's supported target set,
not part of the Windows installers being prepared.

Oxbox does not call `VariantStrIter` or `array_iter_str`. Searches also found no such calls in the resolved
Tauri, Wry, Tao, Muda, tray-icon, GTK, WebKitGTK and GIO source packages. That narrows identified exposure;
it is **not an exhaustive reachability proof for the Linux framework** and does not fix the dependency.
Patched GLib releases are `>=0.20.0`, while GTK 0.18.2 requires `glib ^0.18`; 0.18.5 remains the latest 0.18
release checked. Adding a parallel newer GLib dependency, updating the system C library, or a routine
`cargo update` cannot replace the affected Rust binding in that path. Maintainers must track a compatible
upstream GTK/WebKit/Tauri migration or review a narrowly scoped upstream-source backport before treating
Linux acceptance as closed. Windows-only draft assets do not establish Linux acceptance.

**LRU 0.16.4 — [RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html).** The original
path was `oxbox → oxbox-core → rqrr 0.10.1 → lru 0.16.4`. RQRR is used at runtime by Barcode Reader as well
as by QR tests; this was not merely a development dependency. The advisory requires a panicking key
destructor during `LruCache::pop`, unwinding/catching that panic, and subsequent cache use. RQRR's cache
has `u8` keys and its preparation code uses `get`, `put`, `iter` and `pop_lru`, rather than `pop`; a `u8`
destructor cannot panic. Oxbox's release profile also uses `panic = "abort"`. Those facts mean the stated
trigger was not identified on the inspected application path, without asserting that all cache behavior
is proven safe.

The remediation upgraded the existing approved dependency to RQRR 0.11.0, which requires `lru ^0.18`
and Rust 1.85.1. The final locked reverse tree resolves **LRU 0.18.5**, above the patched minimum 0.18.2;
the affected 0.16.4 instance is absent. QR round-trip/decode checks, the Rust 1.93 locked check, and the
final audit are recorded in [VALIDATION.md](VALIDATION.md). No separate direct LRU dependency was added.
The advisory remains visible in this history, but no longer matches the resolved LRU version.

### Maintenance warnings and follow-up

These are upstream maintenance advisories, not evidence that the following code is currently exploitable.
They remain actionable upkeep and must be revisited with new advisories and releases.

| Resolved package and advisory | Current path / exposure | Maintainer follow-up |
|---|---|---|
| `chrono 0.4.45` ([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html)); `chrono-tz 0.10.4` ([RUSTSEC-2026-0555](https://rustsec.org/advisories/RUSTSEC-2026-0555.html)) | Direct runtime dependencies for World Time, Alarm, Date Countdown, rate dates and media naming; RXing also uses Chrono. All versions are marked unmaintained, with no patched version. | Plan an approved migration to a maintained time/time-zone stack and an updated RXing dependency path. Preserve DST gap/fold, weekday, serialization and rate-date tests. Any new direct crate name still needs approval under the dependency policy. A patch update alone cannot close these advisories. |
| `basic-toml 0.1.10` ([RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374.html)) | `age 0.11.5 → i18n-embed-fl / i18n-embed-impl → i18n-config → basic-toml`; build-time localization configuration, not a runtime parser for encrypted user files. | Track migration by the localization/age maintainers to the maintained TOML parser. Age 0.12.1 still resolves the same i18n-config 0.4 family through its newer localization macros, so that upgrade alone is not a demonstrated fix. |
| `proc-macro-error 1.0.4` ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html)) | Linux GTK/glib procedural-macro build dependencies. | Coordinate replacement with the GTK/Tauri dependency migration; do not force an incompatible macro crate into the old dependency requirement. |
| `proc-macro-error2 2.0.1` ([RUSTSEC-2026-0173](https://rustsec.org/advisories/RUSTSEC-2026-0173.html)) | `age → i18n-embed-fl`; build-time macros on all targets. Cargo also reports a future-compatibility notice for this version. | Track a maintained localization-macro replacement upstream. The newer i18n-embed-fl 0.10.0 metadata still requests this package; replacing the original deprecated macro package with this fork is no longer a maintenance solution. |
| `zbus_names 4.3.5` ([RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html)); `zvariant 5.15.1` ([RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html)); `zvariant_derive 5.15.1` ([RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html)) | Linux D-Bus paths through Zbus 5.19.0, keepawake, xcap and Tauri notification/opener/single-instance plugins; the derive package runs during builds. | These standalone crates moved into Zbus 6 (`names`, `wire` and derive APIs). Track compatible upstream upgrades across the owning libraries. Updating only one transitive package cannot satisfy the old Zbus 5 API/dependency requirements. Recheck Linux awake, capture, notifications and single-instance behavior after migration. |

Reproduce reverse-path inspection with `cargo tree --locked --target all -i PACKAGE@VERSION`; compare it
with a concrete target before making a platform-specific claim. Primary package metadata was checked through
the [RQRR package](https://crates.io/crates/rqrr), [GTK package](https://crates.io/crates/gtk), and
[Age package](https://crates.io/crates/age). The table's advisories link to the upstream notices and migration
references. No local dependency source or operating-system package was patched during this assessment.

## Reproducing Windows third-party notices

After installing approved dependencies from frozen lockfiles, run:

```bash
node scripts/generate-third-party-notices.mjs --target x86_64-pc-windows-msvc
node scripts/generate-third-party-notices.mjs --check --target x86_64-pc-windows-msvc
```

The generator uses locked, offline Cargo metadata and the installed UI dependency sources. It does not
download packages or change lockfiles. It writes [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt), with
package names, versions, license expressions, versioned source links, original copyright/license/NOTICE
texts, and lockfile hashes. Cargo normal/build dependencies are labeled by use; the UI inventory includes
Svelte and SvelteKit runtime sources even though their packages are development dependencies. Package-level
build helpers are conservatively included, so this is not a claim that every inventoried module is shipped.
Installed optional UI packages depend on the generation host; prepare Windows notices from the Windows
build environment. The checked-in file is bundled with the Windows app together with Oxbox's own licenses.

Some published package archives omit their upstream monorepo license files. The checked-in
`docs/third-party-licenses/sources.json` records supplements from exact upstream revisions, their original
URLs, and SHA-256 hashes verified on every generation. Bech32's complete license/copyright source header
and Selectors' MPL source header are preserved. MPL-covered packages include explicit source archive and
upstream repository references. Canonical license reference terms are identified separately from an
upstream package's own attribution.

**Open human attribution gate:** `io_tee 0.1.1`, `is-reference 3.0.3`, and `locate-character 3.0.0` provide
license declarations but no standalone copyright/license notice in their published/pinned sources.
Their exact declarations and canonical reference terms are included, without inventing a copyright owner
or date. Each is flagged in the generated document for maintainer review before public publication; a
successful `--check` proves reproducibility, not resolution of that gate or completion of a legal audit.
FFmpeg/Tesseract are not redistributed. OS/runtime and installer-toolchain notices need separate review.

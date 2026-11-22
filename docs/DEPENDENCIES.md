# Dependency policy

`Cargo.lock` and `pnpm-lock.yaml` pin the resolved builds. Use `pnpm install --frozen-lockfile` and Cargo's
`--locked` flag in CI/release work. Ask before adding dependencies outside the tool's approved plan. Check
maintenance, license, platform requirements, binary impact, and runtime network behavior in a dependency PR.
Do not commit downloaded packages, `node_modules`, `target`, installers, or credentials, except for
the three explicitly approved source backports described below. That exception does not permit
unrelated vendoring or a new dependency/version/backend.

| Area | Direct dependencies and purpose |
|---|---|
| Native desktop | eframe 0.37.0 with accesskit/clipboard/default_fonts/system_fonts/glow/wayland/x11; rfd, tray-icon, global-hotkey, auto-launch, open; native notification adapters |
| Icon/legacy sources | ISC-licensed Lucide vectors copied into the native catalogue; Svelte/Tauri sources remain for comparison, outside the native binary graph |
| Hashing | sha2, sha1, md-5, blake3 |
| Codes/time | qrcode, barcoders, approved Jiff 1.0.2 time/time-zone migration; historical Chrono/Chrono-TZ paths are tracked below; rqrr/rxing decode local barcodes at runtime and in tests |
| Files | walkdir, trash, dirs, zip 9.8 (patched parser), tar, flate2, sevenz-rust2 |
| System | sysinfo, keepawake, target-specific windows-sys |
| Capture/media/privacy | xcap, arboard, image (PNG/JPEG/WebP), Age 0.11.5 (0.12.1 candidate held at its security gate), rqrr/rxing; Windows COM/WinRT via windows |
| Unix provider lifecycle | libc =0.2.191, user-approved as a direct Unix-only dependency on 2026-11-22; typed waitid(WNOWAIT) preserves the group leader identity until descendant cancellation |
| Shared/test | serde, serde_json, thiserror, tempfile, vitest, svelte-check |

The full transitive graph and third-party licenses should be reviewed before each public release. Package
manifests are authoritative for versions/features and licenses; do not assume every transitive dependency shares
Oxbox's dual license. Generate a machine-readable graph with `cargo metadata --locked --format-version 1` and
inspect UI licenses with `pnpm licenses list`. Do not paste local absolute-path metadata into public release notes.

No HTTP library is added. The opted-in currency provider invokes system curl with a fixed HTTPS URL, no curl
configuration file, no redirects, a timeout and a response-size ceiling. Other tools do not access remote services.
FFmpeg/Tesseract are user-managed optional local providers; their versions, codecs, device support and licenses
belong to those installations. No provider binary is copied into Oxbox bundles. The native Windows normal
dependency graph contains no Tauri host, Wry, WebView2 or common HTTP client crate. The similarly named
`tauri-winrt-notification` is a standalone WinRT toast wrapper, not a Tauri host. A dependency graph alone does
not prove zero network activity. Historical WebView2 traffic and its supported SDK limits motivated the
[native migration](adr/0004-runtime-offline-boundary.md); production runtime measurements, DNS coverage and
platform acceptance remain separate release gates. See [runtime privacy findings](RUNTIME_PRIVACY.md).

Useful primary references: [Tauri plugins](https://v2.tauri.app/plugin/), [image](https://docs.rs/image/),
[age](https://docs.rs/age/), [sysinfo](https://docs.rs/sysinfo/), [xcap](https://docs.rs/xcap/),
[Jiff](https://docs.rs/jiff/1.0.2/), and [sevenz-rust2](https://docs.rs/sevenz-rust2/).

## Accepted source remediation — 2026-11-22, validation in progress

The user approved phases 1 and 2 of [ADR 0005](adr/0005-dependency-security-remediation.md), including
Age 0.12.1, Jiff 1.0.2 and the named vendoring-policy exception. The selected eframe 0.37.0, AccessKit,
all 56 tools, application data and offline defaults remain requirements. Macro/tray/Zbus migrations
described as separate options in the ADR are **not approved or implemented by these changes**.

**Security gate:** the tested Age 0.12.1 candidate introduced `crypto-bigint 0.5.5` through mandatory
P-256/HPKE dependencies, matching [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html).
Its published dependency bounds cannot select the fixed family. The application therefore retains
exact Age 0.11.5. [ADR 0006](adr/0006-age-upgrade-security-gate.md) proposes a separate, narrow localization
remedy; its extra source exception needs approval before implementation. The fresh held-candidate audit
reports zero vulnerability entries and six maintenance warnings, without ignores; patched GLib is omitted
by the scanner's path-package handling and remains explicitly accounted for in source provenance.

The exception permits only complete, original-license-preserving published source for these packages,
with the approved minimal upstream-derived changes. The original registry cache is not modified.

| Patched package, version unchanged | Applied source change | Exact modified source tree SHA-256 |
|---|---|---|
| GLib 0.18.5 | Mutable C out-pointer correction from gtk-rs `05dff0ee696f9bcd8617cd48c4b812d046d440cb` | `a42d6ad83dcdb52c0df284fedd8d373cfa0d346ee48a529aed524d86aeea617d` |
| i18n-config 0.4.8 | Maintained TOML parser replacement, extracted from cargo-i18n `2ce7b1dfe9a8cc224288bf50cbc2a67a6de863c4` | `0afbebac8143a9bb0c85f51a83ec5c5bdf99f37934dff898d6a8acc4d96495a4` |
| RXing 0.9.4 | Optional Chrono and standard-clock decoder timestamps from `912b1083ffd164b29e6b244e4845de9df06b107a`; client/WASM support preserved | `b4f75fef1931c796cb21ea3ffebf08736599390da7963eb8b71b058f739261aa` |

[Vendor provenance](../vendor/PROVENANCE.json) records the verified original archive URLs/checksums,
published VCS revisions, exact applied patch artifacts and hashes, changed-file before/after hashes,
and complete final file manifests. [Vendor instructions](../vendor/README.md) define reconstruction and
the tree-hash algorithm. Preserve source LF bytes and every recorded file, including upstream package
lockfiles and Markdown files ignored by the packages' original `.gitignore` files. Check a fresh checkout;
do not silently regenerate integrity records to accept missing, edited or reformatted upstream source.

```sh
node --test scripts/vendor-integrity.test.mjs scripts/license-inventory.test.mjs
node scripts/verify-vendor.mjs
```

The five generated integrity fixtures and nine notice fixtures passed locally. The verifier checks 572
package files and five source/patch artifacts, refuses links/path escapes and unexpected bytes, and
does no network or package execution. It is an integrity check, not a security or legal audit. A
coordinated change to source and provenance still requires review. No advisory ignore or version spoof
is authorized: GLib's reviewed source fix does not turn version 0.18.5 into a published fixed release,
and a path package being omitted by an audit tool must not become a clean-audit claim.

The held Age 0.12.1 candidate resolved i18n-embed-fl 0.10.1 and proc-macro-error3 rather than the old
macro-error2 path; the separate retained i18n-config backport replaces basic-toml. The integrator completed
Core compilation and native tests with the candidate API before its audit failed. Generated 196,731-byte
binary fixtures passed both Age 0.11.5 → candidate Core and candidate Core → Age 0.11.5 directions,
with wrong-password, corruption,
truncation, actual stream cancellation, exclusive-output and source-preservation guards. Production
encryption still emits factor 18 and decryption refuses factors above 20. This is bounded passphrase-file
compatibility evidence, not acceptance of every age recipient, plugin, armor or high-factor legacy file.

A [small checked-in historical fixture and producer](../crates/oxbox-core/tests/fixtures/age-0.11.5/README.md)
make the old-writer direction reproducible in Core's normal `legacy_age_0115_` tests, using factor 10
only for generated test data. Both current-Core tests passed in the integrator's Rust 1.95 Windows
workspace run, with no failures or ignored tests. The existing production
round-trip test retains the factor-18 header assertion without adding another heavy KDF here. The
[standalone parser compatibility workspace](../tests/dependency-compat/README.md) passed all five generated
valid/invalid/missing Cargo and localization configurations, Unicode/default values and explicit parent
fallback checks, with zero failures or ignored tests. The minimally patched upstream i18n-config emits
two pre-existing lifetime-syntax warnings under Rust 1.95; they are not suppressed or edited out of
the source-pinned package. These validation workspaces are not application dependencies.

Jiff's approved feature selection includes a bundled offline IANA fallback on every target. Time/date
adapters, persisted values, DST gaps/folds and rates need the final migration checks before removing
historical direct dependencies. RXing's optional Chrono/Chrono-TZ client/WASM capabilities are retained.
Cargo may keep optional packages in its lockfile even after compiled application paths stop using them;
record native/legacy target trees separately from lockfile/audit entries. Do not prune upstream features
to improve an audit count. Linux AccessKit/AT-SPI, keepawake and capture still have the separately tracked
Zbus 5 maintenance path; this work does not remove or replace accessibility to silence it.

**Final post-resolution audit counts, full all-platform tests/Clippy/builds, optimized Linux GLib iterator
execution, regenerated notices and new production artifact acceptance remain pending.** Earlier CI and
runtime hashes do not certify this changed dependency build. Clean-machine and human attribution gates
remain open. Updated validation belongs in [VALIDATION.md](VALIDATION.md) when actual receipts arrive.

## Historical RustSec assessment — 2026-11-21

This assessment records findings rather than suppressing them. It used `cargo audit --json` with advisory
database commit `55e1c3dfbd834237a05bbbf7b50ba3b5be27b662`, reverse dependency trees from the locked workspace,
and source inspection of the resolved crates. Re-run against the native migration lockfile completed with
**zero vulnerability entries, eight maintenance warnings, and the GLib unsoundness warning below**. This
is not a clean-audit claim. The lockfile also preserves legacy and other-target packages, so compare each
finding with the concrete native target graph. Run the audit again after every lockfile update. No advisory
ignore list or audit exit-code override was added.

This section preserves the pre-remediation assessment. The 2026-11-22 pre-change audit, using database
commit `d1fe6ac3eba823a3515e849045f36dc34543885d`, again reported zero vulnerability entries, eight
unmaintained packages and one GLib unsoundness finding. Neither receipt is the final result of the
accepted changes above.

### Unsoundness findings

**GLib 0.18.5 — [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).** The affected
`VariantStrIter` iterator passes an immutable pointer as a mutable C out-argument, which can cause undefined
behavior and null-pointer crashes in optimized builds. The native Linux path includes
`oxbox-native → gtk 0.18.2 → glib 0.18.5`; tray/menu components share this GTK family. The retained legacy
host also reaches it through Tauri/Wry/WebKitGTK. The selected native Windows normal/build tree contains
no GLib. It remains a Linux desktop dependency in Oxbox's supported target set and an open release gate.

Oxbox does not call `VariantStrIter` or `array_iter_str`. Searches also found no such calls in the resolved
Tauri, Wry, Tao, Muda, tray-icon, GTK, WebKitGTK and GIO source packages. That narrows identified exposure;
it is **not an exhaustive reachability proof for the Linux framework** and does not fix the dependency.
Patched GLib releases are `>=0.20.0`, while GTK 0.18.2 requires `glib ^0.18`; 0.18.5 remains the latest 0.18
release checked. Adding a parallel newer GLib dependency, updating the system C library, or a routine
`cargo update` cannot replace the affected Rust binding in that path. Maintainers must track a compatible
upstream GTK migration or review a narrowly scoped upstream-source backport before treating
Linux acceptance as closed. Windows-only draft assets do not establish Linux acceptance.

**LRU 0.16.4 — [RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html).** The original
path was `oxbox → oxbox-core → rqrr 0.10.1 → lru 0.16.4`. RQRR is used at runtime by Barcode Reader as well
as by QR tests; this was not merely a development dependency. The advisory requires a panicking key
destructor during `LruCache::pop`, unwinding/catching that panic, and subsequent cache use. RQRR's cache
has `u8` keys and its preparation code uses `get`, `put`, `iter` and `pop_lru`, rather than `pop`; a `u8`
destructor cannot panic. The historical release profile used `panic = "abort"`; native jobs now use unwind
so owned worker panics can be joined and reported. The remediation below, rather than a panic-profile
assumption, removes the affected cache version. The historical trigger was not identified on the inspected
RQRR path, without asserting that all cache behavior is proven safe.

The remediation upgraded the existing approved dependency to RQRR 0.11.0, which requires `lru ^0.18`
and Rust 1.85.1. The final locked reverse tree resolves **LRU 0.18.5**, above the patched minimum 0.18.2;
the affected 0.16.4 instance is absent. QR round-trip/decode checks, the Rust 1.93 locked check, and the
final audit are recorded in [VALIDATION.md](VALIDATION.md). No separate direct LRU dependency was added.
The advisory remains visible in this history, but no longer matches the resolved LRU version.

### Maintenance warnings and follow-up

These are upstream maintenance advisories, not evidence that the following code is currently exploitable.
They remain actionable upkeep and must be revisited with new advisories and releases.

| Baseline package and advisory | Pre-remediation path / exposure | Follow-up at the baseline |
|---|---|---|
| `chrono 0.4.45` ([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html)); `chrono-tz 0.10.4` ([RUSTSEC-2026-0555](https://rustsec.org/advisories/RUSTSEC-2026-0555.html)) | Runtime time/rate-date APIs and RXing. All versions are marked unmaintained, with no patched version. | Plan an approved migration to a maintained time/time-zone stack and an updated RXing dependency path. Preserve DST gap/fold, weekday, serialization and rate-date tests. ADR 0005 subsequently approved the Jiff and RXing changes above; a patch update alone cannot close these advisories. |
| `basic-toml 0.1.10` ([RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374.html)) | `age 0.11.5 → i18n-embed-fl / i18n-embed-impl → i18n-config → basic-toml`; build-time localization configuration, not a runtime parser for encrypted user files. | Track migration by the localization/age maintainers to the maintained TOML parser. Age 0.12.1 still resolves the same i18n-config 0.4 family through its newer localization macros, so that upgrade alone is not a demonstrated fix. |
| `proc-macro-error 1.0.4` ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html)) | Linux GTK/glib procedural-macro build dependencies. | Coordinate replacement with the GTK/Tauri dependency migration; do not force an incompatible macro crate into the old dependency requirement. |
| `proc-macro-error2 2.0.1` ([RUSTSEC-2026-0173](https://rustsec.org/advisories/RUSTSEC-2026-0173.html)) | `age → i18n-embed-fl`; build-time macros on all targets. Cargo also reports a future-compatibility notice for this version. | The baseline review checked i18n-embed-fl 0.10.0, which still requested this package. Subsequent inspection identified 0.10.1's maintained macro-error3 replacement, selected above. Replacing the original deprecated macro package with macro-error2 is no longer a maintenance solution. |
| `zbus_names 4.3.5` ([RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html)); `zvariant 5.15.1` ([RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html)); `zvariant_derive 5.15.1` ([RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html)) | Linux D-Bus paths through AccessKit/AT-SPI, keepawake and xcap; legacy Tauri integrations also remain locked. The derive package runs during builds. | These standalone crates moved into Zbus 6 (`names`, `wire` and derive APIs). Track compatible upstream upgrades across the owning libraries. Updating only one transitive package cannot satisfy the old Zbus 5 API/dependency requirements. Preserve accessibility; recheck affected Linux behavior after migration. |

Reproduce reverse-path inspection with `cargo tree --locked --target all -i PACKAGE@VERSION`; compare it
with a concrete target before making a platform-specific claim. Primary package metadata was checked through
the [RQRR package](https://crates.io/crates/rqrr), [GTK package](https://crates.io/crates/gtk), and
[Age package](https://crates.io/crates/age). The table's advisories link to the upstream notices and migration
references. No local dependency source or operating-system package was patched during that historical
assessment. The separately approved 2026-11-22 source backports are recorded above.

## Reproducing Windows third-party notices

After installing approved dependencies from frozen lockfiles, run:

```bash
node scripts/generate-third-party-notices.mjs --target x86_64-pc-windows-msvc
node scripts/generate-third-party-notices.mjs --check --target x86_64-pc-windows-msvc
```

The generator uses locked, offline Cargo metadata, an active normal/build Cargo tree rooted at
`oxbox-native`, and installed Lucide source dependencies. Inactive optional dependencies in broad metadata
are excluded using the active target tree. Nine Node regressions cover native selection, missing or
ambiguous roots, conservative icon-source roots, tree parsing, alternate output and patched-source
identity/provenance refusal; run `node --test scripts/license-inventory.test.mjs`.
The generator does not
download packages or change lockfiles. It writes [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt), with
package names, versions, license expressions, versioned source links, original copyright/license/NOTICE
texts, and lockfile hashes. Cargo normal/build dependencies are labeled by use. The conservative npm inventory
starts at `@lucide/svelte`, whose vectors are copied into native assets, and includes its dependency/peer sources
(including Svelte); these do not imply an embedded Svelte/browser UI. Unused legacy Tauri JS and SvelteKit
are not application roots. Package-level helpers are conservatively included, so this is not a claim that
every inventoried module is shipped. Installed optional packages depend on the generation host; prepare notices from the Windows
build environment. The checked-in file is bundled with the Windows app together with Oxbox's own licenses.
Use `--output target/notices-TARGET.txt` when probing another target so the checked-in Windows inventory
remains intact. Output paths are constrained to the workspace; this option does not fetch missing packages.

Before invoking Cargo or reading license files, the generator checks every approved patched package
against the offline integrity verifier and binds its Cargo name/version/path to that record. Patched
entries identify the modified project directory, final tree SHA-256, provenance SHA-256, original
immutable archive and checksum, and applied patch/upstream references. An original registry archive
is labeled as a reconstruction base, never as the modified shipped source. Project branch URLs are
clearly mutable locators whose bytes must match the recorded hashes; the generator does not read Git
HEAD or create a self-referential notice identity. No local workspace/profile paths are published.

Some published package archives omit their upstream monorepo license files. The checked-in
`docs/third-party-licenses/sources.json` records supplements from exact upstream revisions, their original
URLs, and SHA-256 hashes verified on every generation. Bech32's complete license/copyright source header
and Selectors' MPL source header are preserved. MPL-covered packages include explicit source archive and
upstream repository references. Canonical license reference terms are identified separately from an
upstream package's own attribution. Native renderer, AccessKit, GL binding generator and profiling supplements
retain their pinned workspace licenses. Embedded default-font notices retain Hack/Bitstream, Ubuntu Font
License, emoji-icon-font MIT and OFL terms; the four checked-in font notices match their published 0.37.0
crate bytes. See the [supplement provenance notes](third-party-licenses/README.md) for the exact egui release-tag
fallback when a published VCS revision was no longer available upstream.

**Open human attribution gate:** `io_tee 0.1.1`, `is-reference 3.0.3`, and `locate-character 3.0.0` provide
license declarations but no standalone copyright/license notice in their published/pinned sources.
Their exact declarations and canonical reference terms are included, without inventing a copyright owner
or date. Each is flagged in the generated document for maintainer review before public publication; a
successful `--check` proves reproducibility, not resolution of that gate or completion of a legal audit.
FFmpeg/Tesseract are not redistributed. OS/runtime and installer-toolchain notices need separate review.

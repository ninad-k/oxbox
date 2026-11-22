# ADR 0005: Remediate dependency findings without weakening the native boundary

- Status: **Accepted for phases 1 and 2 — user approval recorded 2026-11-22; Age 0.12.1 held by its failed security gate**
- Date: 2026-11-22
- Scope: the selected eframe/egui 0.37.0 application on Rust 1.95, shared core, and retained Tauri comparison host
- Supersedes: no existing decision; [ADR 0004](0004-runtime-offline-boundary.md) and the [dependency policy](../DEPENDENCIES.md) remain in force

This decision separates fixing defective source, removing an obsolete dependency, and passing a version-based
advisory check. These are different outcomes. Preserve all 56 tools, AccessKit, offline defaults, existing
data, provider cancellation and the unpublished release. The user explicitly approved phases 1 and 2,
including the three named source patches and vendoring exception, on 2026-11-22. The macro/tray options
and the larger Zbus migration below remain unapproved. Approval is not a completed validation receipt.

The Age 0.12.1 candidate compiled and passed generated interoperability tests, but the fresh audit
introduced `crypto-bigint 0.5.5` / [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)
through its mandatory P-256/HPKE dependencies. There is no compatible version-only update within their
published requirements. The release candidate therefore retains the preceding exact Age **0.11.5**;
this supersedes the implementation step below until a separate remedy is reviewed. The approved GLib,
i18n-config, RXing and Jiff work continues. The failed upgrade is evidence, not a fixed security finding
or permission to add another vendor patch. See [the proposed remedy](0006-age-upgrade-security-gate.md).

## Evidence and constraints

The baseline `cargo audit --json` receipt uses advisory database commit
`d1fe6ac3eba823a3515e849045f36dc34543885d` and reports **zero vulnerability entries, eight unmaintained
packages and one GLib unsoundness finding**, with no ignored advisories. This is not a clean audit.
Concrete Windows MSVC and Linux GNU normal/build reverse trees were inspected separately for both hosts.
The local receipts are under ignored `target/security-accessibility/`; they must be regenerated after changes.

| Finding | Actual selected dependency path | Consequence for remediation |
|---|---|---|
| GLib 0.18.5, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) | Native Linux: direct GTK 0.18.2 and tray-icon 0.25.1 → Muda 0.20 / libappindicator 0.9 → GTK/GLib. Retained Linux host also uses GTK through Tauri 2.12.7, Tao 0.37.3, Wry 0.57.3 and WebKit2GTK 2.0.2. | Windows native normal/build paths do not activate it. A native-only tray change does not fix or remove the retained host's instance. |
| `proc-macro-error` 1.0.4, [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | Linux GLib macros 0.18.5 **and** GTK3 macros 0.18.2 | Both build-time paths must change to remove the package. |
| `proc-macro-error2` 2.0.1, [RUSTSEC-2026-0173](https://rustsec.org/advisories/RUSTSEC-2026-0173.html) | Age 0.11.5 → i18n-embed-fl 0.9.4, for both hosts and all targets | A published Age/localization update can replace this macro dependency. |
| `basic-toml` 0.1.10, [RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374.html) | Age localization macros → i18n-config 0.4.8 | Build-time localization configuration, rather than parsing encrypted user files. Age's update alone does not remove it. |
| Chrono 0.4.45 and Chrono-TZ 0.10.4, [RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html) / [0555](https://rustsec.org/advisories/RUSTSEC-2026-0555.html) | Shared time/rate-date logic, native adapters, retained adapters and RXing 0.9.4 | All versions are marked unmaintained. Updating their patch versions cannot resolve the findings. |
| `zbus_names` 4.3.5, `zvariant` 5.15.1 and `zvariant_derive` 5.15.1, [RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html) / [0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html) / [0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html) | Native Linux AccessKit/AT-SPI plus core keepawake 0.6.1 and xcap 0.8.3 → Zbus 5.19.0; retained integrations add paths. Derives also execute at build time. | The standalone packages moved into Zbus 6. Their final standalone versions have no maintained patch replacement. |

Current GTK 0.19.0 uses GLib 0.22, and libappindicator 0.10 uses GTK 0.19. Those releases require Rust
1.92, within the selected compiler. However, current Muda and the retained Tauri/WebKitGTK stack still
request the GTK 0.18 family. Adding GTK 0.19 or GLib 0.22 alongside it cannot replace that old family or
interchange its Rust types. See the primary [GTK 0.19 dependencies](https://crates.io/api/v1/crates/gtk/0.19.0/dependencies),
[libappindicator 0.10 dependencies](https://crates.io/api/v1/crates/libappindicator/0.10.0/dependencies)
and [Muda manifest](https://github.com/tauri-apps/muda/blob/dev/Cargo.toml).

## Accepted phase 1 — bounded remediation

Approval covers these three changes together. Keep eframe, renderer features, tool IDs and platform
effects unchanged. Root owns manifests, lockfile resolution, sequential checks and integration.

### 1. Backport the exact GLib pointer correction

Use the verified published GLib **0.18.5** source as the base. Its crates.io archive checksum is
`233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`; its published VCS revision is
`42b9caf98e03ded086362d9653ca58fe94dc8658`, subdirectory `glib`.

Apply only the relevant change from official gtk-rs commit
[`05dff0ee696f9bcd8617cd48c4b812d046d440cb`](https://github.com/gtk-rs/gtk-rs-core/commit/05dff0ee696f9bcd8617cd48c4b812d046d440cb)
in `src/variant_iter.rs`: declare the C out-pointer mutable and pass `&mut p`. This fixes the identified
undefined behavior while preserving the GTK 0.18 API family. The original patch was merged in
[gtk-rs PR 1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343); the advisory's published fixed versions
start at 0.20.0. The current upstream 0.18 branch still contains the old pointer call.

The approved workspace override is a source patch, not a replacement version:

```toml
[patch.crates-io]
glib = { path = "vendor/glib-0.18.5" }
```

This would apply to both native and retained Linux hosts. Keep the package version 0.18.5 and original
MIT license; record the base archive, upstream commit, precise diff and final tree hashes. Do not label
the source 0.20, alter advisory version ranges, or suppress the finding. A version-based audit may still
report GLib; report it as a reviewed source backport only after verification, with that limitation visible.

Before applying the change, exercise the real string-array iterator at optimization using generated
values and `next`, `nth`, `last`, `next_back` and `nth_back`. Linux can reach the existing GTK re-export
without adding a new application dependency. Record the baseline behavior and the patched result;
do not manufacture a failure if a particular compiler does not reproduce the undefined behavior.

### 2. Update the existing Age dependency to 0.12.1

Change core's Age requirement to **`=0.12.1`**, retaining its existing default/no-extra-feature selection.
Resolve **i18n-embed-fl 0.10.1**, which requests maintained **proc-macro-error3 `^3.0`**; the current published
resolution candidate is **3.1.1**. This should remove the sole identified `proc-macro-error2` path across
both hosts, subject to a fresh lockfile and reverse-tree check. No new direct Oxbox dependency is needed.
Primary [Age metadata](https://crates.io/api/v1/crates/age/0.12.1),
[localization macro dependencies](https://crates.io/api/v1/crates/i18n-embed-fl/0.10.1/dependencies) and
[proc-macro-error3 metadata](https://crates.io/api/v1/crates/proc-macro-error3/3.1.1).

Age 0.12.1 declares Rust 1.74, below 1.95. Its [release notes](https://github.com/str4d/rage/blob/v0.12.1/age/CHANGELOG.md)
describe the localization update and non-exhaustive public error enums. Oxbox's used streaming and
passphrase interfaces require a compile/API check, not an assumption of complete semver compatibility.
Keep encryption factor 18, decryption ceiling 20, existing age file format, exclusive outputs, receipts,
wrong-passphrase errors, corruption/truncation behavior and cancellation. Do not enable plugins, SSH,
networking or a new cryptographic format as part of this update.

### 3. Backport i18n-config's maintained TOML parser

The latest published **i18n-config 0.4.8** still requests `basic-toml ^0.1`. Its archive checksum is
`3e06b90c8a0d252e203c94344b21e35a30f3a3a85dc7db5af8f8df9f3e0c63ef`; its VCS base is
`ae88dc79fda061e1bbf96b8d8e8fe8d23f1e52e7`, subdirectory `i18n-config`.

Upstream commit
[`2ce7b1dfe9a8cc224288bf50cbc2a67a6de863c4`](https://github.com/kellpossible/cargo-i18n/commit/2ce7b1dfe9a8cc224288bf50cbc2a67a6de863c4)
already replaces it with maintained `toml ^1.1.6`. Extract that parser change rather than importing the
commit's unrelated workspace/edition/dependency overhaul: replace the dependency, two `from_str` calls
and the public parse-error type with `toml::de::Error`. The baseline lock already contains TOML 1.2.2,
but actual unification and enabled features must be checked after resolution.

The approved second workspace patch is:

```toml
[patch.crates-io]
# Together with the GLib entry above, in one table.
i18n-config = { path = "vendor/i18n-config-0.4.8" }
```

Retain version 0.4.8, MIT attribution, base/source hashes and patch provenance. Test generated valid,
invalid and missing localization/Cargo configurations, and compile the actual Age localization macros.
The error type changes, so downstream compilation is part of acceptance. This is a source backport,
not a claim that the unpublished upstream workspace has released a new compatible package.

### Explicit accepted source-policy exception

The dependency policy normally says not to commit downloaded packages. The user's explicit approval of
this ADR permits only the minimal complete, license-preserving upstream package sources needed for
GLib 0.18.5, i18n-config 0.4.8 and the phase-2 RXing 0.9.4 patch. A checked-in provenance record must include original archive checksums,
immutable upstream references, every downstream change, license/NOTICE files, a deterministic integrity
check and removal criteria when compatible fixed releases become available. Do not modify Cargo's shared
registry cache, include build artifacts, use moving Git branches, or vendor unrelated packages.

This exception records the user's approval; it does not cover additional macro, tray or Zbus source
packages discussed below. Third-party notices must identify patched sources accurately. A path package
being omitted or treated differently by a scanner must never become a clean-audit claim.

## Accepted phase 2 — maintained time and time zones

Add direct **Jiff 1.0.2** where time APIs are needed, with this exact approved feature selection:

```toml
jiff = { version = "=1.0.2", default-features = false, features = [
  "std", "tz-system", "tz-fat", "tzdb-bundle-always",
  "tzdb-zoneinfo", "tzdb-concatenated", "perf-inline",
] }
```

This retains system-time/local-zone support and historical timezone transitions, with an embedded IANA
fallback on every target. Zoneinfo and concatenated databases read local OS data; this selection enables
no runtime network client, remote database fetch, JavaScript clock or new serialization format. Jiff
1.0.2 declares Rust 1.85 and Unlicense OR MIT.
Its [published features](https://crates.io/api/v1/crates/jiff/1.0.2) and [upstream project](https://github.com/BurntSushi/jiff)
are the primary references. The user approved this direct package and its embedded timezone-data scope.

Replace Chrono/Chrono-TZ use in shared `alarm`, `world_time` and `currency`, native time/currency forms,
and retained time/currency adapters. Preserve serialized zone IDs, alarm schedules, weekday numbering,
rate-date strings and imported preferences. Specify equivalent behavior for ambiguous/nonexistent local
times before editing. Keep generated DST gap/fold, offsets, midnight, leap-day, invalid-zone and date
fixtures; compare old/new results and existing persisted fixtures. Do not reset users' saved clocks.

RXing 0.9.4 also requests Chrono even with Oxbox's `client_support` disabled. Its upstream commit
[`912b1083ffd164b29e6b244e4845de9df06b107a`](https://github.com/rxing-core/rxing/commit/912b1083ffd164b29e6b244e4845de9df06b107a)
already makes Chrono optional and uses `SystemTime` for decoder timestamps, preserving Chrono for
client date parsing and WASM. This exact revision still declares 0.9.4; later master declares an
unreleased 0.10.0. Prefer the small pinned upstream change against published 0.9.4, not moving master.
The base archive checksum is `6fbf2a0e464c316b30b4cab99e898f20a9d8e42ca3d7a767e670c9a330f2202f`.

A local source patch for RXing is included in the user's approval and must retain its provenance. Preserve current
decoder features and QR/EAN/Code128 generated decode fixtures; do not disable barcode capabilities to
drop Chrono. Only when both direct use and the active RXing path are gone can the two maintenance
findings be described as removed from the resolved workspace.

Check active graphs separately from the full lockfile. Cargo resolves a broader feature graph for
`Cargo.lock` than the features actually compiled; RXing's preserved optional client/WASM support may
therefore retain Chrono packages in the lock or audit after normal application paths stop using them.
Record the actual outcome without deleting upstream optional capabilities or calling active-path
absence a warning-free lockfile. [Cargo's resolver documentation](https://doc.rust-lang.org/cargo/reference/resolver.html#features).

## Separately approveable macro and tray options

Both GLib macros 0.18.5 and GTK3 macros 0.18.2 use Syn 2 and `proc-macro-error` 1. A simple manifest alias
to proc-macro-error3 is **not established as compatible**: its attribute expansion uses the absolute
`::proc_macro_error3::entry_point` path, so preserving only the old dependency name does not provide that
name. The maintained fork [documents Syn 2 support](https://docs.rs/proc-macro-error3/3.1.1/proc_macro_error3/)
via `default-features = false, features = ["syn2-error"]`; its
[attribute implementation](https://github.com/gamma0987/proc-macro-error3/blob/main/proc-macro-error-attr/src/lib.rs)
shows the generated path. A real port would rename imports/dependencies in **both** macro packages and
test successful expansions plus invalid-input diagnostics. This is a downstream adaptation, not a GTK
upstream backport identified during this review.

GTK upstream instead removed the GLib macro dependency by propagating `syn::Result` in commit
[`c74a40a16b23a1b9c8e67842d32d02bc187f551d`](https://github.com/gtk-rs/gtk-rs-core/commit/c74a40a16b23a1b9c8e67842d32d02bc187f551d).
It changes multiple macro implementations and does not alone remove GTK3 macros' separate path. Evaluate
that larger upstream-derived backport separately; do not claim phase 1 removes this warning.

For the native host alone, an alternative is **tray-icon `=0.26.1`, default features disabled**, with
Linux `ksni` enabled, removal of the direct GTK dependency, and removal of GTK initialization/pumping in
`integration.rs`. This keeps the public menu API and uses a GTK-free StatusNotifier backend. The existing
0.25.1 already offers KSNI, but 0.26 fixes startup before the panel's StatusNotifierWatcher exists, which
matters for launch at login. See the [backend documentation](https://github.com/tauri-apps/tray-icon/blob/tray-icon-v0.26.1/README.md)
and [versioned changelog](https://github.com/tauri-apps/tray-icon/blob/tray-icon-v0.26.1/CHANGELOG.md).

This alternative needs explicit version/backend approval, session-start/menu/Stop/Quit tests and Linux
desktop acceptance. KSNI itself still uses Zbus 5. It removes native GTK paths only; the retained host
still requires the phase-1 GLib fix and retains its macro warning. It is not required for phase 1.

## Protected accessibility and unresolved Zbus maintenance path

Preserve eframe **0.37.0** and AccessKit. Their active Linux path includes AccessKit-Winit 0.34.2 →
AccessKit-Unix 0.24.1 / AccessKit-AT-SPI-Common 0.21.1 → AT-SPI 0.29 and Zbus 5.19. These are the current
published AccessKit versions checked. Even AT-SPI 0.30 / common-proxies 0.14 still use the old Zbus/name/
variant family: [AT-SPI dependencies](https://crates.io/api/v1/crates/atspi/0.30.0/dependencies),
[common dependencies](https://crates.io/api/v1/crates/atspi-common/0.14.0/dependencies),
[AccessKit-Unix dependencies](https://crates.io/api/v1/crates/accesskit_unix/0.24.1/dependencies).

The standalone name/wire/derive crates were folded into Zbus 6; upstream's
[migration record](https://github.com/z-galaxy/zbus/issues/2043) confirms their final releases and API
changes. Updating xcap alone, or using the already selected notify-rust Zbus 6 path, cannot remove
AccessKit's and keepawake's remaining Zbus 5 instances. A complete change requires coordinated owning
library migrations/backports and Linux accessibility, awake, capture and tray acceptance. There is no
identified published, compatible version-only update that closes all three findings while preserving
the exact renderer and accessibility configuration.

Do not disable AccessKit, rename deprecated crates, force incompatible Zbus versions under old APIs,
or add audit exceptions to present these maintenance findings as fixed. Phase 1 and the time migration
do not close this path. Any larger migration requires another explicit implementation decision.

## Validation and release consequences

The recorded approval covers only phases 1 and 2, the exact package selections and named source-policy
exception. Implementation acceptance still requires:

1. Record generated regression baselines before each behavioral patch, and verify patched source identity.
2. Resolve and inspect native **and retained** normal/build target graphs; re-run the advisory database
   without ignores and explain remaining warnings, including version-matched patched GLib.
3. Run Rust 1.95 fmt, strict all-target Clippy, core/native tests and production builds on Windows,
   macOS and Linux. Check retained-host compilation and comparison tests where its shared dependencies changed.
4. Verify Age interoperability/streaming/resource limits and, if selected, persisted time/DST/barcode
   compatibility. Check source-derived native binary graph still excludes a browser/automatic network client.
5. Regenerate notices, provenance/integrity checks, package metadata and installer/runtime receipts for
   the new artifact. Existing artifact hashes and measurements do not certify a changed dependency build.

Phase 1 is expected to fix the identified GLib source defect and remove two unmaintained packages,
subject to actual resolution/tests. It is not expected to produce a warning-free full lockfile.
Clean-machine installation/uninstallation, native OS/hardware effects, Linux/macOS runtime accessibility,
DNS/broker/short-flow observation limits, unsigned release handling and human license/security reviews
remain separate gates. Keep the release a draft until its actual acceptance record permits publication.

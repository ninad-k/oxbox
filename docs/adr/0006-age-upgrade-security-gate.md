# ADR 0006: Hold the Age upgrade and separate localization maintenance

- Status: **Proposed — the fourth vendor package is not approved or applied**
- Date: 2026-11-22
- Scope: retain published Age 0.11.5 for passphrase files; propose only the upstream localization diagnostic-macro change
- Related decision: [ADR 0005](0005-dependency-security-remediation.md), whose Age 0.12.1 selection failed the post-resolution security gate

## Finding and current safe hold

The approved Age 0.12.1 update compiled and passed generated-format compatibility checks, but the
subsequent audit found **crypto-bigint 0.5.5, [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)**.
That vulnerability concerns timing variability in arithmetic intended to be constant-time; downstream
secret values may be exposed through observable side channels. The advisory's published fixed range
is `>=0.7.6`. A passing functional test does not test constant-time arithmetic.

Age 0.12.1 unconditionally introduces both `hpke ^0.12` and `p256 ^0.13`. The selected path is
`Age 0.12.1 / Age-Core 0.12.0 → HPKE 0.12.0 / P256 0.13.2 → elliptic-curve 0.13.8 → crypto-bigint ^0.5`.
The newest published compatible crypto-bigint 0.5 is still 0.5.5. A normal lock update cannot replace
it with the incompatible 0.7 family; adding a parallel fixed version leaves the affected instance.
Primary [Age dependencies](https://crates.io/api/v1/crates/age/0.12.1/dependencies),
[HPKE dependencies](https://crates.io/api/v1/crates/hpke/0.12.0/dependencies) and
[elliptic-curve dependencies](https://crates.io/api/v1/crates/elliptic-curve/0.13.8/dependencies).

The selected Oxbox passphrase path constructs only the scrypt recipient/identity, and refuses
non-passphrase headers before decryption. Source review identified no use of P256/HPKE arithmetic by
that path. This narrows identified exposure; it is not an exhaustive side-channel proof and does not
resolve the dependency finding. The release gate failed regardless. Do not suppress the advisory.

The integrator therefore restored exact published **Age `=0.11.5`**, which preserves the previously
selected passphrase implementation and avoids the introduced backend. Using the same advisory database
commit `d1fe6ac3eba823a3515e849045f36dc34543885d`, the recorded Age 0.12.1 trial reports **one vulnerability
and five maintenance findings**, while the held Age 0.11.5 resolution reports **zero vulnerabilities
and six maintenance findings**. These are different preserved receipts, not a warning-free claim.
The held build reintroduces the macro-error2 maintenance path described below. GLib is a local patched
source omitted by the scanner and still requires its explicit source-advisory record.

Production scrypt factor **18**, decryption ceiling **20**, streaming, exclusive outputs, transient
passphrases, receipts and original preservation remain unchanged. All 56 tools, eframe 0.37.0, Rust
1.95, AccessKit and the existing GLib/i18n-config/RXing patches remain selected. No new cryptographic
primitive, protocol, recipient type or renderer is proposed here.

## Proposed smallest maintenance change

Keep Age `=0.11.5` and its compatible **i18n-embed-fl 0.9.4 / i18n-embed 0.15** API family. If the user
approves a fourth named source package, backport only the diagnostic-macro dependency/import change
from official cargo-i18n commit
[`dd46d747fe5b319d5de6b2364a2394c50ab82f54`](https://github.com/kellpossible/cargo-i18n/commit/dd46d747fe5b319d5de6b2364a2394c50ab82f54).
That upstream change replaces proc-macro-error2 with proc-macro-error3, retaining the imported
`abort`, `emit_error` and `proc_macro_error` API. It was published in i18n-embed-fl 0.10.1.

| Source identity | Proposed pinned value |
|---|---|
| Package/version, unchanged | `i18n-embed-fl 0.9.4` |
| Published base archive | `https://static.crates.io/crates/i18n-embed-fl/i18n-embed-fl-0.9.4.crate` |
| Published archive SHA-256 | `04b2969d0b3fc6143776c535184c19722032b43e6a642d710fa3f88faec53c2d` |
| Published VCS revision/subdirectory | `ceb3da0ee3acf91b17a7a52e02642267ddb47a3d` / `i18n-embed-fl` |
| Upstream change | `dd46d747fe5b319d5de6b2364a2394c50ab82f54` |
| Published 0.10.1 VCS containing the change | `d3c20dcc62967abd3d7a8129d6c8a957e2c14173` / `i18n-embed-fl` |
| License/notice to retain | MIT, original `LICENSE.txt` and package attribution |
| Proposed source directory | `vendor/i18n-embed-fl-0.9.4` |

These source identities were checked against published metadata and cached published-package VCS
records. Before applying the patch, download the listed base archive and independently verify its
checksum. Metadata is not a claim that the new source tree already exists or has passed integrity checks.
Primary [0.9.4 release metadata](https://crates.io/api/v1/crates/i18n-embed-fl/0.9.4) and
[0.10.1 source](https://github.com/kellpossible/cargo-i18n/tree/d3c20dcc62967abd3d7a8129d6c8a957e2c14173/i18n-embed-fl).

Only three published-package files would change:

1. `Cargo.toml`: replace the normalized `proc-macro-error2` dependency with the exact maintained
   `proc-macro-error3 =3.1.1`, default features disabled, `syn2-error` enabled.
2. `Cargo.toml.orig`: make the same dependency change in its original manifest form.
3. `src/lib.rs`: rename the one imported crate from `proc_macro_error2` to `proc_macro_error3`.

The precise proposed changes are:

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
-[dependencies.proc-macro-error2]
-version = "2.0.1"
+[dependencies.proc-macro-error3]
+version = "=3.1.1"
+default-features = false
+features = ["syn2-error"]
--- a/Cargo.toml.orig
+++ b/Cargo.toml.orig
-proc-macro-error2 = "2.0.1"
+proc-macro-error3 = { version = "=3.1.1", default-features = false, features = ["syn2-error"] }
--- a/src/lib.rs
+++ b/src/lib.rs
-use proc_macro_error2::{abort, emit_error, proc_macro_error};
+use proc_macro_error3::{abort, emit_error, proc_macro_error};
```

The fork's current default error-conversion feature targets Syn 3; the proposed package retains Syn 2,
so it uses the fork's explicit supported
[`syn2-error` selection](https://docs.rs/proc-macro-error3/3.1.1/proc_macro_error3/).
This exact pin/feature choice is the small downstream adaptation to upstream's `proc-macro-error3 ^3.0`
change. It is not a fake package alias: the actual dependency and import name become
`proc_macro_error3`, matching the attribute macro's generated absolute crate path.

The candidate workspace addition, only after approval, is:

```toml
[patch.crates-io]
# Added to the existing patch table; do not duplicate the table.
i18n-embed-fl = { path = "vendor/i18n-embed-fl-0.9.4" }
```

## Compatibility assessment and limits

Read-only comparison of published 0.9.4 and 0.10.1 confirms that their source contains the same three
diagnostic macro names and exported `fl!` entry point. The 0.10 line also changes its i18n-embed
requirement to 0.16 and includes unrelated argument-order/internal changes. **Do not import those
changes or replace the complete 0.9 package with 0.10.1.** The proposed extraction leaves 0.9.4's
parser, localization loader, Fluent/config integration, argument handling, generated application API
and all runtime cryptography intact. Compilation and negative-diagnostic fixtures still must establish
actual compatibility; source inspection is not their passing receipt.

The proposal requests a fourth explicit exception to the normal no-vendoring rule, limited to this
package. Preserve its full published source, unchanged version and MIT notice. Extend
`vendor/PROVENANCE.json`, its integrity fixtures and patched-source notices with verified base/final
tree hashes, the exact three-file diff and immutable upstream references. Exclude it from application
workspace auto-membership; do not modify the registry cache, spoof versions or add an audit ignore.
The existing three-package approval does not authorize this fourth package by itself.

## Acceptance before using the patch

- Capture the current Age 0.11.5 graph/audit and a generated valid localization macro fixture first.
  Add meaningful valid and compile-fail fixtures for message IDs, missing/wrong named arguments and
  malformed macro input. Preserve useful diagnostics; a panic is not the desired failure mode.
- After approval and implementation, build the actual Age 0.11.5 localization call sites and run
  generated macro fixtures on Rust 1.95. Re-run persistent legacy passphrase tests and the existing
  production factor-18/maximum-20/streaming/receipt tests, without repeatedly adding heavy KDF cases.
- Verify every new source/patch byte and notice identity; rerun the audit without ignores. Demonstrate
  removal of the active macro-error2 path and absence of the introduced crypto-bigint instance in the
  actual resolved application graph. Keep optional lockfile and retained-host findings distinct.
- Complete selected native/legacy checks and three-platform CI, notices and fresh artifact/runtime
  receipts. Neither the successful Age 0.12.1 format trial nor an older binary hash certifies this build.

The expected outcome is removal of one maintenance path while retaining the safer historical crypto
dependency selection. Actual post-patch audit counts remain pending. Chrono's optional lock path,
GLib's explicit source finding, GTK macro-error1, and protected AccessKit/Zbus maintenance work are not
resolved by this proposal. Clean-machine, hardware, privacy and human attribution gates remain open;
the release remains an unpublished draft.

## Alternatives

1. **Keep the unmodified Age 0.11.5 localization dependency for now.** This needs no fourth vendor
   exception. Retain the macro-error2 maintenance warning honestly and track an upstream compatible
   release. It does not reintroduce the Age 0.12.1 timing-vulnerability dependency path.
2. **Migrate the full RustCrypto stack for Age 0.12.1.** Current fixed-family packages exist, but Age's
   dependency bounds and APIs require coordinated owning-crate changes; upstream master contains a
   broader unpublished migration. This would need another source/cryptographic review and explicit
   approval. It is not selected by this narrow proposal.
3. **Suppress the new vulnerability or present the passphrase-only observation as its fix.** Rejected.
   Preserve the failed trial's audit and explain the actual held selection instead.

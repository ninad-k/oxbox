# ADR 0006: Hold the Age upgrade and separate localization maintenance

- Status: **Accepted — user approved the fourth vendor package on 2026-11-22; implemented, with automated and scoped binary validation completed; manual release gates remain open**
- Date: 2026-11-22
- Scope: retain published Age 0.11.5 for passphrase files; apply only the upstream localization diagnostic-macro change
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

## Accepted smallest maintenance change

Keep Age `=0.11.5` and its compatible **i18n-embed-fl 0.9.4 / i18n-embed 0.15** API family. The user
approved a fourth named source package: backport only the diagnostic-macro dependency/import change
from official cargo-i18n commit
[`dd46d747fe5b319d5de6b2364a2394c50ab82f54`](https://github.com/kellpossible/cargo-i18n/commit/dd46d747fe5b319d5de6b2364a2394c50ab82f54).
That upstream change replaces proc-macro-error2 with proc-macro-error3, retaining the imported
`abort`, `emit_error` and `proc_macro_error` API. It was published in i18n-embed-fl 0.10.1.

| Source identity | Approved pinned value |
|---|---|
| Package/version, unchanged | `i18n-embed-fl 0.9.4` |
| Published base archive | `https://static.crates.io/crates/i18n-embed-fl/i18n-embed-fl-0.9.4.crate` |
| Published archive SHA-256 | `04b2969d0b3fc6143776c535184c19722032b43e6a642d710fa3f88faec53c2d` |
| Published VCS revision/subdirectory | `ceb3da0ee3acf91b17a7a52e02642267ddb47a3d` / `i18n-embed-fl` |
| Upstream change | `dd46d747fe5b319d5de6b2364a2394c50ab82f54` |
| Published 0.10.1 VCS containing the change | `d3c20dcc62967abd3d7a8129d6c8a957e2c14173` / `i18n-embed-fl` |
| License/notice to retain | MIT, original `LICENSE.txt` and package attribution |
| Approved source directory | `vendor/i18n-embed-fl-0.9.4` |

These source identities were checked against published metadata and cached published-package VCS
records. The base archive checksum and modified tree were verified during implementation, as recorded
below. Reconstruction must independently verify the listed archive checksum before applying the patch.
Primary [0.9.4 release metadata](https://crates.io/api/v1/crates/i18n-embed-fl/0.9.4) and
[0.10.1 source](https://github.com/kellpossible/cargo-i18n/tree/d3c20dcc62967abd3d7a8129d6c8a957e2c14173/i18n-embed-fl).

Only three published-package files may change:

1. `Cargo.toml`: replace the normalized `proc-macro-error2` dependency with the exact maintained
   `proc-macro-error3 =3.1.1`, default features disabled, `syn2-error` enabled.
2. `Cargo.toml.orig`: make the same dependency change in its original manifest form.
3. `src/lib.rs`: rename the one imported crate from `proc_macro_error2` to `proc_macro_error3`.

The precise approved changes are:

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

The fork's current default error-conversion feature targets Syn 3; the selected package retains Syn 2,
so it uses the fork's explicit supported
[`syn2-error` selection](https://docs.rs/proc-macro-error3/3.1.1/proc_macro_error3/).
This exact pin/feature choice is the small downstream adaptation to upstream's `proc-macro-error3 ^3.0`
change. It is not a fake package alias: the actual dependency and import name become
`proc_macro_error3`, matching the attribute macro's generated absolute crate path.

The approved workspace addition is:

```toml
[patch.crates-io]
# Added to the existing patch table; do not duplicate the table.
i18n-embed-fl = { path = "vendor/i18n-embed-fl-0.9.4" }
```

## Compatibility assessment and limits

Read-only comparison of published 0.9.4 and 0.10.1 confirms that their source contains the same three
diagnostic macro names and exported `fl!` entry point. The 0.10 line also changes its i18n-embed
requirement to 0.16 and includes unrelated argument-order/internal changes. **Do not import those
changes or replace the complete 0.9 package with 0.10.1.** The selected extraction leaves 0.9.4's
parser, localization loader, Fluent/config integration, argument handling, generated application API
and all runtime cryptography intact. The generated compilation and negative-diagnostic results below
establish compatibility for those cases; source inspection alone would not establish it.

The user approved a fourth explicit exception to the normal no-vendoring rule, limited to this
package. Preserve its full published source, unchanged version and MIT notice. Extend
`vendor/PROVENANCE.json`, its integrity fixtures and patched-source notices with verified base/final
tree hashes, the exact three-file diff and immutable upstream references. Exclude it from application
workspace auto-membership; do not modify the registry cache, spoof versions or add an audit ignore.
The original three-package approval did not authorize this fourth package; its separate approval is
recorded here. No further package, GTK macro, tray or Zbus source changes are authorized by this decision.

## Implementation record — local and CI validation completed

The approved fourth archive was downloaded and matched its recorded checksum before extraction.
Its ten published files are retained, with only the three approved files changed. The modified source
tree SHA-256 is `93079287d7c1f6aa203286f57c1dc5d52defb263965064bbe2b05f7b8803270c`, and the applied
patch SHA-256 is `5ba3d03e3dc8b4c2d341461f70411dbc0fb14f31ff3a0ea5355fb4b92dfe9f37`.
The updated [provenance](../../vendor/PROVENANCE.json) retains the base and final source identities.
All three earlier package trees remain unchanged.

The integrator captured the **original registry 0.9.4** baseline: generated plain/named/attribute
messages executed correctly, and all four invalid fixtures produced the expected diagnostics without
a build panic. An initial runner attempt stopped before compilation because it confused Cargo's
`(proc-macro)` kind annotation with a source path; the harness classifier was corrected after a failing
fixture and its four pure guards passed. That harness issue is not an application macro defect.

The generic offline integrity verifier passed for 582 package files and six artifacts; its six generated
guards, nine notice guards and six classifier guards passed locally. The integrator then activated the
patch and captured the same **five successful patched fixtures**: valid plain/named/attribute runtime
output and four expected compiler diagnostics, with no build panic. The original and patched outcomes
therefore agree for these generated cases.

The fresh approved-resolution audit, using database `d1fe6ac3eba823a3515e849045f36dc34543885d`, reports
**zero vulnerability entries and five maintenance findings**, without ignores: optional Chrono,
GTK macro-error1 and three Zbus-family packages. Macro-error2, basic-toml, Chrono-TZ and crypto-bigint
are absent from the root lockfile. The scanner omits local patched GLib; its source fix/advisory remains
explicitly recorded and is not silently counted as an unaffected registry release. Regenerated Windows
notices report **358 packages, zero missing notice texts and three unresolved human attribution reviews**.
The earlier held/trial counts above remain separate historical receipts.
The resumed final audit reproduced the selected zero-vulnerability/five-maintenance result using the
same advisory database, and `pnpm audit` reported zero findings.

For source `8a62ec1ad7e65ad9b3378dbcb495b897ef725c47`, the Rust **1.95.0** Windows workspace run passed **294 Core tests, five frozen
time-comparison tests, 150 native tests and 12 retained legacy tests**, with zero failures or ignored
tests. Strict workspace Clippy, formatting and **34 Node checks** passed. This includes the persistent
legacy passphrase tests and the existing production factor-18/maximum-20/streaming/receipt checks.

[CI run 45039126970](https://github.com/ninad-k/oxbox/actions/runs/45039126970) passed all five jobs for
that exact source. Core/frozen-comparison/native results were **294/5/150 on Windows, 289/5/152 on
macOS ARM and 289/5/153 on Linux**, with zero failed or ignored tests. Each target also passed all five
localization-configuration tests, the five patched macro fixtures, strict Clippy and production build
on Rust 1.95.0. The optimized Linux GLib regression passed one test with 152 filtered and none failed or
ignored. The exact Windows production artifact and bounded desktop checks are recorded in
[DEPENDENCIES.md](../DEPENDENCIES.md); those checks do not establish complete manual acceptance.

## Acceptance record and remaining gates

- Completed locally: the original registry macro baseline and patched comparison, each with one valid
  runtime fixture and four expected compile errors; the selected Age 0.11.5 call sites and persistent
  passphrase/production-policy tests; source/patch integrity and notice identity checks; and the
  no-ignore audit and resolved-graph checks recorded above.
- Completed: the selected native/legacy workspace checks and final three-platform CI on Rust 1.95.0,
  static checks of the six current staged assets, and bounded desktop checks. The existing release draft,
  ID **453727915**, was refreshed to the exact checked source after verified backup of old assets/notes;
  all six replacements were downloaded and matched, and local/downloaded static checks passed with zero
  errors. No installer was executed. See the
  [verification record](../verification/2026-11-22-dependency-accessibility.md#final-approved-source--8a62ec1).
  Optional lockfile findings and retained-host paths remain distinct from active production paths.
- Pending: clean-machine installation, full keyboard and assistive-technology
  acceptance, and remaining hardware/privacy checks. In particular, Ctrl+Shift+Q left the process alive
  but hid the window for an unexplained reason, and specific Narrator speech reports remain pending.
  Neither the successful Age 0.12.1 format trial nor an older binary hash certifies this build.

The observed post-patch outcome removes the macro-error2 maintenance path while retaining the
published Age 0.11.5 crypto dependency selection. Chrono's optional lock path,
GLib's explicit source finding, GTK macro-error1, and protected AccessKit/Zbus maintenance work are not
resolved by this decision. Clean-machine, hardware, privacy and the three human attribution reviews
remain open; the release remains an unpublished draft without a real release tag.

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

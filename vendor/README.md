# Approved dependency source patches

The user approved these three package source patches and the named vendoring-policy exception on
2026-11-22, in [ADR 0005](../docs/adr/0005-dependency-security-remediation.md). These are upstream
open-source packages, with their original versions, license files and copyright headers preserved.
No downloaded executable or build artifact belongs here. The normal Cargo registry cache is unchanged.

| Package | Published base | Approved change |
|---|---|---|
| `glib` | 0.18.5 | Exact mutable C out-pointer correction from gtk-rs commit `05dff0ee696f9bcd8617cd48c4b812d046d440cb`; preserves GTK 0.18 types. |
| `i18n-config` | 0.4.8 | Only the maintained TOML parser replacement from cargo-i18n commit `2ce7b1dfe9a8cc224288bf50cbc2a67a6de863c4`; no unrelated workspace modernization. |
| `rxing` | 0.9.4 | Exact optional-Chrono/standard-clock change from commit `912b1083ffd164b29e6b244e4845de9df06b107a`; preserves decoder features and optional client/WASM behavior. |

`PROVENANCE.json` records each official crates.io archive URL and verified SHA-256, the published VCS
revision, immutable upstream fixes, an exact applied diff, before/after hashes for changed files, the
pristine tree digest, and the final complete file manifest. `patches/` contains the actual downstream
diffs against the published archives. Normalized `Cargo.toml` files and their `.orig` counterparts are
updated together where dependencies/features changed. Other published package bytes are retained,
including original package lockfiles; those lockfiles are provenance, not Oxbox's resolution authority.
The root `Cargo.lock` and its source overrides control application resolution.

Run from the repository root:

```sh
node --test scripts/vendor-integrity.test.mjs
node scripts/verify-vendor.mjs
```

Verification reads local files only; it does not download packages, execute package code or modify the
manifest. It rejects changed, extra, missing or renamed package files, inconsistent metadata, symlinks/
junctions, special files and changed patch artifacts. The tree digest is SHA-256 over UTF-8 records in
lexicographic relative-path order: `<file sha256> <decimal byte length> <JSON-quoted POSIX path>\n`.
It binds filenames and file bytes, not filesystem permissions or a package's security properties.
Reviewing a coordinated edit to the source and its recorded hashes remains a maintainer responsibility.

For independent reconstruction, download the exact archive URLs in `PROVENANCE.json`, verify each base
archive digest before extraction, refuse links/escaping entries, then apply that package's recorded
diff with its package directory as the working directory. The final manifests must pass the command
above. Git must preserve source LF bytes (`vendor/.gitattributes`) so Windows checkout conversion does
not silently invalidate these source hashes. Do not regenerate hashes to bless unexplained changes.

GLib's original version still matches RUSTSEC-2024-0429. A source backport does not turn it into 0.20 or
make a version-only audit clean. Keep the advisory and reviewed source provenance visible; never spoof
a version, add an audit ignore, or rely on a local/path package being omitted by a scanner. The original
upstream iterator tests remain present; optimized Linux execution is separate from the recorded source
baseline and the Node integrity fixtures. No UB failure is claimed merely from inspecting the old code.

Remove a patch when an approved, compatible maintained release replaces every relevant dependency
path, after regressions, all-target graphs, notices and new artifact acceptance are recorded. Keep
AccessKit and the selected eframe 0.37 renderer unchanged. The separate GTK macro/tray/Zbus proposals
remain unapproved, and their maintenance warnings are not resolved by these packages.

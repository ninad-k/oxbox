# Dependency source compatibility checks

This standalone Rust 1.95 test workspace validates the approved i18n-config 0.4.8 TOML parser backport
without adding a direct application dependency or changing vendored package bytes. It runs generated
valid/invalid/missing Cargo and localization configurations, Unicode/optional values, and explicit
parent fallback behavior. Every file is confined to a new temporary directory. It does not invoke a
provider, inspect a user profile or mutate environment/OS settings.

From the repository root, use the integrator's serial Cargo slot:

```sh
cargo +1.95.0 test --manifest-path tests/dependency-compat/Cargo.toml -- --nocapture
```

Initial resolution creates this isolated workspace's lockfile; retain it and use `--locked` afterward.
Its packages are validation-only, outside the application workspace and production notice roots. Do not
add this harness to the application's workspace/dependencies or mistake its resolution for the app graph.
Check `node scripts/verify-vendor.mjs` separately before running these tests. A test result is parser
compatibility evidence, not a claim that the vendor patch or the overall audit has no remaining findings.

Persistent Age 0.11.5 → current-core compatibility fixtures live under
[`crates/oxbox-core/tests/fixtures/age-0.11.5`](../../crates/oxbox-core/tests/fixtures/age-0.11.5/README.md).
Run `cargo +1.95.0 test -p oxbox-core --locked legacy_age_0115_` for those. Their public test passphrase
and low-factor legacy file are test data; application encryption remains factor 18 / decrypt maximum 20.

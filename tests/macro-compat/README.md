# Generated localization macro comparison

This separate Rust 1.95 workspace validates the approved i18n-embed-fl 0.9.4 diagnostic dependency
backport without adding application dependencies. It preserves i18n-embed 0.15.4, generated Fluent
messages, the public `fl!` entry point, named arguments and attributes. Four intentionally invalid
binary targets must produce their expected compiler diagnostics, never a build panic or unrelated
dependency/setup error. Do not run `cargo test --all-targets` here: those targets intentionally fail.

The committed manifest and lockfile select the **approved patched** i18n-embed-fl 0.9.4. The normal
contributor/CI command is:

```sh
node scripts/check-macro-compat.mjs --expect patched
```

The integrator recorded the original registry baseline **before** adding this override, then ran the
same five fixtures with the patched source. Both runs passed: one valid-runtime target exercises plain,
named and attribute messages, while four invalid targets emit the expected errors without a build
panic. The final source override is:

```toml
i18n-embed-fl = { path = "../../vendor/i18n-embed-fl-0.9.4" }
```

Retain the final isolated lockfile and run the patched case in CI. The runner invokes typed
`cargo +1.95.0 tree/run/check` arguments with
`--locked --offline`, verifies which package source actually compiled, and prints sanitized outcomes.
It does not fetch packages or edit locks. Each compiler call has a five-minute bound; run it only in
the integrator's designated slot. Pure runner guards can run separately with
`node --test scripts/macro-compat.test.mjs` and invoke no compiler.

To independently repeat the historical registry comparison, make a **disposable copy** at ignored
`target/macro-compat-original`: copy this directory's `Cargo.toml`, `i18n.toml`, `src/` and `i18n/`,
omitting its lockfile and build `target/`. In the copied manifest, remove only the i18n-embed-fl
`[patch.crates-io]` entry. Keep the i18n-config patch; its `../../vendor/...` path is still correct at
that specified copy depth. The approved application and committed test manifests and registry cache
remain unchanged.

Resolve the copy's separate lockfile, verify that its i18n-embed-fl source is the registry release,
then run the same targets manually using the copied manifest:

```sh
cargo +1.95.0 generate-lockfile --manifest-path target/macro-compat-original/Cargo.toml --offline
cargo +1.95.0 tree --manifest-path target/macro-compat-original/Cargo.toml --locked --offline -p i18n-embed-fl --depth 0
cargo +1.95.0 run --manifest-path target/macro-compat-original/Cargo.toml --locked --offline --bin valid
```

The tree must identify registry 0.9.4, not a local package path; its lock entry must retain published
checksum `04b2969d0b3fc6143776c535184c19722032b43e6a642d710fa3f88faec53c2d`. For each of
`unknown_message`, `missing_named_argument`, `unexpected_named_argument` and `malformed_input`, run
`cargo +1.95.0 check --manifest-path target/macro-compat-original/Cargo.toml --locked --offline --bin NAME`.
Each must fail with its intended diagnostic, not a panic or setup error; the expected patterns are in
`scripts/check-macro-compat.mjs`. The normal runner intentionally stays pinned to the committed
**patched** workspace, so do not use its `--expect original` mode on the final manifest. The original
mode exists for capturing a genuine pre-activation baseline, not for silently changing dependencies.
If historical packages are not cached, explicitly fetch this disposable workspace's locked developer
dependencies before the offline checks; neither the fixtures nor the application request network data.

The valid fixture executes only generated local messages. Invalid cases cover an unknown message,
one missing argument in a message with two named arguments, an unexpected named argument and malformed
macro syntax. Supplying one of the two required arguments deliberately exercises 0.9.4's existing
validator; this does not introduce upstream's separate no-argument-validation change. There is no app
startup, private data, user-profile scan, device, clipboard, provider or network behavior in a fixture.

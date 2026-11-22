# Generated Age 0.11.5 passphrase fixture

This is original generated test data, never user content. `plaintext.bin` contains 78 bytes of text,
NUL and non-UTF-8 binary values. The public fixture passphrase is
`Oxbox public age 0.11.5 fixture password`. Do not use it for real files.

`legacy.age` is produced by the actual historical **Age 0.11.5** library using scrypt factor **10**.
The reduced work factor keeps the compatibility tests fast; it is not an application policy. Current
production encryption remains factor **18**, and decryption's maximum remains **20**. The existing
`round_trip_wrong_password_and_no_overwrite` test checks the real production header without adding
another expensive production KDF here.

Generation receipt (2026-11-22): the integrator ran the checked-in `generate.rs` through the isolated
historical harness, resolving exact Age 0.11.5, and its old-reader self-check passed. The generated
`legacy.age` SHA-256 is `122a8bf7081fffe5ddf353f3a1f3f8e60860b63ea15dd0f668a6eaea7adc7d5e`.
The producer SHA-256 is `19012d816453a9ffaea51bfec5a624fd4294b380e218a4b5cf6f43ebe1e7dafb`,
and the plaintext SHA-256 is `ecc35f2b763cea776125cd314f18e311e768b4f4af90f09a37411339bd6af339`.
Both current-Core `legacy_age_0115_` regression tests passed in the integrator's Rust 1.95 Windows
workspace run on 2026-11-22, with no failures or ignored tests. This is generated-format compatibility
evidence; final cross-platform CI and new production artifact acceptance remain separate.

The integrator used this command form with its absolute equivalent of the output path:

```sh
cargo +1.95.0 run --manifest-path target/security-accessibility/age-interop/Cargo.toml --locked --example export_legacy_fixture -- crates/oxbox-core/tests/fixtures/age-0.11.5/legacy.age
```

Contributors should use their own new generated path. The private harness itself is not required:
the checked-in producer and manifest
recipe below provide the independently reproducible source and historical dependency requirement.

The checked-in `generate.rs` producer accepts only a new output filename, encrypts the fixed included
plaintext and self-checks it with the old reader. It refuses an existing output. A contributor can
create a disposable Cargo manifest outside the application workspace (for example under ignored
`target/age0115-fixture-generator`) with this content, changing only the absolute `path` to this source:

```toml
[package]
name = "generate-age-0115-fixture"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
age_old = { package = "age", version = "=0.11.5", default-features = false }

[[bin]]
name = "generate-age-0115-fixture"
path = "/absolute/path/to/oxbox/crates/oxbox-core/tests/fixtures/age-0.11.5/generate.rs"

[profile.dev.package.scrypt]
opt-level = 3
[profile.dev.package.salsa20]
opt-level = 3
```

Then use Rust 1.95 to run that manifest and supply a **new generated fixture filename**, not a user file.
For example, `cargo +1.95.0 run --manifest-path target/age0115-fixture-generator/Cargo.toml -- target/age0115-regenerated.age`.
Keep its separate lockfile and confirm its tree resolves `age 0.11.5`. This deliberately historical
generator is not an application dependency; its old localization/macro dependencies must not be
mistaken for the selected application graph. Do not weaken or ignore the application's audit.

Age generates random salt, file keys and nonce, so regenerating the same plaintext intentionally gives
different encrypted bytes. Verify decryption and the factor-10 header; record the new hash explicitly
before intentionally replacing a checked-in fixture. The core tests copy the fixture into generated
temporary directories and verify decryption, wrong password, authenticated corruption, cancellation,
exclusive output creation and input preservation. They do not modify the checked-in fixture, collect
clipboard/screen/device data, access a provider or change OS settings.

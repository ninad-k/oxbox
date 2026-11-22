# Contributing to Oxbox

Help is welcome with bugs, tests, documentation, accessibility and platform acceptance. Keep each pull request
focused on one tool or one shared concern. All 49 tool routes are implemented; remaining acceptance boxes
identify verification work, not necessarily missing features.

For fork/clone commands, a real code path to follow and your first pull request, start with the
[contribution walkthrough](docs/GETTING_STARTED.md). The [documentation hub](docs/README.md) links the
feature catalogue, architecture, testing, troubleshooting and maintainer guides. Current native work lives
on `codex/native-renderer`; use it as the base for related contributions while migration review is open.

## Quick start

1. Install the prerequisites listed in the [README](README.md#build-from-source).
2. Run `cargo run -p oxbox-native --locked` for the selected native app. Use a disposable `--profile` directory for acceptance fixtures.
3. Review [validation gaps](docs/VALIDATION.md) or pick a specified roadmap task in [tasks/plan.md](tasks/plan.md).
   Comment on its issue (or open one) so work isn't duplicated. Unchecked phase gates can refer to acceptance,
   even when their implementation already exists.
4. Follow **"Recipe: add or change a tool"** in [AGENTS.md](AGENTS.md). It lists every file to touch.
5. Before opening the PR, run:

   ```bash
   cargo fmt --all -- --check
   cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
   cargo test -p oxbox-core -p oxbox-native --locked
   node scripts/build-native.mjs --check
   node scripts/build-community-docs.mjs --check
   pnpm check
   pnpm test
   pnpm build
   node scripts/build-website.mjs
   node --test scripts/website.test.mjs
   ```

6. In the PR description, say which OSes you tested on by hand and include a screenshot of the tool.

Read the [architecture guide](docs/ARCHITECTURE.md), [usage guide](docs/USAGE.md), and
[testing matrix](docs/TESTING.md). Start new branches with `codex/` when using Codex, and keep an individual PR
focused on one tool or one shared concern. Submit original code, tests, documentation, and license notices together.

### Native prerequisites

Use Rust **1.95+**. Node 22.17+ and the pinned pnpm are needed for legacy/site checks and packaging tooling,
not for the native runtime. Windows needs MSVC C++ tools; the selected eframe/egui host has no WebView2 requirement.
macOS needs Xcode command-line tools. Linux CI and packaging use Ubuntu 24.04; the capture backend needs
PipeWire/SPA headers newer than Ubuntu 22.04's stock packages. Ubuntu/Debian development packages include:

```bash
sudo apt-get install libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libpipewire-0.3-dev libspa-0.2-dev libwayland-dev libxcb1-dev libxcb-randr0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libfontconfig1-dev libfreetype6-dev libx11-dev libx11-xcb-dev libxi-dev libxcursor-dev libxrandr-dev libgbm-dev libegl1-mesa-dev libgl1-mesa-dev libclang-dev clang
```

Package names may vary by distribution. Follow the CI workflow for the tested runner setup. Capture and
clipboard operations need a desktop session; core fixture tests do not. Never run cleanup or power effects
against somebody else's working desktop as part of a routine test.

### Review expectations

Write failing logic tests first and record the red/green evidence. Keep user file operations streaming and use
exclusive creation for outputs. Test hostile archives and path boundaries with temporary fixtures. For OS effects,
inject a backend or test pure command construction; reserve the real effect for a consenting test machine.
Document remaining manual checks without ticking their acceptance boxes. Do not weaken a failing check or replace
a real feature with a planned-tool placeholder to make CI green.

Native changes follow the current recipe: core module/tests, real typed form in the native tools group,
canonical catalog/tools.json entry, minimal OS privileges and documentation. Use the bounded Task runner for
streaming/CPU work; background services must work hidden and stop/join on Quit. Preferences use atomic
transactions and preserve original legacy files. Do not substitute generic JSON forms or mock results.

The Svelte/Tauri source remains a parity reference. Its registry files now hold only view/icon adapters and
read metadata from the same canonical JSON. Keep those adapters and website output consistent when changing
the catalogue. pnpm tauri dev is a legacy comparison command, not the selected native entrypoint.

See [ADR 0004](docs/adr/0004-runtime-offline-boundary.md), [catalogue maintenance](catalog/README.md) and
[the 49-tool migration matrix](tasks/native-migration.md). Existing manual/installer/privacy/hardware evidence
does not transfer automatically between hosts. Native bundles use node scripts/build-native.mjs --bundle;
the installed bundler is packaging tooling, not an embedded runtime.

## Ground rules

- **Original work only.** Do not copy code, icons, names or text from proprietary software.
- **Safety first.** Tools never delete permanently by default and always confirm destructive actions.
- **Offline.** No network calls or telemetry without prior discussion in an issue.
- **New dependencies** need a short justification in the PR (what it does, license, maintenance, size).
  Obtain approval before adding a dependency outside the approved task/ADR set; a justification alone is not approval.
- **Use AI assistants freely**, but you are responsible for every line you submit. Make sure it builds, is tested, and you understand it.
- Unless explicitly stated otherwise and agreed with maintainers, submitted contributions use the project's
  existing dual MIT OR Apache-2.0 licensing. Include attribution for any permitted third-party material.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org): `feat(find-duplicates): group by size before hashing`.

## Code of conduct

Follow the [community code of conduct](CODE_OF_CONDUCT.md). Reviewers should explain requested changes and
contributors should be able to ask for help without harassment. Report vulnerabilities privately using
[SECURITY.md](SECURITY.md), not a public issue containing sensitive details.

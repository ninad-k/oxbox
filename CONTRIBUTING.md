# Contributing to Oxbox

Thanks for helping. Oxbox grows one tool at a time, and each tool is a small, self-contained pull request.

## Quick start

1. Install the prerequisites listed in the [README](README.md#build-from-source).
2. `pnpm install`, then `pnpm tauri dev` to run the app.
3. Review [validation gaps](docs/VALIDATION.md) or pick a specified roadmap task in [tasks/plan.md](tasks/plan.md).
   Comment on its issue (or open one) so work isn't duplicated. Unchecked phase gates can refer to acceptance,
   even when their implementation already exists.
4. Follow **"Recipe: add a tool"** in [AGENTS.md](AGENTS.md). It lists every file to touch.
5. Before opening the PR, run:

   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   pnpm check
   pnpm test
   pnpm build
   ```

6. In the PR description, say which OSes you tested on by hand and include a screenshot of the tool.

Read the [architecture guide](docs/ARCHITECTURE.md), [usage guide](docs/USAGE.md), and
[testing matrix](docs/TESTING.md). Start new branches with `codex/` when using Codex, and keep an individual PR
focused on one tool or one shared concern. Submit original code, tests, documentation, and license notices together.

### Native prerequisites

Use Rust 1.93+ and Node 22.17+; `package.json` pins pnpm. Windows needs MSVC C++ tools and WebView2.
macOS needs Xcode command-line tools. Linux CI and packaging use Ubuntu 24.04; the capture backend needs
PipeWire/SPA headers newer than Ubuntu 22.04's stock packages. Ubuntu/Debian development packages include:

```bash
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev patchelf libpipewire-0.3-dev libspa-0.2-dev libwayland-dev libxcb1-dev libxcb-randr0-dev libgbm-dev libegl1-mesa-dev libclang-dev clang
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

New command groups still follow the existing recipe: core module, thin adapter, registered handler, typed wrapper,
view, catalogue entry, minimal permissions, tests and docs. Views may import the appropriate typed group wrapper;
`api.ts` remains the public facade. No view may import `invoke`.
Local imports use the `#lib/*` mapping in `package.json` with explicit extensions, for example
`#lib/api.ts` and `#lib/ui/Notice.svelte`. SvelteKit 3 configuration is in `vite.config.js`.

## Ground rules

- **Original work only.** Do not copy code, icons, names or text from proprietary software.
- **Safety first.** Tools never delete permanently by default and always confirm destructive actions.
- **Offline.** No network calls or telemetry without prior discussion in an issue.
- **New dependencies** need a short justification in the PR (what it does, license, maintenance, size).
- **Use AI assistants freely**, but you are responsible for every line you submit. Make sure it builds, is tested, and you understand it.
- Unless explicitly stated otherwise and agreed with maintainers, submitted contributions use the project's
  existing dual MIT OR Apache-2.0 licensing. Include attribution for any permitted third-party material.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org): `feat(find-duplicates): group by size before hashing`.

## Code of conduct

Be kind and constructive. Harassment or discrimination of any kind is not tolerated; maintainers may remove
comments, commits or contributors that break this.

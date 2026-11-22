# Your first Oxbox contribution

You can help with a reproducible bug, a documentation correction, a generated fixture, accessibility feedback
or an unverified platform case. All 56 tool routes already have implementations; an unchecked acceptance
box does not mean its tool needs to be written again.

## Get the current source

Install Git, Rust 1.95 and the native prerequisites in [CONTRIBUTING](../CONTRIBUTING.md#native-prerequisites).
Fork `ninad-k/oxbox` on GitHub, then clone your fork. Replace `YOUR-ACCOUNT` below. The selected source is
currently `codex/native-renderer`; fetch it from upstream even if your fork initially copied only the default
branch. Use that as the pull request base while migration review is open.

```bash
git clone https://github.com/YOUR-ACCOUNT/oxbox.git
cd oxbox
git remote add upstream https://github.com/ninad-k/oxbox.git
git fetch upstream codex/native-renderer
git switch --no-track -c codex/describe-your-change upstream/codex/native-renderer
rustup toolchain install 1.95.0 --component rustfmt --component clippy
cargo +1.95.0 test -p oxbox-core --locked tools::text_transform
cargo +1.95.0 run -p oxbox-native --locked -- --profile ./target/first-contribution-profile
```

Use disposable input/output files in `target/`, not personal documents. A separate profile isolates saved
preferences and prevents automatic startup registration; it does not sandbox OS actions, capture, clipboard
or devices. Do not start those effects unless you intend to test them on your own consenting desktop.
Quit with Ctrl/Cmd+Q or the tray menu; closing the window normally hides it.

Node 22.17+ and pinned pnpm are required only when running legacy UI, site or packaging checks:

```bash
corepack pnpm install --frozen-lockfile
```

If Corepack is unavailable, install the exact pnpm version in `package.json` using your normal Node package
manager. Do not upgrade the lockfile to resolve a local setup problem.

## Trace one small behavior

For a pure tool, start with `crates/oxbox-core/src/tools/text_transform.rs`. The public
`transform(input, TextCase)` function owns the result; its tests cover cases, Unicode and empty input.
The actual native controls are `TextTools::transform` in
`crates/oxbox-native/src/tools/utilities/text.rs`. The `transform-text` entry in `catalog/tools.json`
supplies its name, description, category and platforms to every interface.

1. Reproduce a specific discrepancy using an existing test or a small disposable fixture. State the expected
   result and the actual result in an issue. Pick one behavior, not a rewrite of the tool.
2. Add a test to the core module and run its filter. Observe the failure before changing the implementation.
   Test OS effects through a fake backend; never use a routine unit test to move the real mouse or sleep a PC.
3. Fix the smallest responsible function. If controls change, update the concrete native form, visible labels,
   error/status text and cancellation path. Keep file work off the UI thread using the existing `Task` pattern.
4. Run the test again, then [the required checks](../CONTRIBUTING.md#quick-start). For native UI changes,
   use the isolated profile and check keyboard focus, empty/busy/success/error states, Back, hide and Quit.
5. Update the relevant guide/task row. Record only what you actually exercised. A fake backend or headless
   AccessKit test cannot certify a real screen reader, GPU, device or clean-machine installation.

For a new tool, discuss its specification, platform limits, network behavior and dependencies first, then
follow the full recipe in [AGENTS.md](../AGENTS.md#5-recipe-add-or-change-a-tool). Use the canonical metadata;
do not create a second list of tool names in a host or documentation file.

## Send the change for review

```bash
git diff --check
git status --short
git add path/to/changed-file path/to/related-test
git commit -m "fix(transform-text): describe the corrected behavior"
git push -u origin codex/describe-your-change
```

Open a pull request against `ninad-k/oxbox` → `codex/native-renderer`. Explain the visible before/after
behavior, test results, OS and commit tested, and any remaining limitation. Include a screenshot for UI
changes using generated, non-private data. Never commit `target/`, build output, installers, secrets or a
nested checkout. A documentation-only contribution does not require pretending to have run unrelated
hardware checks: identify its scope and validate the links and commands you changed.

Your contribution uses the project's MIT OR Apache-2.0 licensing unless agreed otherwise. Respect the
[code of conduct](../CODE_OF_CONDUCT.md) and report vulnerabilities through [the private channel](../SECURITY.md).

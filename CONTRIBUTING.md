# Contributing to Oxbox

Thanks for helping. Oxbox grows one tool at a time, and each tool is a small, self-contained pull request.

## Quick start

1. Install the prerequisites listed in the [README](README.md#build-from-source).
2. `pnpm install`, then `pnpm tauri dev` to run the app.
3. Pick an unchecked tool in [tasks/todo.md](tasks/todo.md). Comment on its issue (or open one) so work isn't duplicated.
4. Follow **"Recipe: add a tool"** in [AGENTS.md](AGENTS.md). It lists every file to touch.
5. Before opening the PR, run:

   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   pnpm check
   pnpm test
   ```

6. In the PR description, say which OSes you tested on by hand and include a screenshot of the tool.

## Ground rules

- **Original work only.** Do not copy code, icons, names or text from proprietary software.
- **Safety first.** Tools never delete permanently by default and always confirm destructive actions.
- **Offline.** No network calls or telemetry without prior discussion in an issue.
- **New dependencies** need a short justification in the PR (what it does, license, maintenance, size).
- **Use AI assistants freely**, but you are responsible for every line you submit. Make sure it builds, is tested, and you understand it.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org): `feat(find-duplicates): group by size before hashing`.

## Code of conduct

Be kind and constructive. Harassment or discrimination of any kind is not tolerated; maintainers may remove
comments, commits or contributors that break this.

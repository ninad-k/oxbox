# Testing and acceptance

Use Rust **1.95+** and a supported Node.js release (22.17+ recommended). Install UI dependencies from the lockfile.
All commands run from the repository root. The nested untracked `oxbox/` copy, if present in a local checkout,
is not part of this workspace; Vitest deliberately includes only `src/**/*.test.ts`.

```bash
pnpm install --frozen-lockfile
cargo fmt --all -- --check
cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
cargo test -p oxbox-core -p oxbox-native --locked
node scripts/build-native.mjs --check
pnpm check
pnpm test
pnpm build
cargo run -p oxbox-native --locked
```

The native package selects eframe/egui 0.37 with Glow, without a webview. The legacy Svelte checks remain
comparison/catalogue checks; they do not certify native interaction. Packaging uses the build-native wrapper
and must be checked separately. See [the 49-row native matrix](../tasks/native-migration.md).

## Automated coverage

- Native tasks: cancel/abandon/late-result handling, latest-only bounded progress, shutdown and joined workers.
- Native instance/settings transactions and finite bundled icon vectors.
- Headless route rendering/name checks for 13 utilities and 14 media forms, including legacy import preservation/default-off currency.
  These tests consume textures_delta without a GPU. They do not click device/destructive controls or prove accessibility.
- Headless default-form/name checks for all 22 system forms with task admission closed before rendering;
  no OS-reading/changing job runs. Expanded archive/password/provider and two-row fixtures are checked separately.
- Generated external-provider children exercise timeout/natural-exit pipe inheritance and process-tree cleanup.
  They do not open devices or network sockets. Password fixtures use generated text; scrypt resource-limit
  tests reject excessive factors before running their KDF. Actual screen-reader/device/OS acceptance is separate.
- Text cases, Unicode boundaries, hash vectors, streaming checksums, progress/cancellation.
- All conversion categories, temperature offsets/absolute zero, SI/binary bytes, round trips, numeric errors.
- IANA zones and DST transitions; QR raster decoding and invalid barcode checksums.
- Job-token sharing, duplicate grouping/content discrimination/overlapping roots.
- Archive round trips, actual traversal/link fixtures, output limits, overwrite refusal, cancellation.
- ZIP64 directory framing and local/central consistency; bounded TAR GNU/PAX metadata; actual plain and
  compressed 7z headers, 128 KiB stored/decoded metadata rejection, malformed file counts, cumulative
  decoder-model budgets, virtual-reader bounds and frozen-metadata handoff after source changes.
- Protected filesystem roots, cache allow-list fixtures, platform command argument tables.
- System snapshot invariants, network counter resets, fake-clock sleep/break scheduling.
- Same-thread awake-guard ownership and exact desktop-state restoration using injected backends.
- HiDPI crop geometry, clipboard byte/count/pin budgets, resize dimensions/output preservation, age round trip
  and wrong-passphrase/partial-output behavior.
- UI search, catalogue importability, checksum comparison, preference sanitization, monotonic clock behavior.
- Favorites-only launcher default and Alt+1–9 favorite-slot mapping.
- Mouse Activity fake-clock/input scheduling, observed-token versus Start baseline, monitor-edge/gap geometry,
  errors, expiry, Stop/backend-drop acknowledgement and refusal of queued Start after Quit. No test moves the pointer.
- Promoted window geometry/ownership, rollback timing, volume/app-command validation, reminder DST/restart behavior,
  FFmpeg argument/output safety, QR/EAN-13/Code128 decoding, and bounded dated currency XML conversion.

Tests that need files use disposable temporary directories. Automated tests do not lock the workstation,
schedule real shutdowns, change desktop icons, capture private displays, or clean the user's caches.

## Manual Windows matrix

Run on a real Windows desktop using a disposable fixture folder. Record the commit, OS version, display scaling,
and result in [VALIDATION.md](VALIDATION.md). A source implementation and passing unit test do not replace these checks.

| Area | Check |
|---|---|
| Launcher | Native decorated 800 × 800 starting window, favorites-only Dashboard, All tools/categories, Ctrl+Alt+O/Cmd+Option+O, Ctrl/Cmd+K, Alt+1–9, Ctrl/Cmd+Q, tray/close-to-tray, single instance and shortcut conflicts |
| Settings | All three themes, restart persistence/migration, actual login registration, save failure surfaced |
| Text/time/codes | Conversion swap and invalid input, timer pause/resume/laps/hidden completion, DST clocks/reorder persistence, QR PNG/SVG/copy |
| Checksum | Drop/pick, published-hash match/mismatch, large generated file, progress, cancellation |
| Duplicates | Identical fixture files with distinct names; select copies; review confirmation and Trash result |
| Archives | ZIP/tar.gz fixture round trip, 7z fixture extraction, invalid archives, existing destinations, cancellation |
| Cleanup | Scan only first; inspect allow-listed paths and counts; cleanup only in a disposable test account/VM |
| Monitor/network | Values update only while open, CPU trace, physical/virtual interface filter, session totals |
| Mouse Activity | Consenting Windows test session only: explicit Start/full idle delay, real input pause, held input, optional duration, monitor edges/gaps, banner/tray Stop, Quit and restart stopped; macOS/Linux unavailable |
| Awake | Indefinite/timed/end-time modes, display option, tray toggle, release after stop/expiry/Quit |
| Lock/power | Use a test machine; lock, short sleep timer warning, cancellation from another view/tray, actual actions only after saving work |
| Break Time | Reminder while launcher hidden, snooze/skip, eye-rest, restart settings, idle/fullscreen best effort |
| Capture | Multiple monitors/windows, 0/3/5/10 s delay, area/Escape, 100% and 150% scaling, folder and clipboard |
| Clipboard | Explicit start/pause, text/image/dedupe/search/pin/copy/delete/clear, concealed formats, empty after Quit |
| Resize | Batch percentage/fit/exact, JPEG quality, EXIF rotation, PNG/JPEG/WebP, collisions, cancellation |
| Desktop | Originally visible and hidden states; toggle/tray; preceding state restored after Quit |
| Encryption | Large disposable file, correct/wrong passphrase, encrypt/decrypt progress/cancel, no overwritten original/output |
| Accessibility | Keyboard-only controls, visible focus, labels, errors announced, light/dark/reduced-motion |

## Cross-platform and release checks

The [installer protocol](INSTALLER_ACCEPTANCE.md), [privacy boundaries](PRIVACY_ACCEPTANCE.md),
[accessibility matrix](ACCESSIBILITY.md) and [security review](SECURITY_REVIEW.md) provide reproducible,
scoped follow-up commands. Run `node --test scripts/build-native.test.mjs scripts/license-inventory.test.mjs
scripts/verify-native-installers.test.mjs` for pure packaging checks, and on Windows
`node --test scripts/observe-native-endpoints.test.mjs` for observer guards/validity without sampling.

CI runs Rust and UI checks on fresh runners. Native builds need desktop development packages even when tests
do not launch windows. Review CI results on the public PR; local Windows tests do not prove macOS/Linux builds.
Run the same supported-tool matrix on macOS and Linux, record session/desktop details, and confirm unavailable
tools are labeled correctly. Check runtime network activity in an isolated test environment, not by changing
the developer's firewall configuration. Test installer behavior and final size separately from source builds.

Expanded tool/provider/manual checks are in [ROADMAP_TOOLS.md](ROADMAP_TOOLS.md) and the Phase 5 task files.
Explicit developer provider smoke commands are documented there; ordinary tests never make network calls or open devices.
Do not tick a phase checkpoint or publish a release until its recorded acceptance checks are complete.

## Website checks

```bash
node scripts/build-website.mjs
node --test scripts/website.test.mjs
```

The build validates relative assets, local fragments and the canonical JSON catalogue. Registry tests compare
metadata/icon derivation against a changed JSON fixture; website data tests compare every canonical tool and platform. Reset regressions model
the cancelable reset event before native default value restoration. Browser QA must still cover desktop and
375px mobile layout, keyboard/focus, search/category/platform combinations, empty/reset behavior and FAQ controls.
Record the actual Pages deployment separately; a local build is not evidence that the public site is live.

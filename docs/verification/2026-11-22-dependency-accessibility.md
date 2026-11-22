# Dependency and accessibility follow-up — 2026-11-22

This receipt follows the [56-tool pass](2026-11-22-56-tool-pass.md). The user approved phases 1 and 2 of
[ADR 0005](../adr/0005-dependency-security-remediation.md), including three pinned upstream source
backports, Age 0.12.1 and Jiff 1.0.2. Exact eframe 0.37.0, Rust 1.95, all 56 stable tool IDs, saved data,
offline defaults and the unpublished draft are retained. Clean-machine acceptance remains intentionally pending.

The Age 0.12.1 candidate subsequently failed its security gate: it adds the vulnerable `crypto-bigint
0.5.5` family through mandatory HPKE/P-256 paths. The application retains exact Age 0.11.5. The user
subsequently approved the [separate localization-only remedy](../adr/0006-age-upgrade-security-gate.md);
its fourth source-package implementation and validation are in progress. The successful Age 0.12.1
compile/interoperability results below are historical candidate evidence, not acceptance of that upgrade.

This is an **intermediate receipt** for source
`35d6afc3b9a38057c1164198b2f634083e50e5b9`. Later popup-focus fixes and the newly approved fourth
source patch are outside this commit's CI, executable and runtime evidence. The GitHub release draft still
contains the prior A9 candidate's six assets; no version tag has been created.

## Compatibility evidence

- Before changing production time code, the integrator captured the original public API corpus:
  `crates/oxbox-core/tests/fixtures/chrono-time-compat.json`, 675,194 bytes, SHA-256
  `c83f87e6b476346672f2cf0dfc9d8185c599b8f954e6aabbed82cbecb8797b85`.
  It contains 597 ordered zone names, 1,863 clock rows, 40 timestamp-range rows, 44 local-time cases,
  48 recurrence cases, eight restore cases, saved records and 17 rate dates. All five baseline comparison
  tests passed against the original Chrono implementation. This frozen fixture must not be regenerated
  from the new implementation merely to make a comparison pass.
- A separate original-parser probe reproduced signed extended years, numeric padding and Unicode
  whitespace acceptance. A new currency regression then failed for `+10000-02-29` with direct Jiff
  parsing. The checked compatibility adapter preserves the original accepted calendar range and raw
  stored date strings, using Jiff to validate the repeating Gregorian calendar. That regression passes.
- Two future-time regressions failed against the original implementation: after Chrono-TZ's finite
  transition table, summer New York clocks and a year-2200 alarm used EST indefinitely. The maintained
  time implementation must apply the zone's recurring future rules. This is an explicit behavior correction;
  it must be distinguished from exact preservation of the frozen historical/current corpus.
- The integrator ran both directions of a generated Age interoperability harness: actual Age 0.11.5
  factor-10 output decrypted in the current core, and real current-core factor-18 output decrypted in
  Age 0.11.5 with ceiling 20. Both recovered the same generated 196,731 bytes, SHA-256
  `669f2290ddeaf7c800b997b38c82798cb253edaf4027a3757a7eb86fe0ab8bd5`.
  Wrong passwords, authenticated corruption, truncation, real-progress cancellation, pre-cancel,
  excessive-factor rejection, exclusive outputs and unchanged originals passed. These compatibility
  checks were expected to pass; no failing application behavior is claimed for them.
- A checked-in low-factor historical fixture, producer and reproduction guide keep legacy decrypt
  compatibility independently runnable without adding historical Age as an application dependency.
  Generation with actual Age 0.11.5 and its old-reader self-check passed; ciphertext SHA-256
  `122a8bf7081fffe5ddf353f3a1f3f8e60860b63ea15dd0f668a6eaea7adc7d5e`.
- The isolated [configuration harness](../../tests/dependency-compat/README.md) passed five generated
  tests for valid, invalid and missing Cargo/localization configuration, Unicode values, defaults and
  parent fallback. Production Age localization macros also compiled. This preserves the distinction
  between a missing optional configuration and malformed configuration.

## Accessibility fixes and scope

Reproduced behavioral regressions cover popup Escape dismissal; actual expanded/collapsed metadata;
screen-reader Expand/Collapse action dispatch for dropdowns and disclosure headers; disabled text and
numeric fields refusing accessibility value edits; keyboard-editable numeric screenshot-area bounds;
distinct decoded-barcode Copy names; and stable contextual image-row identity after another row is removed.
Tests failed before their corresponding fixes. No test captures a screen, moves the pointer, reads the
clipboard, deletes user files or changes an OS setting.

The integrated native accessibility suite passed **146 tests** before the time migration. All 56 default
routes have named interactive controls and actual combo expansion state. These headless AccessKit checks
prove those properties; they do not prove spoken announcements, visible focus or complete keyboard traversal.
The user assisted with the intermediate binary's Narrator check, described below. Actual final-binary
speech content and complete keyboard traversal remain unverified.

The local held-Age Windows workspace run for the intermediate source passed **294 core tests, five frozen comparisons,
148 native tests and 12 retained-host tests**, zero failed/ignored. It includes three additional checked
time-range tests and a reproduced/fixed legacy alarm grammar regression for Unicode numeric whitespace
and signed padded years. Strict all-target workspace Clippy and formatting passed on Rust 1.95.
The frozen compatibility fixture still has its original hash.

## Intermediate cross-platform CI

[CI run 45035930243](https://github.com/ninad-k/oxbox/actions/runs/45035930243) completed successfully
for exact source `35d6afc3b9a38057c1164198b2f634083e50e5b9`; all five jobs passed.

| Native runner | Core tests | Frozen comparisons | Native tests | Isolated localization tests | Production build |
|---|---:|---:|---:|---:|---|
| Windows x64 | 294 | 5 | 148 | 5 | Passed, 3m 18s |
| macOS ARM64 | 289 | 5 | 150 | 5 | Passed, 3m 01s |
| Linux x64 | 289 | 5 | 151 | 5 | Passed, 3m 18s |

Each full suite had zero failed, ignored or filtered tests. Strict all-target Core/native Clippy passed
on all three runners; formatting passed on Linux, where that CI step runs. The separate minimum-Rust
job used actual Rust **1.95.0** and passed the locked Core/native all-target Windows check in 38.45s.
The UI job passed 39 tests across 11 files, zero Svelte errors/warnings, its production build and
generated contributor-index check, five source-integrity tests and 22 packaging/license/installer tests.
Fresh-checkout verification accepted the exact approved upstream source bytes.

Linux also ran `approved_glib_backport_string_iterator_methods` successfully in both its normal native
suite and an optimized release test. The latter reported **one passed, zero failed/ignored, 150 filtered**;
those filters select the one regression rather than omit tests from the full suite above. Its release-test
compilation took 4m 45s. This provides actual Linux execution evidence for the reviewed GLib backport.
The five localization configuration tests, all five immutable time comparisons, the legacy alarm grammar
regression, three checked time-boundary tests and two corrected future-DST tests passed on each OS.

## Intermediate Windows artifact and observations

The local production executable for the intermediate source is **13,637,120 bytes**, SHA-256
`cac752c108b9c66cbb7b23159117eeb510303718e112251aeb3e2f6092cbb365`.
Static installer verification passed, including matching that exact executable in the extracted MSI
payload. Candidate installer SHA-256 values are:

- MSI: `682312b113e6f64ff4507f9b7d973da24609bccc992ad5e6c01f10e71ec27fca`.
- NSIS: `f9b0ae541a7ce56c4bd52a6c50cb8a7b08b09c04440c37c9560fc44d3f9b9189`.

These are local intermediate artifacts, not a draft-asset refresh. No installer or MSI custom action was
executed; independent NSIS payload extraction and clean-machine install/login/upgrade/uninstall remain
pending. Static compiler-input and MSI extraction evidence does not establish those behaviors.

The consenting desktop check used the exact executable above. Ctrl+K, typing Unit Converter and Enter
opened that route. Tab/Shift+Tab traversal reached Dashboard, Copy and the To-unit chooser; Enter opened
the chooser and Escape closed it without leaving the route. **Escape then lost keyboard focus**, an
actual defect. A new headless regression reproduced that focus loss; a narrow event-filter guard fixed
it. The subsequent local suite passed 150 native tests and strict workspace Clippy, but that later fix
is not in this intermediate executable or its CI receipt.

Clicking the input, Ctrl+A and entering invalid input retained focus, exposed a named alert and removed
Copy. The user confirmed Narrator was audible, but did not report its announcement words; a tool also
reported Narrator at higher integrity. No speech recap or exact announcement sequence was established.
These observations do not certify screen-reader output, complete focus order or another assistive tool.

A user report of closing the app did not by itself prove exit: a read-only process check still found an
app process. Ctrl+Shift+Q left the app running, and Ctrl+Q did not exit during the Narrator session;
the shortcut outcome is inconclusive rather than a pass. Clicking **Quit Oxbox** later exited all QA app
processes. That separate desktop observation is not post-exit endpoint coverage.

The hash-pinned endpoint observer completed a valid **300.081-second, 5,999-sample** startup/converter
window, with zero PID-owned TCP rows, zero UDP bound endpoints and zero sample errors. The executable
hash stayed unchanged. **One app process remained at the final successful sample**, so this window
does not cover Quit or post-exit cleanup. Sampling can miss short-lived sockets and excludes DNS Client,
brokers and unrelated system services; no packet/DNS/ETW trace was performed. This is bounded evidence,
not proof of zero network activity, and it does not certify the later changed binary.

## Source identity and supporting checks

The local integrity verifier passed for all **572 upstream package files** and five provenance/patch
artifacts. Notice generation now binds patched package identity to the verified directory, tree hash,
base archive, immutable upstream revision and exact patch; it refuses mismatched or missing provenance.
The native build wrapper verifies source integrity before distribution work.

The combined source-integrity, notices and installer/build regressions passed **27/27**; legacy UI
tests passed **39/39** across 11 files; Svelte check reported zero errors/warnings; legacy production
build passed; website generation preserved 56 tools/eight categories and its four regressions passed.

## Approved final local candidate

The user approved ADR 0006's narrow fourth backport. Root activated the exact i18n-embed-fl 0.9.4
override while retaining Age 0.11.5. Both the original and patched macro packages pass the same valid
runtime case (plain, named and attribute messages) and four intentional compiler-diagnostic cases,
without a build panic. Independent review reproduced two evidence-classifier gaps; six final Node
guards now reject attribute/derive panics mixed with expected diagnostics and a different checkout's
similarly named vendor directory. The real patched comparison passed again after those guard fixes.

The approved Windows workspace passed **294 core tests, five frozen comparisons, 150 native tests
and 12 retained-host tests**, zero failed/ignored, with Rust 1.95 formatting and strict all-target
workspace Clippy. The 150 native tests include the actual Enter/Escape/next-Tab focus regression and
negative cases that protect other fields and outside pointer clicks.

All **582 upstream files and six provenance/patch artifacts** pass integrity verification; independently
reading the staged Git blobs confirmed all 582 source files retain their exact recorded bytes. The
combined Node source/build/notices/installer/macro guards pass **34/34**. Windows notices reproduce
358 packages, zero missing texts and three unresolved human attribution reviews. The native graph
still has 345 package names without a browser host. Normal/build graphs for both native and retained
hosts on Windows/Linux contain no active Chrono, Chrono-TZ, basic-toml, macro-error2 or crypto-bigint.

The approved-resolution audit reports **zero vulnerability entries and five maintenance warnings**
at the same advisory database, without ignores. The remaining packages are optional locked Chrono,
GTK macro-error1 and three Zbus-family packages. The local GLib source fix remains separately visible
because the scanner omits path packages. These results do not establish a warning-free audit or
independent security acceptance. Final source/CI, rebuilt installer and desktop/runtime receipts follow
separately; the intermediate `35d6afc` executable does not certify this changed candidate.

## Final acceptance still required

The intermediate audit for the held Age 0.11.5 resolution reports **zero vulnerability entries and six
maintenance warnings**, using advisory database `d1fe6ac3eba823a3515e849045f36dc34543885d`, without
ignores. Chrono remains locked for RXing's preserved optional client/WASM support; Chrono-TZ,
basic-toml and the introduced crypto-bigint chain are removed. GLib is a path package omitted by the
scanner, so the reviewed source advisory remains visible separately. The native Windows graph check
passed with **345 package names** and no Tauri/WebView host. Regenerated Windows notices verify
**358 packages, zero missing texts and three human attribution reviews**.

The intermediate cross-platform CI, optimized Linux GLib execution, exact executable, static installer
checks and bounded runtime observations are recorded above. The later popup-focus fix and accepted
ADR 0006 implementation still require their own final integrated checks, source identity, production
artifact and runtime receipts. Prior hashes and measurements do not certify changed source or
dependencies; no passing CI or runtime outcome for the fourth patch is claimed here.

GTK macro and AccessKit/Zbus maintenance migrations remain outside the approved implementation scope.
Never suppress advisories or disable AccessKit to present a clean audit. Patched GLib retains its original
version and needs source-level accounting even if a scanner omits local path packages.

Clean-machine install/login/upgrade/uninstall, full per-tool native effects, another screen reader,
IME/DPI/high contrast and other-OS runtime accessibility, DNS/broker/short-flow coverage, independent
security/attribution reviews and signing remain separate gates. Keep the release a draft/prerelease;
do not create a version tag or publish it.

## Final approved source — 8a62ec1

This section records the completed follow-up for source
`8a62ec1ad7e65ad9b3378dbcb495b897ef725c47`, including the two focus regressions and the approved fourth
localization source backport. The earlier intermediate/pending statements are historical snapshots;
their executable and observation windows remain separate from this final-source evidence. Age remains
exact 0.11.5, and the immutable time corpus keeps its original hash.

### Final CI

[CI run 45039126970](https://github.com/ninad-k/oxbox/actions/runs/45039126970) completed successfully
for the exact source above: **all five jobs passed**.

| Native runner | Core tests | Frozen comparisons | Native tests | Isolated configuration tests | Production build |
|---|---:|---:|---:|---:|---|
| Windows x64 | 294 | 5 | 150 | 5 | Passed, 3m 19s |
| macOS ARM64 | 289 | 5 | 152 | 5 | Passed, 2m 35s |
| Linux x64 | 289 | 5 | 153 | 5 | Passed, 2m 56s |

All full suites had zero failed, ignored or filtered tests. Strict all-target Core/native Clippy passed
on each OS; the Linux formatting step passed. The minimum-Rust job used actual Rust 1.95.0 and passed
its locked Windows all-target check in 31.39s. The UI job passed 39 tests across 11 files, zero Svelte
errors/warnings, its production build and the generated contributor-index check.

Each native job explicitly fetched its locked macro-fixture dependencies, then passed the source-bound
offline checker against the reviewed `i18n-embed-fl 0.9.4` directory. Its valid generated fixture ran;
unknown message, missing named argument, unexpected named argument and malformed input each produced
the required compile diagnostic without a build panic. All five cases passed on all three OSes, in
addition to their five configuration tests. Both actual popup-focus regressions, all five frozen time
comparisons and the six focused time grammar/range/future-rule regressions passed on each OS.

Fresh-checkout integrity verification passed for all **four packages, 582 upstream files and six
provenance/patch artifacts**. The six integrity tests and 28 build/macro-checker/license/installer tests
passed, **34 Node tests** combined. Linux's GLib iterator regression also passed in both normal and
optimized native execution; the optimized selection reported one passed, zero failed/ignored and 152
other tests filtered, with 2m 41s release-test compilation. The full Linux suite above was unfiltered.

### Final local artifact and installer identity

The Rust 1.95 Windows production executable is **13,637,632 bytes**, SHA-256
`101ed74680fc884960ebfaaee33a8f1677b7ed09e0dd83c5cac90083b142b6ac`.
The primary and peer static-verification runs passed and matched this executable in the extracted MSI
payload. The prepared six-asset set is:

| Asset | Bytes | SHA-256 |
|---|---:|---|
| `Oxbox-native_0.1.0_x64-setup.exe` | 4,324,931 | `2ce96ef65f431cdd39d11712696f97a5868340c8a0ad69c150a011b5abba2af4` |
| `Oxbox-native_0.1.0_x64_en-US.msi` | 6,062,080 | `96b78fe77c29d0ee130748babf0e568eb21efc9cc2e1ffda4b2e9634a1cc7f4e` |
| `LICENSE-MIT` | 1,076 | `dff2f23bad58d45b0830bf91f27ef278996da5ca00f1ab198969c854e8060f4f` |
| `LICENSE-APACHE` | 11,358 | `c95bae1d1ce0235ecccd3560b772ec1efb97f348a79f0fbe0a634f0c2ccefe2c` |
| `THIRD_PARTY_NOTICES.txt` | 3,718,522 | `779210e025e9b9f9a227c44bed73ac26c8216d419440dfb06a0c221baa217784` |
| `SHA256SUMS.txt` | 447 | `eec439a4a958c4c5301e93d88e472dd447b0dc9d96ac110a4df7bac57f03ea76` |

No installer or MSI custom action was executed. Independent NSIS extraction, clean-machine acceptance,
signing and human license review remain pending.

The [existing release draft](https://github.com/ninad-k/oxbox/releases/tag/untagged-6ee10a588a88a930f10a),
ID **453727915**, now targets exact source `8a62ec1ad7e65ad9b3378dbcb495b897ef725c47`.
All six assets were downloaded and matched the names, sizes and SHA-256 values above. The prior A9
six-asset set and notes were backed up and verified before replacement. Both local and downloaded
installer sets passed static verification with zero errors. The refresh ran no installer, application
or MSI custom action. The release is still **draft=true, prerelease=true, published=false**; its release
tag field is `v0.1.0`, but no actual `v0.1.0` Git tag exists.

The resumed final Rust audit again reported **zero vulnerability entries and five maintenance warnings**
against advisory database `d1fe6ac3eba823a3515e849045f36dc34543885d`, without ignores. The fresh pnpm
audit reported zero advisories. The separately recorded path-package GLib finding, maintenance follow-up
and human security/attribution gates remain visible; these counts are not independent security acceptance.

### Resumed consenting-desktop checks

All observations below used the exact final executable hash above. After resuming the interrupted
session, Ctrl+Q exited its previously running app process. A fresh launch then passed Ctrl+K, typing
Unit Converter and Enter. Tab reached Dashboard, Shift+Tab reached Copy, and another Shift+Tab reached
the To-unit chooser. Enter opened its nine options; Escape closed the popup **and retained To focus**;
the next Tab reached Copy. This directly rechecked the focus defect fixed after the intermediate build.

The native checksum picker accepted a generated Japanese filename, `generated-画像.txt`, without changing
its name. The displayed SHA-256
`9fad1eb93c668a3e3470680d6e6cb8ee8d31b1c4948d91cf217a9bb41e774574` matched both an independent
PowerShell calculation and the Expected field; status was Completed. Copy was not activated, so this
does not claim clipboard-effect acceptance. Closing the window hid it while the process remained alive;
a second launch restored the same original process/window and its checksum result.

Ctrl+Shift+Q left the process running but the window vanished. Source review found no backend binding
for that chord; the cause of the visibility change remains **inconclusive**, without a speculative fix
or a passing negative-visibility claim. Another activation restored the same window, and Ctrl+Q then
exited the app process. Narrator was not relaunched: the earlier confirmation that it was audible is
retained, while specific announcement words and a final screen-reader sequence remain pending.

### Separate final-source endpoint windows

Both hash-pinned observers completed normally, saw the target and retained the exact executable hash.

| Window | Duration | Samples | TCP rows / UDP bound endpoints (max) | Sample errors | Processes at final sample |
|---|---:|---:|---|---:|---:|
| Resumed converter/checksum/hide/restore | 300.103s | 5,998 | 0 / 0 | 0 | 1 |
| Fresh launch and Ctrl+Q | 90.033s | 1,800 | 0 / 0 | 0 | 0 |

The resumed window had at most two matching processes, a 63.214ms p95 sampling gap and 112.812ms maximum
gap. It ended with one process and therefore provides **no Quit/post-exit coverage**. The independent
Quit window had at most one process, a 62.557ms p95 gap and 79.201ms maximum gap. Its first owned sample
was 26.506s after observer start and its last was 48.553s; approximately 41.480s remained after that last
owned sample, ending with zero processes following the observed Ctrl+Q exit.

These remain sampled process-owned endpoint observations, not proof of zero network activity. Short
flows between samples, DNS Client, brokers and unrelated system services remain outside coverage; no
packet/DNS/ETW trace was performed. Neither window proves all 56 live tool effects or clean-machine
installer behavior. The pending manual, assistive-technology, hardware, signing and attribution gates
listed above remain open.

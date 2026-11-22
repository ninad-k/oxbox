# Native privacy acceptance

This record separates reviewed application behavior, reproducible tests and bounded runtime observations.
It does not certify that every operating-system service or user-installed provider is network-free.
The release remains a draft while the open gates in [RELEASE.md](RELEASE.md) remain unresolved.

## Scope and evidence

The current maintenance source is `c30a179`, EXE SHA-256
`006769afce34f73fd4b1b9eee2216356e3f8ac7f39101d8fcf1587a74d8cb841`.
[Three separate bounded endpoint observations](RUNTIME_PRIVACY.md#current-maintenance-candidate-c30a179--2026-11-22)
passed with zero owned TCP/UDP rows; only the final restart/Quit window ends with no owned processes.
These supplement the older `67f0715` table below; its provider/49-form results retain their own hash scope.

New privacy boundaries: Browser Review reads extension metadata only after an explicit scan; its GUI check
used a generated root. Screenshot capture defaults to memory-only review, with explicit flattened export;
the new GUI case opened a generated PNG instead of capturing a private desktop. Text cleanup accepts
explicit input and leaves history paused. Presets exclude content/paths/credentials/active sessions.
Startup/registry backups contain commands/paths and should stay private. File Shred requires separate
permanent-action confirmation and does not guarantee SSD, snapshot or backup erasure. See
[the guide](MAINTENANCE.md) and [current verification receipt](verification/2026-11-22-maintenance-workflows.md).

Review date: 2026-11-22. Production host: `crates/oxbox-native` (eframe), with tool logic in
`crates/oxbox-core`. The retained Tauri/WebView2 host is historical and is not part of the native binary.
[RUNTIME_PRIVACY.md](RUNTIME_PRIVACY.md) retains the renderer investigation and exact measured artifacts.

| Boundary | Evidence | Acceptance and limits |
| --- | --- | --- |
| Native startup and owned endpoints | Current source `67f0715`, exact EXE SHA-256 `6606f3d4e172e75bbebbe4201d92b5989fca15ab51b2769cac0893dbac412069`: separate **300.066 s / 6,000-row** and **120.039 s / 2,400-row** strict observations. [Full timings and scope](RUNTIME_PRIVACY.md). | Both completed their requested duration with zero query/marker errors, stable hashes and zero all-TCP/UDP maxima and sums across every row. The first covers startup but ends with one app alive: no post-Quit claim. The second begins with an already-running host and ends with zero owned processes, 41.534 s after the last owned sample: no startup claim. Do not merge them into a continuous lifecycle trace. |
| Native form exposure | A separate exact-67 Windows GUI run opened **49/49 unique canonical default forms**, checking headings/Back/control presence and keyboard search; idle Timer preview and QR edit invalidation were also observed. [Feature-pass evidence](verification/2026-11-22-native-feature-pass.md). | These checks preceded the endpoint windows. They do not prove all 49 effects, exports, hardware or per-tool privacy. No camera/microphone recording, screen capture or clipboard history was started. Opening the media forms does not start their providers or privacy-sensitive capture; the paused clipboard view snapshots only in-memory history. |
| Controlled provider Cancel / hide / Quit | Current-67 second window used only a generated 120-second waiting provider in the app's child-only `PATH`. Native Cancel removed one child after approximately 11.571 s; hide/activation retained a second busy child, and Quit removed host and child after approximately 35.453 s, before natural exit. | No output was created. This proves the stated native app/fixture ownership paths, not actual FFmpeg decoding or device/provider privacy. The first window's earlier 30-second fixture ended naturally before Cancel and is **not** a cancellation pass. Historical source-a7/source-d8 measurements remain evidence for their own hashes. |
| Intentional application networking | Source review of `tools/currency.rs` and native currency controls. | The only application HTTP feature is a fixed ECB HTTPS feed, separately opted in and manually refreshed. Startup/ticks do not refresh. Opening an explicitly selected help link delegates to the user's external browser. Neither action was invoked during this privacy review. |
| Provider discovery | Source review of `media_provider.rs` and native provider controls. | Checks run only after **Check local providers**. Providers are not downloaded, installed or bundled. Relative `PATH` entries, Windows UNC/device paths and remote/unknown Windows drive types are rejected before probing files. Windows reparse aliases are rejected conservatively; put the actual local provider `bin` directory in `PATH`. |
| Provider file inputs/outputs | Source review and pure path-policy regression fixtures. | Conversion/GIF inputs must resolve to regular files; provider staging/output folders use the same Windows local-path checks. FFmpeg receives literal arguments, a fixed demuxer whitelist and `file,pipe` protocol whitelist before `-i`. Windows network/device paths and reparse redirects are refused before canonicalization. A protocol whitelist alone does not constrain SMB paths opened through the OS `file` protocol. |
| Provider execution | Bounded output readers, timeout/cancel checks, owned-process cleanup and no shell command strings. | Provider checks use a five-second/64-KiB bound; conversion jobs have time/output limits. Status checks share the caller's cancellation token. Process-tree containment regressions cover a wrapper that exits or waits while its bounded descendant inherits the output pipe. Actual provider DNS/traffic has not been observed by this review. A user-installed executable and its libraries/configuration remain a separate trust boundary. |
| DNS and broker traffic | Read-only observer feasibility check described below. | **Open.** No DNS trace was captured. Neither zero owner-PID endpoint rows nor a quiet DNS cache proves zero queries. OS brokers and flows between samples remain outside the prior endpoint evidence. |
| Passphrases | Native encryption draft/lifecycle, headless undo regression and core `encrypt.rs` review. | Fields are masked and discard widget undo history every frame; navigation/hide clears drafts; starting moves the passphrase into an age `SecretString`. Passphrases are not serialized, logged or sent to a provider. Errors do not include supplied passphrase contents. UI `String::clear()` is not a secure-memory-erasure guarantee; swapping, OS crash dumps and other privileged memory access are outside this claim. |
| Encrypted/decrypted data | Streaming/authentication/overwrite/cancellation regression coverage and explicit scrypt policy. | Originals remain untouched; output is create-new. Failed authentication, truncation, corruption and cancellation remove the new incomplete output, or report a cleanup failure explicitly. Unix outputs are created mode 0600. The age header is bounded to 64 KiB. New files use factor 18; decryption accepts factors through 20. |
| Clipboard | Core service, native consent and shutdown review; fake/no-OS lifecycle regressions. | History starts paused, remains memory-only and is bounded to 200 items/50 MiB including pins. Start requires explicit consent. Pause/clear/consent withdrawal order against pending Start; Quit prevents a late watcher or clipboard writer restart. Windows marked concealed content is skipped conservatively. macOS/Linux concealed-format detection has not been implemented. |
| Clipboard retention | In-memory history and explicit copy behavior. | Clearing history does not clear the system clipboard. An explicit Copy can remain available through the OS or another clipboard manager after Oxbox exits. Clearing/dropping history does not promise forensic memory erasure. No real clipboard was read during this review. |
| Profile storage | `preferences.rs`, native host and tool save/load call sites. | Preferences are ordinary local JSON, not encrypted. They can contain chosen paths, app lists, reminders, settings and preserved unknown legacy fields. Current code does not save passphrases, clipboard history, image pixels or recording streams into the profile. OS permissions, roaming/sync configuration, backups and privileged access remain external boundaries. |
| Devices/capture | Default constructors, explicit controls and managed-controller lifecycle review. | No camera, microphone, screen capture or clipboard collection is started by opening the app. Live privacy-sensitive acceptance was not invoked. Hardware/provider and macOS/Linux runtime gates remain open. |

## DNS observation feasibility on this host

The following checks read only log/provider metadata and token capability booleans. No event log was
enabled, cleared or reconfigured, and no system DNS-cache contents or unrelated domains were collected.

- `Microsoft-Windows-DNS-Client/Operational` exists but is **disabled**. Its provider metadata exposes
  event schemas 3006 and 3008; schema presence is not evidence that events are being collected.
- `Microsoft-Windows-Sysmon/Operational` is absent.
- The current token is neither Administrator, Performance Log Users nor LocalSystem.
- No new ETW session, packet capture, driver, group membership, firewall rule or policy change was made.

Microsoft documents the rights required to control cross-process trace sessions and recommends the
smallest possible collection scope. Those rights are unavailable in this session.
[StartTraceW documentation](https://learn.microsoft.com/en-us/windows/win32/api/evntrace/nf-evntrace-starttracew).
Sysmon can provide process-attributed DNS query events when already installed/configured, but installing
or configuring its service/driver is outside this check.
[Sysmon event reference](https://learn.microsoft.com/en-us/sysinternals/downloads/sysmon).

A suitable preconfigured disposable environment can close this gate without changing the user's host:

1. Begin its existing DNS/process observer before launching the hash-pinned app. Verify that its event
   source is active and can attribute a controlled generated query to the correct process/creation time.
   Retain counts and fixture identifiers; do not publish unrelated names, addresses or command lines.
2. A `DnsQuery_W` control with `DNS_QUERY_NO_WIRE_QUERY` is documented to search only the local cache.
   It can test a cache-only API call without sending a query, but **cannot establish on-wire DNS coverage**.
   A missing event for this control must not be reported as an app privacy pass.
   [DNS option reference](https://learn.microsoft.com/en-us/windows/win32/dns/dns-constants).
3. Observe ordinary startup/idle/Quit, then separately approved provider operations using only generated
   fixtures. Record observer failures, provider/app hashes, exact actions, start/end times and post-Quit
   process disappearance. A source audit and a positive control supplement the trace; neither replaces it.

Under the present no-elevation/no-settings-change constraints, this host has no verified usable DNS
event source. The correct result is **not measured**, not zero DNS. A packet observer would additionally
be needed for transport-level claims, including non-DNS traffic and broker attribution.

## Reproduce bounded endpoint observations

The reviewed aggregate observer is now available as
[`scripts/observe-native-endpoints.ps1`](../scripts/observe-native-endpoints.ps1). Run it from the checkout
root on Windows **before** a separately controlled launch. It never launches or stops an app. It requires
an exact `oxbox-native.exe` below `target/`, a mandatory verified SHA-256 and a fresh output subdirectory
below `target/native-privacy-qa/`. It rejects reparse-point ancestors and never replaces existing evidence.

```powershell
pwsh -NoProfile -File scripts/observe-native-endpoints.ps1 `
  -ExpectedExecutable 'D:\path\oxbox\target\qa-unique\oxbox-native.exe' `
  -ExpectedSha256 '<verified 64-character SHA-256>' `
  -OutputDirectory 'D:\path\oxbox\target\native-privacy-qa\fresh-run' `
  -DurationSeconds 360 -IntervalMilliseconds 50
```

Wait for that run's `ready.json` before launching. Record GUI action times separately, or write a short
alphanumeric/hyphen phase name into its `phase.txt`. A `stop.txt` marker can end the observer early;
the actual duration remains in the summary. Reserve enough time for startup, idle, selected actions,
Quit and post-exit observation. Retain incomplete/failed attempts rather than relabeling them as passes.

`samples.jsonl` contains only elapsed time, phase and owned-process/endpoint aggregate counts.
`summary.json` includes actual sample gaps, errors, observed own-process names, first/last ownership,
all-TCP (including listeners/loopback) and UDP maxima/sums, and a final executable-hash consistency check.
A normal-completion flag without any observed target process is not a valid target observation. These
snapshot validity checks do not close DNS, packet, OS-broker, short-flow or provider gates.

Tests (`node --test scripts/observe-native-endpoints.test.mjs`) use generated non-executable file contents
and reject outside/UNC paths, existing evidence and hash mismatches before sampling. A separate test
extracts only the pure validity function through PowerShell's AST and checks six fake fact sets. It
refuses query/marker failures, incomplete windows, a missing target or a changed hash. These tests do
not perform a runtime observation. This promoted script is a reproducibility aid; the earlier
accepted B run used its retained ignored predecessor, not the later script revision.

## Source 54dd593 fake-provider observation

On 2026-11-22, the first live use of the published observer began before a Root-controlled launch of
source `54dd593`, executable SHA-256
`428b28603ec2c16d825e833250fba1cb1a1dd06d2d9d6ef1f78b6e4253c94273` (13,523,456 bytes).
Root used a fresh `--profile` and a child-only `PATH` prefix containing two generated fixture executables.
Their identical SHA-256 was
`f96734523203849447f1db67dc6a9f1a5b88f5e65635f49e4a179b3c225519e4`.
The fixture's normal version mode waits 500 ms and prints fixed text; it never opens a device or socket.

| Measurement | Result |
| --- | --- |
| UTC interval | 2026-11-22 03:39:15.235 to 03:45:15.418 |
| Duration / successful samples | 360.095 s / 7,199 of 7,199 |
| Query / marker failures | 0 / 0 |
| First / last sampled ownership | 33.158 / 203.955 s |
| Sampled ownership span / after last ownership | 170.797 / approximately 156.140 s |
| Requested interval; actual p95 / maximum gap | 50 ms; 62.815 / 136.954 ms |
| Maximum owned processes / at end | 3 / 0 |
| Observed own-process names | `oxbox-native.exe`, `ffmpeg.exe`, `conhost.exe`, `tesseract.exe` |
| Samples with multiple owned processes | 37 |
| Every-row all-TCP maximum and sum | 0 and 0, including listeners/loopback |
| Every-row UDP-binding maximum and sum | 0 and 0 |
| Executable hash at end | Unchanged |

Root separately observed Settings/control naming, generated Japanese/café text preservation, Unit
Converter output/error semantics, both explicitly requested fixed provider versions with the expected
fixture paths, and Ctrl+Q followed by disappearance of the native process. Phase labels are coarse
annotations and can lag the actual action; ownership timing comes from samples, not the phase names.
The sampler performed no GUI action or app launch. The complete run was independently checked from
all raw rows, requiring zero query **and marker** failures and the unchanged executable hash.

Only fake version checks were exercised. No actual provider conversion/decoding, camera, microphone,
recording, screen capture, clipboard collection or ECB request occurred. DNS, packets, OS brokers,
short-lived flows and macOS/Linux limits remain open. Raw aggregate evidence is retained locally in
ignored `target/native-privacy-qa/run-review-provider-native/`; no addresses, ports or command lines
are exported.

This is source-54 evidence, **not exact-binary acceptance of the later provider-completion accessibility
fix**. After the run, the observer's validity predicate was corrected to require zero marker failures;
the original raw run/summary was preserved. Its independent strict assessment already had zero marker
failures, so that correction does not change this run's result.

## Exact final d8 executable: separate completion and active-Quit windows

Both observations used source `d8d1d1eb1c1f26cc54ebc80b623e0c9e09a17ad5`, the exact executable
SHA-256 `af363cd668e412f111bdfb37de39738ff35089e993585b19725932ba2de74b26` (13,524,480 bytes),
and published observer SHA-256
`9ecd93ed4f70b9717173d3075c4301668be3b3c50476cd0bc47781163631fdf0`.
Each began before a separate Root-controlled launch with a fresh isolated profile and child-only fixture
`PATH`. Executable, observer and fixture hashes were checked before/after; no host environment or OS
security/privacy setting was changed. These are **two separate windows**, not one continuous trace.

| Measurement | Provider completion / normal Quit | Controlled active-job Quit |
| --- | --- | --- |
| UTC interval | 03:53:30.654–03:57:30.800, 2026-11-22 | 04:00:50.210–04:04:50.328, 2026-11-22 |
| Duration / successful rows | 240.050 s / 4,800 of 4,800 | 240.064 s / 4,799 of 4,799 |
| Query / marker failures | 0 / 0 | 0 / 0 |
| First / last sampled ownership | 31.307 / 157.259 s | 39.250 / 214.989 s |
| After last sampled ownership | Approximately 82.791 s | Approximately 25.075 s |
| Actual p95 / maximum sample gap | 62.782 / 91.348 ms | 62.664 / 133.524 ms |
| Maximum owned processes / at end | 3 / 0 | 3 / 0 |
| Samples with multiple owned processes | 264 | 276 |
| Every-row all-TCP maximum / sum | 0 / 0, including listeners/loopback | 0 / 0, including listeners/loopback |
| Every-row UDP-binding maximum / sum | 0 / 0 | 0 / 0 |
| Final executable hash | Unchanged | Unchanged |

The completion window observed `oxbox-native.exe`, `ffmpeg.exe`, `conhost.exe` and `tesseract.exe`.
The generated version-only fixtures each waited approximately three seconds and printed a fixed
version. Their SHA-256 was
`0a842e2fd462e46b00ef8dc1e59cc9e55ac7f62ddcdaf728358755a1cf276a1d`.
Root separately verified both fixture paths/versions and the actual accessible **Status** text
“Checked 2 local providers,” followed by normal Ctrl+Q exit.

An attempted active-Quit check in that first window remains **inconclusive**. The child was observed alive
at 03:55:57.544, but the post-proof Quit action was over eleven seconds later; both three-second probes
could finish first. Samples containing multiple owned processes ended at 150.307 s while the native
cohort lasted to 157.259 s. Neither cumulative process names nor an earlier Working state proves that
a child was active when Quit was requested.

The second window resolves that timing uncertainty with a different, explicitly reviewed no-effect
fixture, SHA-256 `dda7ce47a02ef816ec8dd53e458cbc71048b5b4500600021a280124e2fb852a9`.
Its version mode prints fixed text; every nonversion invocation only waits 30 seconds and prints a
constant. It never opens input/output files, decodes media, accesses a device/socket or explicitly launches
another program. The job window observed `oxbox-native.exe`, `ffmpeg.exe` and the Windows `conhost.exe` helper.
Root supplied a generated 51-byte inert `.mp4` input and an absent output path; no actual FFmpeg codec
or physical device was involved.

Root's retained before/after JSON shows the controlled child created at **04:04:11.271519 UTC**, alive
alongside the UI Working state at **04:04:13.7156599**, and both app/provider absent after Ctrl+Q by
**04:04:26.0774154**. Its earliest possible natural 30-second completion was **04:04:41.271519**:
it was already absent approximately **15.194 seconds earlier**. Sampled multiple-process ownership
spanned 201.096–214.834 s, and all owned processes disappeared after 214.989 s. This is a **bounded
active-job Quit pass**, supported by process timing rather than a natural fixture completion.

The generated input remained 51 bytes with SHA-256
`d02ec550191f3ea6ff2c7e6f2bc15e45bbb081524114ee0162c4ae20294a0c8f`.
The output remained absent, and the fixture folder contained only the input afterward: no staging
directory remained. This tests managed-job cancellation/cleanup, not successful media conversion.

Raw rows, pins and independent strict assessments are retained in ignored
`target/native-privacy-qa/run-review-final-provider-native/` and `run-review-final-active-job/`.
The latter also retains `root-before-long-job-quit.json` and `root-after-long-job-quit.json`; their actual
fields were checked when deriving the active-Quit assessment. Every successful raw row was aggregated
independently, with zero all-TCP/UDP sums/maxima, zero failures and final hash consistency required.

No camera, microphone, recording, screen capture, clipboard collection, actual media-provider decoding
or ECB request was invoked. DNS, packets, OS-broker activity, short-lived flows, actual provider binaries
and macOS/Linux runtime behavior remain outside these observations. No universal zero-network claim is made.

## Remaining safe provider and profile checks

Root owns all GUI control and app launches. Use a new isolated `--profile` directory and preserve existing
profiles. No camera/microphone/private capture/clipboard monitoring, power action or real network request
is authorized by these checks.

Version dispatch/completion and managed-job Quit/cleanup now have the bounded fixture evidence above.
The plans below describe broader provider/decoding and profile checks; the completed mock checks must
not be substituted for actual provider, DNS or hardware acceptance.

- Use a generated local fake `ffmpeg`/`tesseract` executable in a per-child `PATH`, without mutating the
  user's environment. It should recognize only the expected version/fixture invocations, write only
  allowlisted generated argument markers and create a small fixed output or generated OCR text. Do not
  dump the inherited environment or collect other process command lines.
- Confirm no provider child appears at startup or simply opening provider views. Click the provider
  check explicitly, verify the expected child path/hash, bounded output and cancellation on Quit. A fake
  provider can verify dispatch and lifecycle, but cannot validate an actual provider's network behavior.
- Convert a generated local audio/video fixture and recognize generated text; capture app/provider
  descendants throughout. Pin the actual provider version/hash before separately assessing that binary.
  No recording input or physical device is needed.
- Use a generated passphrase sentinel only in a disposable encryption fixture. Check the resulting
  profile and app-generated diagnostics for that sentinel without printing it; verify authenticated
  round-trip, wrong-passphrase failure and no remaining partial plaintext. Do not inspect user secrets.
- Exercise clipboard policy with injected/generated in-memory content and fake delayed Start tests.
  Do not enable live history to obtain a test fixture.

## Encryption resource policy

Oxbox uses the supported age scrypt APIs, without custom cryptographic primitives or a new calibration
algorithm. New encrypted files use `log2(N) = 18`; decryption sets `max_log2(N) = 20`. With age's `r=8,p=1`,
the main scrypt allocation is approximately 256 MiB for new files and up to 1 GiB for accepted decryption,
**plus library/application overhead**. This is a resource ceiling for that KDF allocation, not a guarantee
against general memory exhaustion. A synchronous KDF call cannot be interrupted mid-derivation; cancellation
is checked around it and during payload streaming.

The format remains standard age. Historical Oxbox/other age files using factors above 20 are deliberately
refused with a distinct safe-limit error, leaving the encrypted input unchanged. They may require an
external compatible tool with a separately assessed resource budget; this change is not backward
compatible with every valid age work factor. The locked age library previously calibrated its default
maximum from CPU timing plus four factor bits, rather than a fixed memory limit.

The excessive-factor fixture uses factor 99 and includes a structurally valid header and payload nonce.
The test first verifies that the age parser accepts it. Factor 99 exceeds even the library's theoretical
default maximum (calibrated target below 64 plus four), so the baseline rejects it without running an
expensive derivation. Its meaningful red assertion is the missing distinct safe-limit error. The existing
ordinary round-trip test additionally inspects the generated factor-18 stanza.
[age scrypt format](https://c2sp.org/age#scrypt-recipient-stanza).

The masked egui password editor previously kept full plaintext undo snapshots even after the draft
string was cleared. The native fields now clear and store their `TextEditState` undoer immediately after
each edit frame. A headless fixture focuses a generated password, clears the draft and checks that no
undo entry or in-progress undo snapshot can restore it, for both password fields. This removes logical
undo retention; it does not claim forensic erasure of all toolkit/allocator/OS memory.

The source guards are defensive, not an OS sandbox: Windows drive/reparse checks can race changes by
another process. Unix/macOS paths can refer to user-mounted network filesystems; this review does not
classify every mount or prove a user-managed provider/configuration is network-free. FFmpeg protocol and
format whitelists constrain Oxbox's planned file inputs; they do not sandbox arbitrary provider code.
[FFmpeg protocol options](https://ffmpeg.org/ffmpeg-protocols.html#Protocol-Options),
[Windows drive types](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew).

The demuxer whitelist excludes HLS, concat, DASH, IMF, image2, SDP and script-loading formats. Generated
M3U8/ffconcat/MPD regression files contain inert external references; the tests only inspect the fixed
argument plans and do not invoke a provider or follow those references. For the supported MOV/MP4 demuxer,
FFmpeg documents both external-track loading (`enable_drefs`) and absolute-track paths
(`use_absolute_path`) as disabled by default. Oxbox does not enable either option. No nested-file bypass
has been reproduced by this review; these facts are not a proof that every provider version/decoder is
free of defects. [FFmpeg MOV input options](https://ffmpeg.org/ffmpeg-formats.html#mov_002fmp4_002f3gp).

## Verification log for this change

- Local-path policy: initial failing compile for the three new pure fixtures was observed by Root in
  `target/privacy-provider-red.log`; all are green in the coordinated run below.
- Provider cancellation: two fake-runner regressions failed to compile before the helper, recorded in
  `target/privacy-provider-cancel-red.log`; both are green in the coordinated run below.
- Encryption resource policy: the corrected factor-99 fixture parsed successfully and produced the old
  generic passphrase error in `target/privacy-encryption-corrected-red.log` (0.09 s). The distinct-error
  assertion failed as intended. The first attempt omitted the payload nonce, failed earlier in parsing
  and is not accepted as evidence of the resource-policy regression. The corrected fixture and the
  existing round-trip/header-factor check are green in the coordinated run below.
- Native password undo retention, selected-path Remove naming and archive extraction spinner naming:
  all three meaningful assertion failures were observed in `target/accessibility-privacy-context-red.log`
  before their fixes. Initial compile-only attempts are not counted as behavior evidence. All three
  are green in the coordinated run below.
- Initial coordinated Rust 1.95.0 Windows validation before the provider-completion-status follow-up:
  **208/208 core tests** (106.16 s), **64/64 native tests**
  (0.31 s), zero failed/ignored. This includes the new path/cancellation/manifest/KDF/password fixtures.
  Strict all-target clippy and format checks also passed. Root retained logs in
  `target/review-clippy.log` and `target/review-rust-tests.log`.
  The later source-d8 build/CI record is maintained in [VALIDATION.md](VALIDATION.md); the two exact-binary
  source-d8 observations above are separate from that initial test-count record.
- Observer guard tests: two Windows tests failed before the script existed, then passed after the
  strict path/hash/new-output guards were implemented. No observation was started by these tests.
- After the completed source-54 live run, an extracted pure-validity regression reproduced the missing
  marker-error check: a fake one-marker-failure observation was incorrectly accepted while the other
  five cases behaved correctly. Adding the marker-failure condition made **3/3 observer tests green**
  (2.53 s). Logs: `target/observer-validity-red.log` and `target/observer-validity-green.log`. The test
  parsed/executed only that function, not the sampler.
- No new dependency, host setting change, live device/clipboard action or app launch was made by this lane.

Update this log with exact test results and runtime evidence as checks complete. Do not promote the
unmeasured DNS/provider/platform gates based only on source review or unit tests.

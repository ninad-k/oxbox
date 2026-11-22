# Runtime privacy and network acceptance

Oxbox implements no application telemetry. Tool commands operate locally; the only intentional application
network feature is separately opted-in, manually requested ECB currency refresh, approved in
[issue #1](https://github.com/ninad-k/oxbox/issues/1). Its request contains no user amounts or files.
The approved native migration removes the embedded browser from the production host. Finite network observations
do not establish a universal zero-network guarantee; the historical browser investigation is retained below.

## Current maintenance candidate c30a179 — 2026-11-22

Source `c30a179a573424b178def065a3c5c0f5c9397074`, Rust 1.95.0, Windows EXE **14,381,056 bytes**, SHA-256
`006769afce34f73fd4b1b9eee2216356e3f8ac7f39101d8fcf1587a74d8cb841`. The selected graph still contains
no embedded browser host; dependency manifests and the read-only endpoint observer are unchanged.
The following are **three separate observations**, not continuous full-tool or universal network coverage.

| Local evidence directory under `target/native-privacy-qa/` | Duration / successful samples | Scope / last sampled owned processes | Maximum / p95 sampling gap |
|---|---|---|---|
| `run-maintenance` | **300.103 s / 6,001** | Starts before launch and covers part of the generated-tool GUI pass; **1** remains at the end. No Quit claim. | 86.029 / 62.719 ms |
| `run-maintenance-quit` | **90.037 s / 1,800** | Begins with an app alive; includes hide/reactivation and a brief second activation process; **1** remains at the end. Despite the directory name, this window does not cover completed Quit. | 79.249 / 62.698 ms |
| `run-maintenance-final` | **60.031 s / 1,200** | Separate restart and explicit Quit; first/last owned samples **9.402 / 49.964 s**, **0** at the end, about **10.06 s** of post-Quit samples. No active provider/destructive job. | 87.802 / 62.801 ms |

All three completed normally, observed the target, retained the exact hash, and had **zero sample/marker
errors**. Every row had zero owned TCP rows and UDP bindings; TCP/UDP maxima and sums were zero. The
new GUI pass used generated files/text/extension metadata and no network opt-in, live browser scan,
clipboard history, screen capture, hardware/provider, Trash, shred or registry/startup mutation. Later
GUI work is not retroactively covered by an earlier finite window.

Owner-PID snapshots can miss short flows and OS-broker/DNS/provider traffic. UDP bindings are not packet
counts. No packet capture, DNS/ETW trace, addresses or ports were recorded. DNS attribution, actual
providers, all-tool effects and macOS/Linux runtime privacy remain open; zero rows are bounded evidence.

<a id="current-native-candidate-67f0715--2026-11-22"></a>

## Previous native candidate 67f0715 — 2026-11-22

Source `67f0715452fb601394f45d3296675d3b85df3fb5`, built with Rust 1.95.0, was observed using the exact
13,577,216-byte Windows executable, SHA-256
`6606f3d4e172e75bbebbe4201d92b5989fca15ab51b2769cac0893dbac412069`.
The published read-only observer remained pinned to SHA-256
`9ecd93ed4f70b9717173d3075c4301668be3b3c50476cd0bc47781163631fdf0`.
These are **two separate windows**, not a continuous startup-to-Quit trace.

| Measurement | Startup and first controlled job | Already-running host: Cancel, hide/restore and Quit |
|---|---|---|
| UTC interval, 2026-11-22 | 07:03:13.756–07:08:13.875 | 07:08:26.593–07:10:26.678 |
| Full requested window / successful samples | 300.066 s / 6,000 of 6,000 | 120.039 s / 2,400 of 2,400 |
| First / last sampled ownership | 30.513 / 299.964 s | 0.025 / 78.505 s |
| Sampled ownership span | 269.451 s | 78.480 s |
| Owned processes at end / maximum | 1 / 4 | 0 / 4 |
| Post-Quit ownership coverage | Not established: an app was still alive at the end | 41.534 s after the last owned sample |
| Requested interval; actual p95 / maximum gap | 50 ms; 62.884 / 104.766 ms | 50 ms; 62.729 / 76.270 ms |
| Query / phase-marker failures | 0 / 0 | 0 / 0 |
| Every-row all-TCP maximum / sum | 0 / 0, including listeners and loopback | 0 / 0, including listeners and loopback |
| Every-row UDP-binding maximum / sum | 0 / 0 | 0 / 0 |
| Executable / observer hashes at end | Unchanged / unchanged | Unchanged / unchanged |

Independent assessment of all 8,400 rows found monotonic, well-formed samples, consistent row/summary
counts, ownership timings, gaps and endpoint aggregates, matching ready/pin metadata, and completion
of each requested duration. All recorded TCP subclasses—loopback, non-loopback, established non-loopback
and public—and UDP bindings had zero maxima and sums. The observed own-process names were
`oxbox-native.exe`, generated `ffmpeg.exe` and `conhost.exe`; deliberate second-instance activation can
add a transient native process. Neither assessment labels its window a complete before-startup-to-post-Quit
lifecycle: the first ended with an app alive, and the second began with the host already alive.

The second window's first raw row already has one owned process at elapsed **0.025 seconds**. Its summary
stores `firstOwnedSeenMs` in milliseconds; this is not a 25-second delay or startup coverage. Phase names
are coarse annotations: even its initial default `before-launch` label does not imply that the app had
not started. Ownership comes from the sampled rows and separate process evidence.

### Controlled job evidence and limits

The first window began before host 20556 launched. Its generated 30-second provider stayed alive with
the same busy tool after hide and second-instance restoration approximately 28 seconds into the job.
It then finished naturally before Cancel, producing the expected missing-output error. **That attempt
does not establish cancellation.** Host 20556 quit, and host 10684 subsequently started during the same
window; host 10684 was still alive at its end. There is no first-window post-Quit claim.

The second window used an already-running, isolated-profile host 10684 and a generated **120-second**
no-effect provider executable, SHA-256
`fcafe14121ea093f91cb7d5eab819384ad6d642276fe33861b2ba6b7aa233c19`, supplied only through that app's
child `PATH`. The fixture's version branch prints fixed text; its job branch only waits and prints a
constant. It does not decode media, open files/devices/sockets or print arguments/environment data.

- **Explicit Cancel:** provider 43788 was created at 07:08:45.412 UTC. After native Cancel, it was absent
  by 07:08:56.983 UTC, approximately 11.571 seconds later, while the host remained alive. The native UI
  displayed Cancelled and no output was created. This precedes its 120-second natural exit.
- **Hide/restoration and active-job Quit:** provider 21256 was created at 07:09:09.982 UTC. After hiding,
  second-instance activation restored the same busy tool with the same provider PID and Cancel control.
  Native Ctrl+Q left both host and provider absent by 07:09:45.434 UTC, approximately 35.453 seconds after
  child creation and well before natural exit. No output was created; the observer then continued with
  no owned processes, including 41.534 seconds after the last owned sample.

Raw aggregate evidence, pins, Root scenario records and independent strict reports remain in
`target/native-privacy-qa/run-verification2/` and `target/native-privacy-qa/run-verification2-final/`.
Separate sanitized GUI evidence under `target/native-qa/verification2/` records opening all 49 canonical
default forms, headings/Back/control presence, an idle 10-minute Timer preview and QR edit invalidation.
That GUI run preceded these observation windows. It is not every tool's end-to-end acceptance and is
not network observation of all 49 forms. See [the dated feature pass](verification/2026-11-22-native-feature-pass.md).

No real provider codec/device, live clipboard collection, screen capture, OS settings change, destructive
action or ECB request was used in these windows. The separate real synthetic FFmpeg core smoke in the
feature record is not part of these endpoint traces. Snapshot timing, DNS/OS-broker/driver attribution,
packets, actual-provider and other-OS runtime gates remain open. No host security/privacy/network setting
was changed, and no universal zero-network claim or release publication is authorized by these results.

## Earlier community candidate a7c91bd — 2026-11-22

Source `a7c91bd696af0f80c86029336593e3d37d64f78b`, Rust 1.95.0 Windows EXE SHA-256
`1c8715a5b9739ad1142bc91475d552ea0384fbd538651704d4f687da6756c7c3` (13,581,312 bytes), completed a
separate **240.073-second / 4,800-sample** observation with the published read-only endpoint observer.
The script hash remained `9ecd93ed4f70b9717173d3075c4301668be3b3c50476cd0bc47781163631fdf0`.

The window ran from 05:13:58.8828345 to 05:17:59.0498734 UTC, before startup through isolated-profile
World Time / EAN-13 interactions, normal Quit and post-exit observation. The independent all-row assessment
found **zero** owned TCP rows/bindings of every recorded class and UDP bindings (both maxima and sums),
zero query/marker failures, stable executable/script hashes and no process at the end. Only the native EXE
appeared, at most one process. First/last ownership was 25.451 / 155.318 seconds; post-last coverage was
84.755 seconds. Requested interval was 50 ms; p95/maximum gaps were 62.715 / 99.123 ms.

UI evidence independently confirms exact `America/New_York` search, clock creation with EST / UTC−05:00,
12-digit EAN-13 preview and normal sidebar Quit. Save/Copy were not invoked. No external providers, devices,
capture, clipboard collection, network refresh, OS settings or destructive tool action was used. An explicit
profile prevented startup registration. Raw evidence remains under `target/native-privacy-qa/run-community-native/`.

Owner-PID snapshots can miss short flows and do not measure DNS/OS-broker/driver or other-provider traffic.
This is bounded Windows evidence for that earlier executable, not a universal offline guarantee. Other runs retain their
own exact hashes/scenarios; generated-provider/active-job evidence is not silently transferred to this binary.
See [the final artifact mapping](VALIDATION.md#community-review-candidate-a7c91bd) and [remaining gates](RELEASE.md).

## Native migration — 2026-11-21

The selected renderer is exact eframe 0.37.0 with Rust 1.95. The Windows native normal dependency graph has
no Tauri host, Wry, WebView2 SDK or common HTTP client. `tauri-winrt-notification` is a local toast adapter,
not a browser host. The legacy implementation remains in the workspace but is not compiled into the native
application. Explicit currency refresh uses fixed system `curl.exe` after opt-in and a manual action;
startup and background ticks never call it. Imported legacy download consent starts off.

A first **debug-only**, clean-profile Windows observation began before launch and covered startup, idle,
read-only launcher/Unit Converter interactions and application Quit. Over **397.547 seconds / 7,845 samples**,
there were no observed owned TCP rows or UDP bindings, and no sampler failures. The maximum process tree
was the native executable plus its debug `conhost.exe`; no browser descendant appeared. All recorded owned
processes disappeared before observation ended. A later polished debug run independently verified favorites,
Alt+1, global Ctrl+Alt+O hide and second-launch restoration of the existing process.

The observer uses read-only Windows IP Helper owner-PID TCP/UDP tables and Toolhelp process ancestry.
Only aggregate counts are retained; no addresses, ports, command lines or unrelated process names are exported.
Requested interval was 50 ms, actual p95 start-to-start gap **62.96 ms**, maximum **551.38 ms**. Sockets wholly
between samples can be missed. UDP tables show bindings rather than destinations or traffic. Windows DNS
Client/cache, other OS brokers, drivers and explicitly opened external apps are outside this attribution;
no ETW/DNS or packet trace is claimed. This first run is debug evidence only.

### Bounded Windows production observations

The Rust 1.95.0 release executable from source `efdab7435bfe6d89b93ff6198779d031ab7f8826`
was checked by SHA-256 before each observation:
`6fb64ef38e8a5725c72ef614cf78fc304775d8ed8310606ac72f45a447ba91b5` (13,480,960 bytes).
These are separate windows, not one continuous trace.

| Window | Duration / samples | Observed scope | p95 / maximum sample gap |
| --- | --- | --- | --- |
| Production tools | 600.141 s / 12,000 | Before launch, startup, idle, Unit Converter, two finite Mouse Activity pointer moves and single-instance activation. The app was still alive at the last sample; Quit is outside this window. | 62.95 / 85.70 ms |
| Quiet-tray lifecycle | 201.768 s / 4,033 | Before launch, `--autostart` without a main window, activation of the existing instance, direct Quit, and approximately 95.8 s after the last owned process sample. | 62.83 / 79.28 ms |

Both windows completed normally with **zero owned TCP rows or UDP bindings observed**, zero query/phase-marker
failures and no browser/provider child. At most two native instances appeared during deliberate activation;
the quiet-tray window ended with no owned processes. UI behavior was observed separately from endpoint queries.
An earlier 73.593 s attempt ended on a phase-marker sharing error and is excluded from complete-run evidence.
The observer was corrected to tolerate atomic marker replacement and require normal completion.

This supports only the stated Windows executable, tools and intervals. The snapshot/DNS/OS-broker limits above
also apply to these production runs. Camera, microphone, recording, capture, clipboard history and ECB refresh
were not activated. Per-tool/provider, DNS and macOS/Linux runtime acceptance remain open; no universal
zero-network claim is made. Exact final-installer source/hash evidence is recorded in [VALIDATION.md](VALIDATION.md).

### Exact final executable lifecycle

The final source `7132daa231627925359410132e754673f68905cc` executable, SHA-256
`257b5b20f92724cc18dcf4e2167917d4d53385974f1521c87940ee1d72f9840c`, was pinned before a separate
**360.154 s / 7,201 sample** observation. It began before normal startup with a fresh isolated profile,
covered the empty Dashboard/49-tool library, brief idle, Unit Converter and direct Quit, and ended with
zero owned processes. First/last sampled ownership was at 22.662/77.051 s, leaving approximately 283.1 s
after the last owned sample. It completed normally with zero query/marker errors, no observed owned
TCP rows (including listeners/loopback) or UDP bindings and no descendants. Aggregate checks of all
7,201 rows confirmed zero sums and maxima for both endpoint counts. Actual p95/maximum sample gaps were
62.62/81.82 ms. This is the exact executable independently extracted from the final MSI.

An earlier 180.044 s / 3,600 sample final-executable window observed startup/brief read-only use but ended
with the process still alive. It does not cover Quit and is not combined with the complete lifecycle window.
All sampling/DNS/OS-broker/provider limitations above remain applicable.

No Windows/Edge policy, firewall, privacy or security setting was changed. The draft must remain unpublished.

## Final native review candidate — 2026-11-22

Source `d8d1d1eb1c1f26cc54ebc80b623e0c9e09a17ad5` was built with exact Rust 1.95.0. The observed Windows
EXE is 13,524,480 bytes, SHA-256 `af363cd668e412f111bdfb37de39738ff35089e993585b19725932ba2de74b26`;
the independently extracted final MSI contains the identical executable.

The reusable [observer](../scripts/observe-native-endpoints.ps1) was pinned by script SHA-256
`9ecd93ed4f70b9717173d3075c4301668be3b3c50476cd0bc47781163631fdf0`. Its validity policy requires normal
completion, a seen target, unchanged executable hash and zero query/phase-marker failures. Three safe
guard/policy tests cover invalid paths/hashes, existing evidence and invalid observation facts.

| Separate final-EXE window | Duration / samples | Scope | p95 / maximum gap |
| --- | --- | --- | --- |
| Provider completion | 240.050 s / 4,800 | Before startup, launcher/search, generated provider version checks, named completion Status, normal Ctrl+Q and 82.791 s after the last owned sample | 62.782 / 91.348 ms |
| Quit during an inert provider job | 240.064 s / 4,799 | Before startup, generated local input/output paths, an inert 30-second child job, Ctrl+Q during work and 25.075 s after the last owned sample | 62.664 / 133.524 ms |

Both windows independently completed with **zero owned TCP rows (including listeners/loopback) or UDP
bindings in every sample**, zero query/marker failures, unchanged hashes and zero owned processes at the end.
At most three owned processes appeared: native app, generated provider and console host. The completion
window observed both generated `ffmpeg.exe` and `tesseract.exe`; the job window used only generated `ffmpeg.exe`.
These are separate windows, not a single continuous trace.

For the active-job check, the child was observed alive with Working status, created at 04:04:11.271519 UTC.
After Ctrl+Q, root and child were absent by 04:04:26.077415 UTC, over 15 seconds before its earliest natural
30-second exit. The generated 51-byte input hash stayed unchanged; no output or staging files remained.
An earlier attempt with short version probes did not establish active-at-Quit timing and is explicitly
inconclusive for that claim; its ordinary completion/Quit endpoint window remains separately valid.

The generated programs only print fixed text or wait; they never decode media or open files/devices/sockets.
This verifies bounded app/child ownership and observed endpoints, **not actual FFmpeg/Tesseract behavior**.
DNS, OS brokers, packets/flows between snapshots, other tools/providers and other-OS runtime acceptance
remain open. No host security/privacy/network setting was changed. See [privacy boundaries](PRIVACY_ACCEPTANCE.md).

## Historical embedded browser investigation

The following findings apply to the retained Tauri implementation and its former draft assets, not to the
new native binary. They explain why supported WebView2 settings were insufficient for the selected boundary.

## Observed Windows release behavior — 2026-11-21

The production build from `5117e6d53943f050b6120e6abfddae17d51d5a9e` was tested without requesting
currency refresh or opening external pages. A 30-snapshot steady-state sample saw up to three established
non-loopback TCP connections among the app and its descendants. A separate 30-snapshot check normalized
IPv4-mapped addresses and found the app's WebView2 browser process holding `40.99.60.2:443` throughout.
The Rust host had no observed established connection. The installed WebView2 runtime was `157.0.4341.60`.

The public peer is within Microsoft's published `40.96.0.0/13` Exchange Online range. Address membership
does not establish which runtime feature made the request or what it contained; the exact purpose remains
unresolved. No matching hostname was found. These were TCP metadata snapshots, not TLS payload inspection,
UDP monitoring or a complete startup/long-term traffic audit. [Microsoft endpoint reference](https://learn.microsoft.com/en-us/microsoft-365/enterprise/urls-and-ip-address-ranges?view=o365-worldwide).

## Isolated diagnosis — 2026-11-21

Two controlled launches of that same release executable used Microsoft's diagnostic network log in
`Default` capture mode: one with the existing app profile, one with a new profile under the ignored
`target/runtime-privacy-qa/` directory. Only the launcher was opened; no currency refresh, external page,
capture, microphone, camera, recording, power or desktop action was requested. The diagnostic options were
passed only to each child process, and are not production configuration. Existing profile data was preserved.

| Profile | Runtime requests recorded in the diagnostic log |
| --- | --- |
| Existing app profile | `config.edge.skype.com/config/v1/Edge/157.0.4341.60` |
| Fresh isolated profile | The same configuration endpoint, plus `edge.microsoft.com/componentupdater/api/v1/update` |

Microsoft identifies `config.edge.skype.com` as its Experimentation and Configuration Service.
The other request targets the runtime's component updater. These observations establish that a fresh profile
alone does not eliminate runtime requests. They do not prove that a particular earlier TCP socket carried
one of these requests, or that either request uploaded user content. Queries, headers, identifiers and raw
logs are not published. [Microsoft endpoint reference](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-security-endpoints).

The user and machine WebView2 policy roots were absent, and the diagnostic launcher environment contained
no pre-existing `WEBVIEW2*` variables. The initial runtime arguments included Wry's default feature switches;
the new app-scoped environment below does not add or rely on those switches. Microsoft documents the
diagnostic log option and cautions against shipping browser flags in production.
[Diagnostic option reference](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags).

## Supported app-scoped controls

**Windows runtime notice:** Oxbox includes Microsoft Defender SmartScreen through WebView2. SmartScreen
collects and sends user information to Microsoft as described in the
[Microsoft Privacy Statement](https://privacy.microsoft.com/privacystatement) and
[SmartScreen privacy details](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy#smartscreen).
The same notice is visible in the app's Windows Settings. This is separate from Oxbox tool-command networking.

The Windows initialization now creates a dedicated `webview-v1` profile beneath Oxbox's own local app-data
directory. It preserves the previous profile. The main window and capture/focus overlays share one SDK
environment on the UI thread with:

- Exclusive access to that app profile; a loader override that changes the profile causes initialization to stop.
- OS-primary-account single sign-on disabled. This was already the SDK default and is not claimed as the cause.
- Custom crash reporting enabled. Microsoft crash upload is suppressed; Oxbox does not upload the local crash dumps.
- Browser extensions disabled and tracking prevention enabled; Microsoft reputation/security defaults are retained.

Each window starts at `about:blank`. A supported `ICoreWebView2_22` request filter is installed for documents,
shared workers and service workers before navigating to local app content. It refuses external web-resource
requests with a local 403 response. Exact local Tauri/IPC/asset origins are allowed; development adds only the
configured `localhost:1420` origin. Failure to install the filter stops app navigation. The status command exposes
counts only, never URLs or content. Currency networking stays in its separately consented Rust command.

These controls use the approved, already-locked `webview2-com 0.39.1` and `windows 0.62.2` SDK bindings.
No global Edge, Windows diagnostic, security, firewall, update or policy setting is changed. The SDK filter
controls web resources, and does not claim to intercept browser-owned background services.
[Request-source filter](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_22),
[exclusive profile](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environmentoptions2),
[custom crash reporting](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environmentoptions3).

## Embedded runtime limits

### Bounded production probe after the STA fix

An intermediate production CLI build containing the supported controls and explicit UI-thread COM
initialization was run with only per-child diagnostic logging and loopback DevTools enabled. It used the
expected `webview-v1` profile and did **not** contain Wry's previous SmartScreen-disable feature flag.
The native host and six WebView2 descendants started successfully.

The SDK status reported `configured: true`, zero blocked web resources and 14 local resources before the
fixed `example.invalid` navigation probe. The probe raised the blocked count to one; restoring the local
app URL produced 54 local-resource callbacks and working status IPC. The browser showed its built-in error
document for the denied navigation. This verifies SDK interception, not a guarantee that speculative DNS
or connection setup cannot occur: the diagnostic log also recorded a preconnect job for the probe URL.

The runtime initiated vendor requests despite that functioning filter:

- `config.edge.skype.com/config/v1/Edge/157.0.4341.60`
- `nav-edge.smartscreen.microsoft.com/api/browser/edge/navigate/3`
- `data-edge.smartscreen.microsoft.com/api/browser/edge/data/settings/3`
- `telem-edge.smartscreen.microsoft.com/api/browser/edge/telemetry/3`
- `edge.microsoft.com/componentupdater/api/v1/update`

Configuration and SmartScreen requests appeared **before** the external navigation probe. Component-update
requests appeared later in the same run. Their presence is consistent with preserving Microsoft's security
defaults; no protection was disabled to obtain a quieter measurement. Endpoint names do not establish which
payload fields were transmitted or prove that every attempted request completed successfully.

A 30-snapshot sample of the verified live host and descendants saw a maximum of one established public
HTTPS connection, plus loopback connections, and no owned UDP endpoints. UDP endpoint counts are not packet
capture and do not prove zero UDP traffic. These finite samples are not a complete startup or long-term audit.
After the app's own `quit_app` command, all seven recorded processes disappeared and the diagnostic listener
closed; the captured stderr contained no panic marker.

The run was an intermediate privacy probe before final default-route normalization and mouse-dispatch
integration. Final installer UI/overlay acceptance must use the final production build without diagnostic
flags. No camera, microphone, recording, capture, clipboard-recording, currency, power or desktop effect
was invoked in this probe. Raw logs remain local beneath ignored `target/runtime-privacy-qa/`.

Microsoft documents optional and required WebView2 diagnostics, with required collection outside the
application developer's overall control. Feature-specific APIs exist for crash reporting and reputation
checking; they are not a general zero-network guarantee. [WebView2 privacy documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy).

Microsoft describes browser flags as development/diagnostic mechanisms that should not ship in production.
An added `--disable-background-networking` flag is therefore not a supported promise that runtime traffic
has been eliminated. Only per-child diagnostic logging flags were used; production has no added browser flags.
[WebView2 browser-flag guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags).

Microsoft documents an enterprise `ExperimentationAndConfigurationServiceControl` policy that can stop its
configuration-service communication. It applies through the shared WebView2 policy root, and Microsoft warns
that disabling the service limits its ability to respond to severe bugs. Oxbox does not change that policy or
disable component/security updates. [WebView2 policy reference](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-webview-policies#experimentationandconfigurationservicecontrol),
[configuration-service caveat](https://learn.microsoft.com/en-us/deployedge/edge-configuration-and-experiments).

## Historical Tauri acceptance status

The strict whole-process offline requirement has **not passed on Windows**. Keep the release a draft until
the remaining browser-owned requests are resolved within the authorized app-scoped/security-preserving
constraints. Production testing confirms that the supported resource filter does not eliminate vendor
configuration, SmartScreen or component-update requests. Do not infer zero runtime traffic from the absence
of a Rust HTTP client, the CSP, the currency opt-in state or successful unit tests. macOS/Linux runtime
traffic has not been measured by this Windows observation.

The broader [validation record](VALIDATION.md) and [release checklist](RELEASE.md) track the other open gates.

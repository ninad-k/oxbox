# Runtime privacy and network acceptance

Oxbox implements no application telemetry. Tool commands operate locally; the only intentional application
network feature is separately opted-in, manually requested ECB currency refresh, approved in
[issue #1](https://github.com/ninad-k/oxbox/issues/1). Its request contains no user amounts or files.
This statement does not guarantee that the embedded operating-system webview makes no network requests.

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

## Acceptance status

The strict whole-process offline requirement has **not passed on Windows**. Keep the release a draft until
the remaining browser-owned requests are resolved within the authorized app-scoped/security-preserving
constraints. Production testing confirms that the supported resource filter does not eliminate vendor
configuration, SmartScreen or component-update requests. Do not infer zero runtime traffic from the absence
of a Rust HTTP client, the CSP, the currency opt-in state or successful unit tests. macOS/Linux runtime
traffic has not been measured by this Windows observation.

The broader [validation record](VALIDATION.md) and [release checklist](RELEASE.md) track the other open gates.

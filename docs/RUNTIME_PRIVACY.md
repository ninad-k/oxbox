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

## Embedded runtime behavior

Microsoft documents optional and required WebView2 diagnostics, with required collection outside the
application developer's overall control. Feature-specific APIs exist for crash reporting and reputation
checking; they are not a general zero-network guarantee. [WebView2 privacy documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy).

Microsoft describes browser flags as development/diagnostic mechanisms that should not ship in production.
An added `--disable-background-networking` flag is therefore not a supported promise that runtime traffic
has been eliminated. No OS security/privacy setting or browser flag was changed during this investigation.
[WebView2 browser-flag guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags).

## Acceptance status

The strict whole-process offline requirement has **not passed on Windows**. Keep the release a draft until
maintainers resolve or explicitly review this gap, identify the runtime behavior in an isolated test environment,
and agree on truthful privacy wording and supported controls. Do not infer zero runtime traffic from the absence
of a Rust HTTP client, the CSP, the currency opt-in state or successful unit tests. macOS/Linux runtime
traffic has not been measured by this Windows observation.

The broader [validation record](VALIDATION.md) and [release checklist](RELEASE.md) track the other open gates.

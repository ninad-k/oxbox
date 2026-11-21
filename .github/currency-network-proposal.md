## Proposal: opt-in ECB currency reference rates

The Phase 5 roadmap includes currency conversion with daily cached rates. The repository requires a public
issue before adding runtime network access. This issue proposes that narrowly scoped exception.

### Reviewable implementation

- Default remains offline. Rates can be imported from an ECB XML file without any network access.
- A separate explicit setting enables manual refresh from only
  `https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml`.
- No background telemetry, arbitrary URLs, API keys, accounts, redirects, or user content uploads.
- The provider receives the ordinary connection IP and HTTP request; amounts and selected currencies never leave the app.
- HTTPS certificate validation remains enabled. The system curl provider has a 20-second timeout, redirect prohibition,
  and a 1 MiB response ceiling; parsing rejects entities/DTDs and invalid/duplicate rates.
- A locally cached, dated rate table remains usable offline. The app shows the rate date and reference-only purpose.
- Refresh is avoided for a cache fetched less than 24 hours ago unless the user explicitly forces it.
- No new HTTP library is added. User data and passphrases are never included in the request.

### Acceptance

- Conversion and bounded XML validation tests pass; opt-out never invokes curl.
- UI explains the fixed provider before opt-in and offers local-file import.
- Failed refresh preserves the existing cache; stale rates remain visibly dated.
- The privacy/usage documentation names the exception. All other tools remain offline.

Primary provider documentation: https://www.ecb.europa.eu/stats/policy_and_exchange_rates/euro_reference_exchange_rates/html/index.en.html

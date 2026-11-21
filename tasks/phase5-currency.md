# Phase 5: Currency reference conversion

The user explicitly promoted the Phase 5+ roadmap on 2026-11-21. Implement a dated ECB reference-rate converter
with offline XML import and optional daily cache refresh. No amounts or selected currencies leave the app.

- Core: bounded strict ECB XML parser, finite positive rates, duplicate/entity rejection, conversion through EUR,
  freshness calculation, bounded system-curl provider with a fixed HTTPS URL and timeout.
- Shell: local-store cache and opt-in, typed API, local file picking, visible rate date/provider and offline status.
- Tests: known rates, invalid dates/rates/entities, missing currencies, numeric limits, opt-out never starts a provider.
- Network gate: the user authorized public issue creation and opt-in refresh. [Issue #1](https://github.com/ninad-k/oxbox/issues/1)
  now records the reviewed exception. Networking remains off by default and requires the separate local opt-in.
- No HTTP crate or arbitrary URL is added. A manual import is fully functional without networking.

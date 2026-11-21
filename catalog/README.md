# Shared tool catalogue

`tools.json` is the metadata authority for all 49 stable tool IDs and seven categories. The native Rust
launcher embeds it at compile time; the legacy Svelte registry imports it as build-time data; the static
website builder reads it directly. None fetches the catalogue over the network.

Tool metadata includes name, description, category, search keywords, supported platforms, source phase/status,
provider caveat (`note`) and the Lucide icon key. Category labels and icon keys are shared too. `ready` means
the source implementation exists; it does not certify installer, device, privacy, accessibility or platform
acceptance. Those records remain in `docs/VALIDATION.md` and `tasks/native-migration.md`.

`icons.json` contains the existing ISC-licensed Lucide vectors flattened for native drawing. Its keys match
the catalogue's `icon` strings. Regenerate it with `node scripts/generate-native-icons.mjs` after an approved
icon change; keep the exact existing dependency and attribution. No SVG parser or remote icon loader runs
inside the native app.

The legacy `src/lib/registry.ts` and `roadmap-registry.ts` contain typed component-import and lazy-view
adapters only. Their icon maps bind canonical icon keys to per-file Lucide imports; their view maps bind
stable IDs to Svelte components. Names, descriptions, platform lists and other catalogue metadata must not
be duplicated there. CSS category tones remain a legacy presentation detail. Missing ready-view adapters,
unknown icon keys and orphan view adapters fail visibly.

When adding or changing a tool:

1. Update the canonical JSON, keeping existing IDs stable for favorites and URLs.
2. Implement its core logic and native view; preserve existing platform limitations and safety behavior.
3. Maintain the legacy view/icon adapters while that implementation remains available for comparison.
4. Update the canonical provider note. Use `website/tool-notes.json` only for a deliberate website-specific
   wording override; it cannot change platform metadata or source status.
5. Run the relevant Rust checks, `pnpm check`, `pnpm test`, `node scripts/build-website.mjs`, and
   `node --test scripts/website.test.mjs`. Record native and website visual acceptance separately.

Registry tests compare the complete ordered metadata and use a changed catalogue fixture to prove the
legacy UI derives values and icons from JSON. Website tests compare generated data with the same catalogue
and preserve search/category/platform reset behavior. These checks do not replace native runtime testing.

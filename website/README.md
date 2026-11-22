# Oxbox website

Static GitHub Pages source. It loads no external fonts, scripts, analytics or CDN resources. Links to GitHub are ordinary visitor-initiated navigation.

From the repository root with Node.js 22.17 or newer (the website build has no package dependencies):

```sh
node scripts/build-website.mjs
node --test scripts/website.test.mjs
```

The generated `website/dist/` is ignored and deployable beneath `/oxbox/`. All resource URLs are relative. Serve that directory with a local static server for preview. The build validates local resource paths and fragment targets.

The build reads `catalog/tools.json`, the shared metadata source used by the native Rust launcher and legacy Svelte adapters. It never parses TypeScript or executes application imports. Tool IDs, names, descriptions, categories, platform restrictions, phase/status, keywords and icon bindings therefore follow the same catalogue and order. It validates unique IDs, categories, required fields, platform values and bindings in `catalog/icons.json` before rendering.

Catalogue notes are the default provider/validation caveats. `tool-notes.json` remains an explicit website-only note override; stale IDs and non-text overrides fail the build. Prefer editing the canonical note when the caveat applies to every interface. Generated `tools.json` retains the canonical metadata plus the resolved note. The complete catalogue is rendered at build time and remains readable without JavaScript; filtering is progressive enhancement. See [catalogue maintenance](../catalog/README.md) for adapters and consistency checks.

`.github/workflows/pages.yml` builds and uploads the generated directory, then deploys through GitHub Pages. A repository maintainer must select **GitHub Actions** as the Pages source and permit the selected branch in the `github-pages` deployment environment. The native source branch is `codex/native-renderer`; when development moves to `main`, update the source/documentation links and workflow branch policy together. The trigger also retains the preceding development branch for existing deployments.

For initial setup, a maintainer can inspect `GET /repos/ninad-k/oxbox/pages`, then create an absent site with `POST /repos/ninad-k/oxbox/pages` and `{"build_type":"workflow"}`. An existing site can use `PUT` with the same build type. The [Pages REST API](https://docs.github.com/en/rest/pages/pages#create-a-github-pages-site) requires permission to manage Pages; fine-grained tokens need Pages and Administration write access. Do not configure a legacy `/docs` source for this workflow-generated artifact.

An authorized push changing a watched path on `codex/native-renderer` builds and deploys that branch after its environment policy permits deployment. Manual dispatch can select it with `gh workflow run pages.yml --repo ninad-k/oxbox --ref codex/native-renderer`, but [GitHub requires the workflow to also exist on the default branch](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow) before dispatch is available. Enabling Pages does not merge source branches or publish the private installer draft.

The hero uses the actual native Windows dashboard image at `assets/native-dashboard-dark.jpg`, with four chosen favorites. It is a product screenshot, not an HTML illustration of the retained Tauri host. The favorites-only dashboard remains the app model; the screenshot does not imply that those four tools are selected for every new profile. The build copies the original outlined O logo directly from `static/oxbox.svg`, so the site and native app share one source asset. No new image/font/CDN dependency is required.

The workflow sections explain Batch Rename and guarded Undo, safe-option presets, screenshot review/flattened redaction, explicit pasted-text cleanup, Quick Actions and OS-scoped maintenance. Code samples are examples, not additional product screenshots or evidence that every workflow has passed native acceptance. The [maintenance guide](../docs/MAINTENANCE.md) records actual scopes, exclusions, backups/restore and irreversible-file limits. It does not claim a performance/health score or identical coverage to another product.

Counts and cards come entirely from the canonical catalogue. Tests compare its dynamic tool/category totals, metadata and order, and require every original ID in `catalog/v01-tool-ids.json` to survive catalogue growth. Do not replace those checks with a fixed total when adding tools.

The current app uses native eframe/egui 0.37 with Rust 1.95. The website describes that selected implementation and links its development branch. Legacy WebView2 privacy findings remain historical evidence; removing the embedded browser is not a substitute for new production observations. A live native launcher window has been observed, while full native tool and installer acceptance remains open in `tasks/native-migration.md`.

Public downloads are deliberately not offered while installers remain in a private draft. Release, privacy and platform claims must follow `docs/VALIDATION.md`, `docs/RUNTIME_PRIVACY.md` and `docs/RELEASE.md`. Publishing the website does not close installer, privacy, security or hardware acceptance gates.

Before deploying changes, run the build and JavaScript syntax checks, then check the page at desktop and 375px mobile widths, keyboard navigation, search/category/platform combinations, the reset/empty state, FAQ disclosure controls and reduced-motion behavior. Browser verification is recorded by the integrating maintainer; a successful static build alone is not a visual acceptance pass.

On 2026-11-21, integrating browser QA confirmed the then-current desktop hero and 375px mobile layout without horizontal overflow, keyboard use of the shortcut FAQ, and no JavaScript console errors. The exact `mouse` search → macOS → zero results → Reset sequence restored the then-current 49 tools; an unmatched search → Reset did the same. That historical result does not certify the later maintenance/workflow layout or expanded catalogue. Fresh desktop/mobile browser QA belongs to the integrating maintainer. These are website observations, not native app, installer or hardware acceptance. GitHub Pages has been configured for workflow publishing with the development branch allowed; public deployment remains a separate step after the reviewed source push.

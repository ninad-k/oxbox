# Oxbox website

Static GitHub Pages source. It loads no external fonts, scripts, analytics or CDN resources. Links to GitHub are ordinary visitor-initiated navigation.

From the repository root, after a frozen pnpm install:

```sh
node scripts/build-website.mjs
node --test scripts/website.test.mjs
```

The generated `website/dist/` is ignored and deployable beneath `/oxbox/`. All resource URLs are relative. Serve that directory with a local static server for preview. The build validates local resource paths and fragment targets.

The build uses the existing TypeScript development dependency to parse `src/lib/registry.ts` and `src/lib/roadmap-registry.ts`. It evaluates only a restricted set of static AST expressions, never imports application modules, and ignores icon/load properties. Tool names, descriptions, categories, platforms and counts therefore follow the launcher catalogue. A registry syntax change outside that subset fails visibly. Tool-specific provider caveats are maintained in `tool-notes.json`; stale note IDs fail the build. The complete catalogue is rendered at build time and remains readable without JavaScript; filtering is progressive enhancement.

`.github/workflows/pages.yml` builds and uploads the generated directory, then deploys through GitHub Pages. A repository maintainer must select **GitHub Actions** as the Pages source and permit the selected branch in the `github-pages` deployment environment. The source branch is currently `codex/complete-v0-1`; when development moves to `main`, update the source/documentation links and workflow branch policy together.

For initial setup, a maintainer can inspect `GET /repos/ninad-k/oxbox/pages`, then create an absent site with `POST /repos/ninad-k/oxbox/pages` and `{"build_type":"workflow"}`. An existing site can use `PUT` with the same build type. The [Pages REST API](https://docs.github.com/en/rest/pages/pages#create-a-github-pages-site) requires permission to manage Pages; fine-grained tokens need Pages and Administration write access. Do not configure a legacy `/docs` source for this workflow-generated artifact.

The initial push containing the website and workflow to `codex/complete-v0-1` automatically builds and deploys that branch. Manual dispatch can select an explicit branch with `gh workflow run pages.yml --repo ninad-k/oxbox --ref codex/complete-v0-1`, but [GitHub requires the workflow to also exist on the default branch](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow) before dispatch is available. Enabling Pages does not merge source branches or publish the private installer draft.

The hero is an original HTML/CSS illustration explicitly labeled as such. Replace or complement it with verified product screenshots when available; do not label the illustration a real capture. The build copies the app’s original outlined O logo directly from `static/oxbox.svg`, so the site and native app share one source asset. The Pages workflow watches that asset for changes. The launcher illustration keeps the native square window proportions and shows only pinned favorites in its dashboard.

Public downloads are deliberately not offered while installers remain in a private draft. Release, privacy and platform claims must follow `docs/VALIDATION.md`, `docs/RUNTIME_PRIVACY.md` and `docs/RELEASE.md`. Publishing the website does not close installer, privacy, security or hardware acceptance gates.

Before deploying changes, run the build and JavaScript syntax checks, then check the page at desktop and 375px mobile widths, keyboard navigation, search/category/platform combinations, the reset/empty state, FAQ disclosure controls and reduced-motion behavior. Browser verification is recorded by the integrating maintainer; a successful static build alone is not a visual acceptance pass.

On 2026-11-21, integrating browser QA confirmed the desktop hero and 375px mobile layout without horizontal overflow, keyboard use of the shortcut FAQ, and no JavaScript console errors. The exact `mouse` search → macOS → zero results → Reset sequence restored all 49 tools; an unmatched search → Reset did the same. These are website observations, not native app, installer or hardware acceptance. GitHub Pages has been configured for workflow publishing with the development branch allowed; public deployment remains a separate step after the reviewed source push.

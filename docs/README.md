# Oxbox documentation

Oxbox is a native Rust desktop toolbox with 56 implemented tool routes. Implementation, automated checks,
manual acceptance and release approval are separate states. Windows installers remain in an unpublished
draft; [release gates](RELEASE.md) identify what still needs verification.

## Start here

| I want to… | Read |
|---|---|
| Use the launcher, shortcuts and tools | [Usage](USAGE.md), [complete feature catalogue](FEATURES.md) |
| Make my first contribution | [Contributor guide](../CONTRIBUTING.md), [first contribution walkthrough](GETTING_STARTED.md) |
| Understand the code | [Architecture](ARCHITECTURE.md), [architecture decisions](adr/0004-runtime-offline-boundary.md), [agent/human working rules](../AGENTS.md) |
| Find a bounded task | [Native tool matrices](../tasks/native-migration.md), [validation gaps](VALIDATION.md), [task specifications](../tasks/todo.md) |
| Clean up or reuse a workflow | [Maintenance, recovery, presets and Quick Actions](MAINTENANCE.md) |
| Test safely | [Testing](TESTING.md), [troubleshooting](TROUBLESHOOTING.md) |
| Get help or report a problem | [Support](../SUPPORT.md), [private security reports](../SECURITY.md) |
| See the functional review and its limits | [56-tool source verification](verification/2026-11-22-56-tool-pass.md), [49-tool source review](FEATURE_REVIEW.md), [dated validation](VALIDATION.md) |
| Check the current expansion and installers | [Maintenance/workflow receipt](verification/2026-11-22-maintenance-workflows.md), [release checklist](RELEASE.md) |
| Change tool metadata or icons | [Catalogue maintenance](../catalog/README.md) |
| Work on the public website | [GitHub Pages source and deployment](../website/README.md) |
| Triage or review contributions | [Maintainer workflow](MAINTAINING.md), [code of conduct](../CODE_OF_CONDUCT.md) |

## Safety and release evidence

- [Security policy and private reports](../SECURITY.md), [security review](SECURITY_REVIEW.md), [dependency policy](DEPENDENCIES.md)
- [Runtime privacy observations](RUNTIME_PRIVACY.md), [privacy acceptance](PRIVACY_ACCEPTANCE.md)
- [Accessibility](ACCESSIBILITY.md), [installer acceptance](INSTALLER_ACCEPTANCE.md), [release checklist](RELEASE.md)
- [Promoted roadmap tools and optional providers](ROADMAP_TOOLS.md), [Mouse Activity](MOUSE_ACTIVITY.md)
- [Changelog](../CHANGELOG.md), [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE) licenses

Current code lives on `codex/native-renderer` while the native migration is under review. Select that branch
when reading GitHub files or opening a related pull request. Historical Tauri/Svelte instructions and results
remain for comparison; use `cargo run -p oxbox-native --locked` for the selected app.

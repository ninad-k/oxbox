# Getting help with Oxbox

Start with the [documentation hub](docs/README.md), [usage guide](docs/USAGE.md) and
[troubleshooting](docs/TROUBLESHOOTING.md). The current native source and guides are on
`codex/native-renderer` while migration review is open. The browser preview and native desktop have
different runtime capabilities; identify which one you are using.

If a problem remains, open an issue using the appropriate template:

- **Bug:** stable tool ID, source commit, OS/session, exact error and minimal generated reproduction.
- **Documentation:** guide/section and the incorrect command or missing explanation.
- **Validation:** candidate source/hash, procedure, expected/actual result and coverage limits.
- **Tool request:** useful behavior, platform requirements and relevant safety/provider/network constraints.

Use disposable fixtures. Redact usernames, personal paths, clipboard contents and recordings before posting
logs or screenshots. Report vulnerabilities privately through [SECURITY.md](SECURITY.md).

The native review branch includes new documentation and validation issue forms. GitHub activates forms from
the repository's default branch after merge; meanwhile, use the available bug/blank issue with the context
listed above. The private vulnerability-report channel is already enabled.

Windows installers remain in an unpublished draft; [release status](docs/RELEASE.md) lists the outstanding
manual, platform and installer checks. A source implementation or CI pass does not certify every device
and OS configuration. There is no guaranteed support response time for ordinary community questions.

To help investigate or submit a fix, follow [CONTRIBUTING.md](CONTRIBUTING.md) and the
[first-contribution walkthrough](docs/GETTING_STARTED.md). All participation follows the
[code of conduct](CODE_OF_CONDUCT.md).

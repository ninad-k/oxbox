# Native accessibility acceptance

This record separates source/tree checks from assistive-technology and real-device acceptance. It applies
to the eframe 0.37 native host. It is not a WCAG conformance statement or a screen-reader certification.

## 2026-11-22 source review

The review reproduced missing accessible names for the appearance selector, capture selectors, numeric
inputs, application-set fields and archive extraction limit. File-picker and repeated row buttons had
ambiguous names. Unit-conversion errors were ordinary text, and the shared control border contrast was
approximately 1.39:1 in the dark theme.

The native host now supplies explicit AccessKit names and label relationships, contextual names for
selected paths, alarm/countdown/clock/application/clipboard rows, and named alerts for user-action errors.
Long user-requested jobs expose polite working/completion status. Passive system/network/clipboard polling
uses quiet output so every sample does not recreate a live announcement. Clipboard rows retain stable
widget identity by entry, including when the collection changes. Password fields do not retain undo history.

Shared control boundaries use a minimum 3:1 contrast check against their normal light/dark fills. This uses
the [W3C non-text contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)
as a design check. Alert/status semantics follow the principle of exposing changes without taking focus,
described in [W3C status-message guidance](https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html).
These narrow checks do not certify every text/state, OS high-contrast mode, rendered pixel or assistive technology.

## Reproducible checks

Run exact Rust 1.95 with the locked dependencies:

```sh
cargo test -p oxbox-native --locked
cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
```

The AccessKit regressions inspect the actual generated trees for all 49 default native forms (13 utilities,
14 media and 22 system), plus selected expanded states and two-row fixtures. The system-form test closes
task admission before rendering; it neither starts OS-reading jobs nor performs OS effects. Media fixtures
do not open a device or observe the real clipboard. Password tests use generated sentinel text only.
Texture updates are consumed without a GPU. Passing these tests proves those tree properties, not native speech,
visual focus, every busy/error/result state or physical keyboard/IME behavior.

## Bounded Windows interaction

The preceding production executable `257b5b20f92724cc18dcf4e2167917d4d53385974f1521c87940ee1d72f9840c`
was inspected through Windows UI Automation in an isolated profile. It exposed named navigation, preserved
literal Japanese/accented text (`日本語 café`), focused search with Ctrl+K and exited with Ctrl+Q. This found
the unnamed Appearance combo box and the search field's description-only naming before the fixes.
Literal Unicode injection is not an IME-composition test. Final-build observations are recorded separately
in [VALIDATION.md](VALIDATION.md).

## Remaining manual matrix

- Test Narrator and another supported screen reader: navigation, names, values, focus, dialog return,
  error/completion announcements and absence of repeated passive announcements.
- Complete keyboard-only traversal and operation of every expanded form and error/result state, including
  the previously inconclusive negative Ctrl+Shift+Q exact-modifier check.
- Test real IME composition/candidate selection, bidirectional/combining text, keyboard layouts and shortcuts.
- Check 100/150/200 percent scaling, monitor changes, magnification, OS text sizing and high contrast.
- Verify native dialogs, reduced motion, remote/GPU sessions and macOS/Linux accessibility adapters.

Keep these gates open until the actual tester, OS/session, executable hash and result are recorded. Do not
infer them from AccessKit availability or headless rendering. See [the release gates](RELEASE.md).

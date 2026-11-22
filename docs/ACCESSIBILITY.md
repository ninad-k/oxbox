# Native accessibility acceptance

This record separates source/tree checks from assistive-technology and real-device acceptance. It applies
to the eframe 0.37 native host. It is not a WCAG conformance statement or a screen-reader certification.

Maintenance source `c30a179` adds seven named native forms (56 catalogue routes total), guarded review/
recovery branches, numeric annotation controls alongside drag input, and neutral polite cancellation
status. Windows native tests **112/112** and macOS/Linux **114/114** passed in CI. Current-binary Windows
checks used Ctrl+K/Enter, accessible value edits, real file pickers and visible form/preview/export states.
The tool's modal focus-reporting/geometry quirks required fresh screenshots and keyboard file-name entry;
they are automation observations, not screen-reader passes. Actual assistive technology, full keyboard-only,
IME/Unicode labels, high contrast and DPI/monitor acceptance stay open. See the
[current scoped record](verification/2026-11-22-maintenance-workflows.md).

Source `67f0715` adds actual Windows checks for all **49 default native forms** on its exact production
EXE: Ctrl+K search, accessible field edits and tile activation, matching headings, Back controls and named
operations. Timer custom-duration preview and QR stale-preview/export-control invalidation were exercised
with generated values. Normal Ctrl+Q, explicit Cancel and hidden-job restoration also passed. This is UI
Automation and selected visible-rendering evidence, not complete keyboard-only or screen-reader testing.
An unavailable-window shortcut attempt and an out-of-bounds automation click remain inconclusive;
resize/DPI and exact-keyboard manual acceptance stay open. See the
[dated verification record](verification/2026-11-22-native-feature-pass.md).

Source `a7c91bd` also has a bounded Windows native UI observation on the exact release EXE: named launcher
search, Ctrl+K / Enter navigation, IANA field/combo values, world-clock actions, EAN-13 inputs and local image
preview, and normal Quit. Its durable original-file cleanup status has a real-poll/headless regression and
polite completion semantics. [Exact artifact evidence](VALIDATION.md#community-review-candidate-a7c91bd)
does not close speech, full keyboard traversal, IME, DPI or the other manual cases below.

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

## 2026-11-22 expanded-state follow-up

The next bounded review reproduced pointer-only screenshot area selection, accessibility value edits
changing disabled file/numeric drafts, ambiguous barcode Copy and resize Remove actions, changing resize
row identity after a preceding removal, missing combo expansion metadata and unhandled screen-reader
Expand/Collapse requests. The tests failed before their corresponding fixes. The integrated native suite
then passed **146/146 tests before the Chrono-to-Jiff time migration**. The
[dependency/accessibility receipt](verification/2026-11-22-dependency-accessibility.md) tracks the subsequent
candidate separately; this count does not certify the changed dependencies or a new production executable.

After the approved dependency/time port, the Windows full-workspace run passed **148 native tests**, plus
294 core tests, five frozen time-compatibility comparisons and 12 retained legacy-host tests, with none
failed or ignored. The two additional native tests check preserved date/fetched-time formatting. This is
current-source headless evidence; production-binary speech and keyboard observations remain pending.

### Follow-up on popup focus and Narrator

The exact intermediate `35d6afc` Windows EXE (`cac752c108b9c66cbb7b23159117eeb510303718e112251aeb3e2f6092cbb365`)
accepted Ctrl+K search/Enter, forward/reverse Tab and keyboard dropdown opening. Escape correctly closed
the dropdown without leaving Unit Converter, but focus fell to the window. A new headless regression
reproduced the lost focus. The shared adapter now locks Escape focus only while an open combo already
owns focus; it never requests/restores focus after outside clicks. Both the original keyboard sequence
and negative cases for another focused field and a pointer dismissal pass. The native suite now passes
**150 tests**, with strict workspace Clippy. Final-binary verification must follow the new source build.

The intermediate executable also preserved Value focus while exposing a named finite-number Alert and
removing Copy on generated invalid input. Narrator was launched with the user's consent; the user
confirmed audible speech and offered to report announcements. Exact spoken names, values and state/error
announcements have not yet been reported, so speech acceptance stays open. The helper reports that
Narrator's control window has higher Windows integrity; speech-recap inspection was not established.
No security/privacy setting or integrity boundary was changed. User assistance is required for that
reader session. Ctrl+Shift+Q left Oxbox running; the Ctrl+Q attempt with Narrator present was inconclusive.
The explicit Quit Oxbox button stopped the isolated app. These observations do not establish complete
keyboard traversal or a screen-reader pass. The [receipt](verification/2026-11-22-dependency-accessibility.md)
retains the exact intermediate scope and outstanding final-binary checks.

All **56 default native routes** are checked for named interactive controls and actual combo expansion
metadata. Shared combo/disclosure helpers report real collapsed/expanded state and handle enabled,
idempotent Expand/Collapse requests while preserving normal click and keyboard behavior. Screenshot Area
mode now offers a named numeric alternative: choose a display and enter physical-pixel X, Y, width and
height before explicit capture. The form explains display-relative coordinates and bounds; headless edits
to all four controls are tested without capturing a screen. Pointer selection remains available.

The regression-covered utility/maintenance text and numeric inputs retain the job's admitted values when
disabled accessibility actions arrive. The timer's draft likewise stays unchanged while running or paused. Resize Remove
actions identify their selected image and retain stable row identity; decoded-barcode Copy actions identify
the format and result number. These are generated-data/tree and fake-worker checks, not spoken-output or
physical-keyboard acceptance. No test captures private content, reads the system clipboard, deletes user
files or changes an OS setting.

The final-binary build and real Narrator session for this follow-up have **not yet been recorded**. The user
offered to help with speech observations; no speech result is inferred from that offer or from the tests.

### Generated-data Narrator contribution checklist

Use the exact candidate executable with a fresh disposable profile and generated local fixtures. Record
the source revision, executable SHA-256, Windows/Narrator versions, keyboard layout and each actual result.
Do not include private filenames, process details or remembered clipboard content in a public report.
These are instructions for a future human session, not completed checks.

- Use Ctrl+K, type a tool name, activate the result with Enter and return with Back. Check spoken names,
  visible focus and Tab/Shift+Tab order; record any unexpected focus movement.
- Open Settings and focus Appearance. Open the chooser, change a value, then reopen and dismiss it with
  Escape. Check the spoken collapsed/expanded state and that Escape returns to the closed chooser.
  Also try Narrator's Expand/Collapse actions where available; repeating an action should not toggle it.
- In Unit Converter, enter generated invalid text such as `not a number`. Check that the error is announced
  without stealing focus, then enter a valid number and verify recovery.
- In Verify Checksum, select a generated local file and start a read-only hash. Use a fixture large enough
  to observe Working on the test machine. Check the polite completion announcement and accessible result;
  do not substitute a personal file or interpret a fast job's inaudible Working state as a passed test.
- Leave a passive System Monitor or Network Usage view open briefly. Check that unchanged or periodic
  samples do not repeatedly interrupt speech. Do not start clipboard history for this check.
- Open a file picker from Verify Checksum, cancel once, then reopen and select the generated fixture.
  Check dialog labels, keyboard navigation, focus on return and the retained exact selection.

Report partial, failed and inconclusive observations explicitly. Even a successful checklist leaves the
complete keyboard-only matrix, another screen reader, real IME composition, DPI/high contrast, monitor
changes and macOS/Linux runtime accessibility pending.

## Reproducible checks

Run exact Rust 1.95 with the locked dependencies:

```sh
cargo test -p oxbox-native --locked
cargo clippy -p oxbox-core -p oxbox-native --all-targets --locked -- -D warnings
```

The AccessKit regressions inspect the actual generated trees for all 56 default native forms (13 utilities,
14 media, 22 system and seven maintenance), plus selected expanded states and two-row fixtures. The system-form test closes
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

The final `d8d1d1e` executable (SHA-256 `af363cd668e412f111bdfb37de39738ff35089e993585b19725932ba2de74b26`)
confirmed named search/chooser controls, keyboard search/Enter navigation, a Windows Status node reading
“Checked 2 local providers”, Working status during a controlled job, and Ctrl+Q shutdown. The preceding
`54dd593` candidate additionally confirmed the repaired Appearance name, Unicode retention, conversion and
an Alert for invalid input without moving focus. These are Windows UI Automation observations, not speech tests.

## Remaining manual matrix

The [56-tool follow-up](verification/2026-11-22-56-tool-pass.md) passed **135 native tests on Windows**
and **137 each on macOS ARM/Linux**, including all default-form names and selected expanded states.
The exact `a9b6e1c` Windows EXE additionally exposed the checksum comparison and a named local-image
sample error through UI Automation; coordinate edits removed stale Copy controls, and a valid sample
recovered. Native picker selection preserved a generated Japanese filename. These bounded generated
interactions do not close the speech, complete keyboard, IME, DPI or cross-OS runtime gates below.

- Test Narrator and another supported screen reader: navigation, names, values, focus, dialog return,
  error/completion announcements and absence of repeated passive announcements.
- Complete keyboard-only traversal and operation of every expanded form and error/result state, including
  the previously inconclusive negative Ctrl+Shift+Q exact-modifier check.
- Test real IME composition/candidate selection, bidirectional/combining text, keyboard layouts and shortcuts.
- Check 100/150/200 percent scaling, monitor changes, magnification, OS text sizing and high contrast.
- Verify native dialogs, reduced motion, remote/GPU sessions and macOS/Linux accessibility adapters.

Keep these gates open until the actual tester, OS/session, executable hash and result are recorded. Do not
infer them from AccessKit availability or headless rendering. See [the release gates](RELEASE.md).

# Using Oxbox

Run `cargo run -p oxbox-native --locked` for the selected eframe/egui desktop app. Use `-- --profile <directory>`
for a disposable fixture profile. The retained `pnpm tauri dev` host and `pnpm dev` browser preview are legacy
comparison tools, not the native entrypoint.
Open Oxbox from its tray/menu-bar icon or the default **Ctrl+Alt+O** shortcut (**Cmd+Option+O** on macOS).
The normal, resizable native window starts at 800 × 800 logical pixels. **Dashboard shows only your favorites**;
use All tools or a category to browse the 56-tool catalog. The original outlined O/ember mark is shared with the website.

The [maintenance and workflow guide](MAINTENANCE.md) covers the seven new tools, per-OS cleanup rules,
startup/registry recovery, file-shred limits, screenshot annotations, pasted-text cleanup, safe saved
presets and search Quick Actions. Its safety limits apply before any corresponding change is confirmed.
Path text fields preserve spaces in a selected filename. Pickers refuse names that cannot be represented
exactly as Unicode text or contain line breaks; the previous selection remains visible with an error.
Choose another supported name rather than assuming that a refused selection replaced the previous one.
Closing the launcher hides it; **Quit Oxbox** in the sidebar/tray or **Ctrl/Cmd+Q** inside the app exits and stops background work.

Search with **Ctrl/Cmd+K** or **/**, press **Enter** to open the first result, and **Escape** to return.
The star in a tool's title adds it to Favorites. **Alt+1–9** opens the corresponding favorite when you are not
editing text (or while the launcher search is empty). Settings changes and favorite tools are stored locally.
The global shortcut is configurable; an existing saved binding is retained rather than replaced by the new default.
The promoted Phase 5 tools and their provider/platform requirements are described in [the expanded guide](ROADMAP_TOOLS.md).
Native tool operations work locally by default; currency alone offers a separate explicit manual ECB-refresh opt-in.
The selected native host contains no embedded browser. Historical WebView2 observations remain in
[runtime privacy findings](RUNTIME_PRIVACY.md); new native production privacy and release gates remain open.

## Startup and tray

On the first launch of a fresh installed Windows or macOS copy, Oxbox enables **Launch at login** for the
current user. Logins use `--autostart`: the app opens quietly in the system tray/menu bar, and the global
launcher shortcut opens its normal window. If the tray is unavailable, Oxbox stays visible with an error.
Closing the window keeps the tray running; Quit exits and stops owned work.

Turn login startup off in Settings. Existing native/legacy profiles, saved choices and Windows startup
records (including OS-disabled choices) are preserved on upgrade instead of being silently re-enabled.
Development builds and explicit `--profile` QA runs cannot change login registration. Portable Linux archives
leave startup off; a release build can be explicitly enabled from Settings. Startup never automatically starts
Mouse Activity, clipboard history, camera/microphone capture or recording.

The installer marker only identifies packaged copies; it does not grant permissions or bypass OS policy.
Windows commands quote the executable path, including spaces. Disabling affects only Oxbox's current-user
Run value. Uninstall cleanup must remove only the exact owned startup command; clean-machine verification
remains in the release checklist. Preferences are retained separately from program uninstall.

## Settings

The Settings control and tray menu open Settings. Choose System, Light, or Dark appearance; toggle launch at login;
or register a launcher shortcut. If another application owns the shortcut, Oxbox reports an error and retains
the preceding binding. The native host saves separate native-*.json files under the OS application-data directory, or the explicit
profile directory. Native shell preferences, system/app sets, utility reminders/world clocks and currency cache
have separate stores. Existing legacy settings/cache files are imported once and preserved byte-for-byte.
Currency networking starts off on first native import, even if the legacy preference was enabled. Legacy
webview-local favorites must first exist in the legacy saved preferences; the native host does not open a browser
to read webview storage. Invalid stores produce an error instead of being overwritten.

## Tools

| Tool | How to use it | Behavior and limits |
|---|---|---|
| Transform Text | Paste text and choose a case | Original text stays in the input; copy the result |
| Unit Converter | Pick a category, units, and value; Swap reverses units | Eight measurement categories; the separate Currency mode uses the same dated reference-rate tool described in the expanded guide, with local import and optional explicit ECB refresh |
| Timer & Stopwatch | Choose a countdown preset/custom duration, or start the stopwatch and record laps | Space starts/pauses and R resets when not typing; clocks survive navigation and a hidden launcher; notifications require OS permission |
| World Time | Search an IANA zone, add a clock, use Up/Down/Remove | Bundled rules handle DST; offsets are shown from UTC and local time |
| QR & Barcode | Enter text or digits, choose a code, save or copy the preview | EAN-13 requires all 13 digits and a valid check digit; Code 128 accepts printable ASCII; outputs never replace existing files |
| Verify Checksum | Choose/drop a file, select an algorithm, optionally paste an expected hash | Reads in chunks, reports progress, supports cancellation; SHA-1 and MD5 are for compatibility, not modern authenticity checks |
| Find Duplicates | Pick folders, scan, select copies or keep oldest/newest | Ignores empty files and symlinks; compares BLAKE3 after size/prefix grouping; review exact paths before confirming Trash |
| Archive & Unarchive | Choose inputs and a new archive filename, or an archive and a new destination folder | Creates ZIP/tar.gz; extracts ZIP/tar/tar.gz/7z; rejects links/traversal; failed extraction can leave a clearly separate partial folder |
| Clean Drive | Scan known cache locations, select categories, review totals, confirm | Accepts category IDs, never arbitrary cleanup roots; skips redirected paths; old temporary files only; contents go to Trash |
| System Monitor | Open the tool to start sampling | CPU, per-core CPU, RAM/swap, disks, top processes, and a 60-sample CPU trace; sampling stops on leaving the view |
| Network Usage | Open the tool, optionally include virtual interfaces | Reads local OS interface counters; measures rates and session deltas; does not contact network services |
| Mouse Activity | On Windows, choose idle delay/interval and optional duration, then explicitly Start | Starts stopped; pointer-only movements while idle, pauses when input resumes; Stop from tool/banner/tray; unsupported on macOS/Linux; [full behavior and limits](MOUSE_ACTIVITY.md) |
| Do Not Sleep | Choose system/display behavior and optional duration/end time | Managed awake lock is released on stop, expiry, and Quit; tray toggle and tooltip reflect state |
| Lock Screen | Select Lock Screen | Locks immediately on Windows/Linux; macOS display sleep requires the OS password-on-wake setting |
| Sleep Timer | Choose action and countdown, review, then confirm arming | Persistent banner and tray cancellation; warning notification within the final minute; Quit cancels pending actions |
| Break Time | Set an active-work interval and enable reminders | Snooze five minutes or skip; eye-rest mode; persisted settings; best-effort idle/fullscreen detection on Windows |
| Screenshot | Choose a display/window or Area, optionally delay, capture | New PNG in Pictures/Oxbox or chosen folder; tries clipboard copy; area selection freezes each monitor, drag a rectangle or Escape |
| Clipboard History | Read the privacy note, then Start recording | Starts paused; text/images stay in memory; search, pin, copy, delete, clear, pause; 200 items and 50 MiB including pins |
| Resize Images | Choose images/output folder, percentage/fit/exact dimensions, output format | Fit preserves aspect; Exact uses the supplied dimensions; EXIF orientation applied; JPEG quality control; new output names, progress/cancel |
| Hide Desktop | Hide/restore icons, or use the tray toggle | Captures and restores the preceding visibility setting on Quit; unavailable on Linux |
| Encrypt Files | Choose source and new output, enter/confirm a passphrase, encrypt or decrypt | age scrypt format; streams large files; wrong passphrase fails; originals kept; passphrases never intentionally stored/logged |

## Native lifecycle

Timers, alarms, date notifications, break reminders and explicitly enabled background controllers continue
while the launcher is hidden. Hiding clears unsubmitted passphrase drafts but preserves in-flight file work.
Navigating away from a file/conversion view cancels and abandons its stale UI job; user Cancel remains visible
in the view. An explicitly requested currency import/refresh can finish and persist its cache after navigation.
Stop/expiry/Quit clean up owned workers and leases. Treat current native behavior as awaiting the recorded
[per-tool manual checks](../tasks/native-migration.md), not as certified by historical Tauri screenshots.

## File safety

Use a dedicated test folder while learning the file tools. Trash can fail when the OS denies access or the target
volume does not support it; Oxbox reports the error. A multi-file operation may complete some files before an
error or cancellation, so always review the resulting folder. Cancellation stops future work at safe checkpoints.
It cannot undo an already completed OS operation.

Archive extraction requires a new directory and an output budget. The smaller of the chosen budget and 20 times
the compressed archive size applies. Highly compressible legitimate archives may exceed this conservative guard.
7z metadata is limited to 128 KiB both before and after decompression. Compression blocks are limited to four
coders and a combined 128 MiB decoder-model estimate, with one decoding thread. Large file lists, large
dictionaries and unusual compression methods may therefore be refused even when an archive is valid. These
limits are not a ceiling on total process memory. Archive passwords and archive creation in 7z format are not supported.

Clipboard history can contain sensitive information copied from other apps. Recording requires an explicit start
and does not survive Quit. On Windows, content carrying supported exclusion/concealment markers is ignored;
other platforms do not expose equivalent protection through the current backend. Pause before copying secrets.

## Mouse Activity sessions

Mouse Activity does nothing until you explicitly start a session. It waits a full idle delay after Start or
new input, then alternates bounded pointer-only movements at your interval. It never clicks, types or scrolls;
held input and unavailable desktop access prevent movement. It does not restore an old cursor position.
Sessions can continue while the launcher is hidden. Stop, optional duration expiry, a native error or Quit ends
the session and releases its input observers. Active state is never restored on restart.

Input timing stays in memory; keys, text and pointer trails are not recorded. Detection is subject to Windows
session permissions, and uncertain accessibility/remote-control input can conservatively restart waiting.
This is not a guarantee about sleep, locking or an application's presence policy. Native pointer checks require
a consenting Windows test session and remain separate from fake-backend tests; see [Mouse Activity](MOUSE_ACTIVITY.md).

## Website and release status

The [GitHub Pages website](../website/README.md) provides a searchable catalog and platform/provider notes.
It is an informational site, not a browser version of the native tools. Windows installers remain in a private
draft, and the new interface, hardware, clean-machine installer and strict runtime privacy acceptance gates
remain open. Read [validation](VALIDATION.md) and [release status](RELEASE.md) before inferring support from a tool tile.

## Platform notes and troubleshooting

- **Windows:** the MSVC build tools are required for development; the selected native host does not need WebView2. A long `target` path can cause
  `LNK1104`; use a short checkout or set `CARGO_TARGET_DIR` to a short path before building.
- **macOS:** screen recording permission may be needed for capture. Grant permissions yourself in System Settings.
  Lock Screen sleeps the display; set the OS to require a password immediately for actual locking.
  Area selection uses an opaque frozen screen image, with logical display coordinates converted using the
  window's backing scale for Retina displays. It does not require private macOS window APIs.
- **Linux:** capture, clipboard ownership, tray support, and keep-awake behavior depend on X11/Wayland and the
  desktop environment. Hide Desktop is marked unavailable. Install the native build packages listed in CONTRIBUTING.
- **No notification:** check OS notification permissions and focus modes. A notification failure does not change
  the timer's elapsed-time calculation.
- **Cannot save:** select a new filename and a writable folder. Existing files are deliberately refused.
- **Cannot capture:** retry an available monitor rather than a protected/minimized window. DRM surfaces and
  restricted Wayland sessions may deny capture.

Report reproducible problems using the bug template. Include tool ID, OS/session type, and steps with disposable
fixtures. Do not upload passphrases, personal clipboard contents, or private files to an issue.

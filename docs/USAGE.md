# Using Oxbox

Run `pnpm tauri dev` for the desktop app. `pnpm dev` is a UI preview: it intentionally cannot run Rust commands.
Open Oxbox from its tray/menu-bar icon or the default **Ctrl+Alt+Space** shortcut (**Cmd+Option+Space** on macOS).
Closing the launcher hides it; **Quit Oxbox** in the tray exits the app and stops its background work.

Search with **Ctrl/Cmd+K** or **/**, press **Enter** to open the first result, and **Escape** to return.
The star in a tool's title adds it to Favorites. Settings changes and favorite tools are stored locally.
The promoted Phase 5 tools and their provider/platform requirements are described in [the expanded guide](ROADMAP_TOOLS.md).
Tool commands work locally by default; currency offers a separate explicit ECB-refresh opt-in. Windows'
embedded WebView2 runtime may contact Microsoft independently; see [runtime privacy findings](RUNTIME_PRIVACY.md).

## Settings

The gear button and tray menu open Settings. Choose System, Light, or Dark appearance; toggle launch at login;
or register a launcher shortcut. If another application owns the shortcut, Oxbox reports an error and retains
the preceding binding. Settings are saved under the OS application-data directory in `settings.json`.
Old `oxbox.favorites` webview storage is migrated after the new store has been saved successfully.

## Tools

| Tool | How to use it | Behavior and limits |
|---|---|---|
| Transform Text | Paste text and choose a case | Original text stays in the input; copy the result |
| Unit Converter | Pick a category, units, and value; Swap reverses units | Length, mass, temperature, data size, time, speed, area, volume; no currency or network requests |
| Timer & Stopwatch | Choose a countdown preset/custom duration, or start the stopwatch and record laps | Space starts/pauses and R resets when not typing; clocks survive navigation and a hidden launcher; notifications require OS permission |
| World Time | Search an IANA zone, add a clock, use Up/Down/Remove | Bundled rules handle DST; offsets are shown from UTC and local time |
| QR & Barcode | Enter text or digits, choose a code, save or copy the preview | EAN-13 requires all 13 digits and a valid check digit; Code 128 accepts printable ASCII; outputs never replace existing files |
| Verify Checksum | Choose/drop a file, select an algorithm, optionally paste an expected hash | Reads in chunks, reports progress, supports cancellation; SHA-1 and MD5 are for compatibility, not modern authenticity checks |
| Find Duplicates | Pick folders, scan, select copies or keep oldest/newest | Ignores empty files and symlinks; compares BLAKE3 after size/prefix grouping; review exact paths before confirming Trash |
| Archive & Unarchive | Choose inputs and a new archive filename, or an archive and a new destination folder | Creates ZIP/tar.gz; extracts ZIP/tar/tar.gz/7z; rejects links/traversal; failed extraction can leave a clearly separate partial folder |
| Clean Drive | Scan known cache locations, select categories, review totals, confirm | Accepts category IDs, never arbitrary cleanup roots; skips redirected paths; old temporary files only; contents go to Trash |
| System Monitor | Open the tool to start sampling | CPU, per-core CPU, RAM/swap, disks, top processes, and a 60-sample CPU trace; sampling stops on leaving the view |
| Network Usage | Open the tool, optionally include virtual interfaces | Reads local OS interface counters; measures rates and session deltas; does not contact network services |
| Do Not Sleep | Choose system/display behavior and optional duration/end time | Managed awake lock is released on stop, expiry, and Quit; tray toggle and tooltip reflect state |
| Lock Screen | Select Lock Screen | Locks immediately on Windows/Linux; macOS display sleep requires the OS password-on-wake setting |
| Sleep Timer | Choose action and countdown, review, then confirm arming | Persistent banner and tray cancellation; warning notification within the final minute; Quit cancels pending actions |
| Break Time | Set an active-work interval and enable reminders | Snooze five minutes or skip; eye-rest mode; persisted settings; best-effort idle/fullscreen detection on Windows |
| Screenshot | Choose a display/window or Area, optionally delay, capture | New PNG in Pictures/Oxbox or chosen folder; tries clipboard copy; area selection freezes each monitor, drag a rectangle or Escape |
| Clipboard History | Read the privacy note, then Start recording | Starts paused; text/images stay in memory; search, pin, copy, delete, clear, pause; 200 items and 50 MiB including pins |
| Resize Images | Choose images/output folder, percentage/fit/exact dimensions, output format | Fit preserves aspect; Exact uses the supplied dimensions; EXIF orientation applied; JPEG quality control; new output names, progress/cancel |
| Hide Desktop | Hide/restore icons, or use the tray toggle | Captures and restores the preceding visibility setting on Quit; unavailable on Linux |
| Encrypt Files | Choose source and new output, enter/confirm a passphrase, encrypt or decrypt | age scrypt format; streams large files; wrong passphrase fails; originals kept; passphrases never intentionally stored/logged |

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

## Platform notes and troubleshooting

- **Windows:** WebView2 and the MSVC build tools are required for development. A long `target` path can cause
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

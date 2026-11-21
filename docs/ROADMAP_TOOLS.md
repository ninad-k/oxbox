# Expanded tool guide

The user promoted the Phase 5+ roadmap into implementation on 2026-11-21. The launcher now has the original
20 tools plus 28 Phase 5 entries and the separately authorized Windows-only Mouse Activity tool: **49 tools**.
Source availability is distinct from platform/manual release acceptance.
Download Video/Audio and Free Memory remain explicitly excluded by the plan.

## Windows, system and application tools

| Tool | Use and safety |
|---|---|
| Window on Top | Choose a native window and pin/unpin it. Window and process IDs are checked together; original owned state is restored on Quit. |
| Window Manager | Choose half/third/quarter/full placement. Geometry handles negative monitor origins and odd work-area widths. |
| Show Desktop | Minimize normal application windows; restore only the windows owned by that action. |
| Close Apps | Select windows, review and confirm a close request. Applications can ask about unsaved documents; no force-kill. |
| Focus on Window | Choose a window and dimming amount. Click-through overlays track its bounds; stop from the tool. No recording is involved. |
| Switch Resolution | Test an enumerated mode after confirmation. A native monotonic deadline rolls back independently of the view; confirm Keep after checking the display. |
| Eject Volumes | Review a removable volume and confirm safe eject. Busy volumes fail; there is no force-eject path. |
| Energy Saver | Choose an available Windows power profile and confirm the actual OS plan change. |
| Hidden Files | Inspect/toggle file-manager hidden-file visibility after confirmation. Protected system-file visibility is not enabled. |
| CPU Temperature | Read OS-exposed sensors. Some hardware exposes none; no driver/admin service is installed. |
| Launch App Sets | Save named local application/file groups with literal argument arrays. Review before launch. No shell strings or URL targets. |
| Uninstall Apps | Inspect registered Windows uninstall entries, review the command and confirm. The registry is rechecked before launch. Separately inspect exact-name AppData leftovers and confirm Trash; no silent removal. |
| Mouse Activity | Windows-only explicit Start; wait for idle and move the pointer gently at a chosen interval, yielding when input resumes. Optional duration and tool/banner/tray Stop; always stopped on launch. No clicks, typing or scrolling. See [full limits and pending acceptance](MOUSE_ACTIVITY.md). |

Window effects, display modes, eject, power profiles and uninstall inspection primarily target Windows. Other
platforms are marked unavailable where an adapter is absent. Temperature and local app/file sets are portable;
sensor availability depends on the OS. The tool catalogue and capability messages are authoritative.

## Presentation and time

| Tool | Use and safety |
|---|---|
| Presentation Mode | Independent system/display awake request and optional icon hiding. Stop restores owned state. Additional-display auto-start is a separate opt-in; it does not infer projector intent. |
| Do Not Disturb | Open documented Windows/macOS notification controls. State is manual/unknown; no private Focus API or unsupported registry toggle. |
| Airplane Mode | Open Windows radio controls. The user and OS decide which hardware can be disabled; Oxbox does not silently disconnect radios. |
| Alarm | Label, local date/time, IANA zone, recurrence and DST-fold choice. Native notifications run when hidden; snooze/dismiss/enable controls and confirmed removal. |
| Date Countdown | Track dated events with signed remaining time and optional once-only notification; confirm removal. |

Linux presentation supports awake behavior with icon hiding off. Notifications honor OS permissions and DND;
alarms cannot wake a powered-off computer. Saved alarm/countdown records survive restart.

## Media and vision

| Tool | Provider and behavior |
|---|---|
| Mute Microphone | Native Windows default input endpoint or supported local OS audio controls. Shows actual state or a clear unavailable/error message. |
| Record Audio | Local FFmpeg and explicit input/duration/output. Confirm before Start; Stop finalizes WAV. |
| Record Screen | Local FFmpeg OS/session backend. Choose screen/area/window; confirm before Start. Global banner/tray Stop survives navigation. |
| Make GIF | Local FFmpeg with palette generation and bounded segment, FPS and width. |
| Convert Audio / Video | Local FFmpeg presets for MP4, WebM, MP3, WAV and FLAC. File inputs only; restricted protocols/demuxers block remote playlists. |
| Take Photo | Local FFmpeg and selected camera; confirmed one-frame capture to a new filename. |
| Take Video | Local FFmpeg and selected camera, visible duration/output and recording controls. |
| Recognize Text | Windows WinRT OCR with installed OS languages, or user-installed Tesseract. Local images only; no upload. |
| Barcode Reader | Built-in rxing/rqrr decoders for supported QR/1D formats in local images; no camera/network. |
| Measure Color | Sample a display pixel or delayed cursor position as HEX, RGB, HSL and CMYK. No screenshot file is saved. |

Install maintained providers yourself and make their executables available in PATH. Oxbox reports availability
and never downloads codecs, language packs, camera drivers, or executables. FFmpeg's license depends on its build;
follow that installation's notices. Oxbox does not redistribute it. Primary references:
[FFmpeg](https://ffmpeg.org/documentation.html), [Tesseract](https://tesseract-ocr.github.io/),
[Windows OCR](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine).

Device names/indexes vary: Windows uses DirectShow/GDI, macOS AVFoundation, and Linux depends on its session.
Restricted Wayland capture, missing codecs, unavailable devices and denied permissions produce explicit errors.
No fallback starts a different device. Only one recording runs at a time; global controls remain visible across
navigation. Quit stops/reaps the child. A failed recording can retain a disclosed partial staging file for recovery.

## Currency reference rates

Currency appears as its own tool and inside Unit Converter. Import daily ECB XML to work entirely offline.
The dated cache remains usable after refresh failure; calculations stay local. Its isolated `currency.json`
store uses serialized access and restores the preceding in-memory value if persistence fails.

The only intentional tool-command network exception is [issue #1](https://github.com/ninad-k/oxbox/issues/1), posted with explicit
user approval. Network access is off by default. A separate checkbox allows manual HTTPS refresh from the fixed
ECB daily XML endpoint using system curl. The provider sees the ordinary connection/IP; amounts and currency
selections never leave Oxbox. No arbitrary URLs, redirects, curl configuration, credentials, uploads or telemetry.
Timeout, response-size and XML validation limits apply. Rates are informational reference values, with their date
shown; see the [ECB provider](https://www.ecb.europa.eu/stats/policy_and_exchange_rates/euro_reference_exchange_rates/html/index.en.html).

## Contributor checks

Promoted scope and acceptance: [system](../tasks/phase5-system.md), [media](../tasks/phase5-media.md),
[time](../tasks/phase5-time.md), [currency](../tasks/phase5-currency.md), [Mouse Activity](../tasks/mouse-activity.md).
Observed results: [VALIDATION.md](VALIDATION.md).
Optional provider checks are explicit and never run in the ordinary test suite:

```bash
cargo run -p oxbox-core --example provider_smoke -- --media
cargo run -p oxbox-core --example provider_smoke -- --currency-network
```

The media check generates synthetic one-second audio/video, converts WebM/GIF and verifies overwrite refusal.
It opens no physical screen, camera or microphone and retains a temporary fixture folder for inspection.
The currency check deliberately makes one fixed ECB request without changing application preferences.

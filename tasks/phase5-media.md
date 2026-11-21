# Phase 5: media and vision tools

These tasks promote the media/vision rows from the future roadmap into concrete source implementations.
The implementation is available; device/provider and platform acceptance remains separate from source completion.
No media provider binary is committed, downloaded, copied, or bundled by Oxbox.

| Tool ID | Implementation and provider | Acceptance before release |
|---|---|---|
| `mute-microphone` | Native Windows default communications endpoint; Linux `pactl` default source | Check mute/unmute and external changes using a consenting test microphone; unavailable on macOS |
| `record-audio` | Local FFmpeg input to PCM WAV; Windows DirectShow, macOS AVFoundation, Linux PulseAudio/PipeWire | Verify exact device selection, explicit consent, start/stop, duration limit, valid WAV and Quit cleanup |
| `record-screen` | Local FFmpeg desktop/window/physical-area capture; Windows GDI and local Linux X11 | Verify all three modes on Windows, X11 desktop/area on Linux, negative monitor origins, FPS and Stop; macOS/Wayland are unavailable through this provider |
| `make-gif` | Local FFmpeg palette-based conversion | Convert disposable clips at boundary durations/FPS/width; verify playback, cancellation and output collisions |
| `convert-audio-video` | Fixed MP4, WebM, MP3, WAV and FLAC presets | Validate audio/video streams and provider codec availability; test malformed local inputs, cancellation and existing outputs |
| `take-photo` | One explicit camera frame to PNG through FFmpeg | Verify selected camera and confirmation; no microphone opens; cancel/unmount/Quit stop the job |
| `take-video` | Explicit camera-only MP4 through the shared recording controller | Verify device selection, no implicit audio, visible status, global/tray Stop and Quit |
| `recognize-text` | Native Windows OCR language packs; optional local Tesseract elsewhere | Recognize a generated local text fixture; test missing languages/provider, cancellation and oversized images |
| `barcode-reader` | Pure Rust `rxing` QR/1D decoding, with `rqrr` QR fallback | Round-trip generated QR and EAN/Code128 fixtures; verify unreadable images and Unicode payloads; never auto-open decoded links |
| `measure-color` | Existing `xcap`; Windows delayed cursor sample or display/pixel coordinates | Verify RGB/HEX/HSL/CMYK against known colors, display origins, HiDPI and boundary errors |

## Dependencies and provider requirements

- `windows` 0.62 supplies Windows COM endpoint controls and WinRT OCR. These APIs do not record automatically.
- `rxing` uses only decoding, QR, 1D, encoding support and multiple-code readers; broad image/client/network features are disabled.
- `rqrr`, `image`, `xcap` and existing cancellation infrastructure are reused.
- FFmpeg must be installed by the user and visible in `PATH`. Its build must include the needed device input,
  filters and selected encoders. MP4 uses the native MPEG-4/AAC preset; WebM needs VP9/Opus, MP3 needs LAME.
- The installed FFmpeg build may be LGPL or GPL depending on its configuration. It is a separate, user-managed
  executable. Oxbox does not redistribute it or claim that an arbitrary installed build is LGPL. Any future
  bundling requires a dedicated source, build-configuration, license and redistribution review.
- Tesseract requires installed language data. Windows OCR takes a BCP 47 code such as `en-US`; Tesseract takes
  codes such as `eng` or `eng+deu`. Oxbox does not install language packs or modify privacy permissions.
- Device names/indices are entered explicitly. The views show local enumeration commands; enumerating a
  device list manually does not authorize Oxbox to start recording it.

## Safety and lifecycle

Recording state is app-managed, starts idle and is shown globally while active. The user confirms the source,
duration and output before recording. Only one recording runs at a time. Stop requests FFmpeg finalization,
then terminates/reaps an unresponsive provider after five seconds. Quit requests the same cleanup. Microphone
mute changes remain as requested after Quit; automatically unmuting would violate the user's privacy choice.

Media jobs use private staging folders and publish with atomic hard-link creation or exclusive streaming copy.
An existing final path is never overwritten, including a file created after preflight. Failed recording output
is retained with its recovery path; a failed final copy can leave a partial new output. Source files are kept.
Conversions/GIF/photo jobs use shared progress/cancellation; limits are two hours per job and 20 GiB output.
Recordings additionally require a 1-second to 24-hour maximum duration. GIFs are limited to 60 seconds,
30 FPS and 1280 pixels wide. OCR/image decoding is bounded and recognized text is capped at 1 MiB.

File inputs are canonical regular local paths. FFmpeg receives a protocol allow-list of `file,pipe` and a
demuxer allow-list excluding playlists. Arguments are arrays, with fixed codec/filter presets. No arbitrary
shell command, remote URL, provider script or user-supplied FFmpeg option is accepted. Provider version/text
output is bounded and provider jobs are cancellable. No tests open a live camera/microphone, capture a real
screen, or change the developer's microphone state.

## Verification record

- Tests were written first; missing media API failures were observed before implementation.
- Core tests cover literal argument handling, duration/dimension/format errors, exclusive publishing,
  language validation, generated barcode decoding, and color conversion edges.
- Native recording, device selection, Windows OCR language availability and cross-platform runtime behavior
  remain manual release gates in [the testing guide](../docs/TESTING.md).

Primary references: [FFmpeg devices](https://ffmpeg.org/ffmpeg-devices.html),
[FFmpeg protocol controls](https://ffmpeg.org/ffmpeg-protocols.html),
[Tesseract command line](https://tesseract-ocr.github.io/tessdoc/Command-Line-Usage.html),
[Windows endpoint controls](https://learn.microsoft.com/en-us/windows/win32/api/endpointvolume/nn-endpointvolume-iaudioendpointvolume),
and [Windows OCR](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine).

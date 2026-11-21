Native renderer draft for maintainer review. These assets build `oxbox-native`; the retained legacy
Svelte/Tauri source is a comparison reference and is not embedded in the native executable.

Build outputs are unsigned. Windows MSI/NSIS packaging skips the WebView2 prerequisite and uses the
MSVC static CRT. macOS DMGs contain the native `.app`. Linux currently ships a portable GNU/Linux
archive, with desktop GTK/graphics/capture runtime libraries supplied by the system.

Review the tagged CHANGELOG, docs/VALIDATION.md, docs/RELEASE.md and tasks/native-migration.md.
Verify SHA256SUMS.txt, installer contents and target-specific notices. Complete clean-machine
install/uninstall, all-tool feature parity, keyboard/accessibility, HiDPI/multi-monitor, hidden
notification, power/input/hardware, Quit restoration, runtime network and human attribution/security
gates before publishing. Compilation and finite traffic samples do not establish a universal
zero-network guarantee. Optional user-installed FFmpeg/Tesseract providers are not redistributed.

This workflow keeps the release a draft and refuses to update an already published release.

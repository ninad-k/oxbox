# Troubleshooting

Include the tool ID, commit, OS/session and exact error when reporting a problem. Prefer generated fixtures
over uploads of private files. [Usage](USAGE.md) covers tool limits; [validation](VALIDATION.md) distinguishes
implementation from the platform/device checks that remain open.

| Symptom | Check or recovery |
|---|---|
| Rust reports an unsupported compiler | Use `cargo +1.95.0 …`; the selected eframe version requires Rust 1.95. Keep `--locked`. |
| Windows linker reports `LNK1104` | Quit the test app using its tray menu, coordinate concurrent builds, and use a short checkout or `CARGO_TARGET_DIR`. Do not kill somebody else's process. |
| Linux build lacks PipeWire/SPA, GTK, X11 or Wayland headers | Install the development packages in [CONTRIBUTING](../CONTRIBUTING.md#native-prerequisites). Ubuntu 24.04 is the CI reference. |
| Closing the window leaves Oxbox running | Close hides the launcher by design. Use the tray menu or Ctrl/Cmd+Q to stop owned workers and exit. |
| Dashboard is empty | Dashboard shows only favorites. Browse All tools or a category and star the tools you want. |
| Global shortcut is unavailable | Choose a free chord in Settings; keep the previous binding if registration fails. The default is Ctrl/Cmd+Alt+O. |
| Startup setting cannot be changed in a developer profile | Debug and explicit `--profile` runs deliberately cannot register startup. Test real login behavior only with an installed package in a disposable environment. |
| Existing output is refused | Choose a new filename. Outputs use exclusive creation to preserve user data. |
| Saved settings cannot be loaded | Preserve the file and error before making changes. Invalid stores surface errors; do not delete the profile as a routine fix. Redact private paths before sharing a report. |
| FFmpeg/Tesseract unavailable | Configure an existing local executable using [the provider guide](ROADMAP_TOOLS.md). Oxbox does not download providers. Check versions and exact errors with generated input. |
| Capture/clipboard/tray differs across desktops | Record X11/Wayland and desktop environment. OS permissions, protected surfaces and device availability can limit the backend. Do not disable OS security protections. |
| No notification appears | Check OS notification/focus settings yourself. Hidden timers use their own clocks; notification delivery and elapsed-time calculation are separate. |

## Browser preview cannot download a tool

`Failed to fetch dynamically imported module: http://127.0.0.1:4174/_app/…js` belongs to the retained
Svelte comparison interface. The native app does not fetch JavaScript modules. A stopped preview server
or rebuilding its hashed output while a tab remains open can make a tool's chunk unavailable.

For a stable browser preview, finish the build, start the server, and leave its terminal running:

```bash
pnpm build
pnpm preview --host 127.0.0.1 --port 4174 --strictPort
```

Save any unsaved form values before reloading the tab. After a rebuild, reload the whole interface so its
module references match the new build. During editing, use `pnpm dev` for hot reload instead of rebuilding
under a running preview. A browser preview shows the legacy forms; desktop commands require its Tauri host.
For the selected product run `cargo run -p oxbox-native --locked`.

If the error persists while the server is running, report the missing chunk URL, command and commit, whether
it returns a response in the browser, and the terminal error. Do not clear personal browser data or reinstall
WebView2 to address a missing local preview file.

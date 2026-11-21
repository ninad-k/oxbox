# Phase 5: desktop, system and application tools

This promotes the Windows/system/apps rows from `tasks/plan.md` into reviewable
implementation tasks. Runtime actions happen only after an explicit user action.
No tool terminates a process, elevates itself, installs a service or monitoring
driver, downloads software, or permanently removes user files.

The source implementation is present. The manual acceptance checks below remain
release gates until exercised on the stated operating systems.

| Tool id | Concrete behavior | Platforms | Automated coverage |
|---|---|---|---|
| `window-on-top` | List titled application windows, pin/unpin one using `SetWindowPos`; restore changes owned by Oxbox on Quit | Windows | Invalid/stale window and PID validation |
| `window-manager` | Snap a selected window to halves, thirds, quarters or its monitor work area, preserving taskbar space | Windows | Odd-size partitions, negative monitor origins, empty work-area rejection |
| `show-desktop` | Minimize normal application windows; remember and restore that group without closing anything | Windows | Current-process/system-process exclusion |
| `close-apps` | Confirm the exact selected windows, then send ordinary `WM_CLOSE` requests; save prompts remain controlled by each application | Windows | Confirmation, selection bounds, duplicate/invalid identifiers |
| `focus-window` | Put click-through transparent shades over each monitor, cut out the selected window, update its position every 250 ms; stop on target close/minimize or Quit | Windows | Clipped multi-monitor geometry and HiDPI conversion |
| `switch-resolution` | Enumerate driver-reported modes, test before applying, restore the original after 15 seconds unless kept; pending changes restore on Quit | Windows | Mode validation and deadline overflow; native effect is manual only |
| `eject-volumes` | List removable/optical drive letters; confirm, lock, dismount and request media ejection; never force a busy volume | Windows | Exact drive syntax and system-drive refusal |
| `energy-saver` | Review installed Windows power profiles and confirm before selecting one with literal `powercfg` arguments | Windows | Locale-independent GUID parsing and argument validation |
| `hidden-files` | Toggle ordinary hidden-file visibility; preserve Windows protected-system-file setting; macOS restarts Finder after confirmation | Windows/macOS | Shell flag masking preserves unrelated bits |
| `cpu-temperature` | Poll CPU-labelled sensors exposed by `sysinfo` while the view is open; explain unavailable sensors instead of fabricating data | Windows/macOS/Linux, hardware-dependent | Missing, non-finite, implausible readings and sensor-label filtering |
| `launch-app-sets` | Persist up to 64 named sets of 1–32 local applications/files; launch literal executable argument arrays and default file handlers | Windows/macOS/Linux | Empty/invalid sets, absolute local paths, NUL and argument constraints |
| `uninstall-apps` | List registered Windows desktop uninstallers; re-read and compare the reviewed command before launching a native executable; separately scan exact-name AppData folders, select none automatically, confirm before Trash | Windows | Command quoting/literal arguments and traversal-safe candidate names |

## Implementation map

- Core: one module per tool in `crates/oxbox-core/src/tools/`; native adapters in
  `crates/oxbox-core/src/platform/desktop.rs`.
- Shell: `src-tauri/src/roadmap_system_commands.rs`; transient dimming windows in
  `src-tauri/src/focus_overlay.rs`.
- Typed API: `src/lib/roadmap-system-api.ts`; one registry-loadable Svelte view per
  tool, with shared window-selection and system-selection components.
- Launch-set configuration: `app-sets.json` in the Tauri application config
  directory, written through the existing store plugin.

## Safety and platform limits

- Window operations identify both the native handle and process ID. A stale
  selection is rejected. Elevated/protected application windows can refuse a
  normal request; Oxbox does not bypass Windows security boundaries.
- A display change is temporary for the Windows session. Keeping it does not
  overwrite Windows' saved display configuration. Hardware removal or driver
  failure can prevent rollback; the error is shown, with Windows display settings
  as a recovery route.
- Drive ejection targets removable/optical media, not fixed external disks.
  Applications with open files can cause a safe refusal.
- CPU sensors are often unavailable through standard APIs on Windows. No extra
  driver is installed and administrator access is not requested.
- Launch sets accept local files and native applications. URL/network targets
  are excluded by the offline boundary. An application controls the effects of
  its own explicitly saved arguments.
- Uninstallers are trusted only as registered OS entries reviewed by the user.
  Script hosts, ambiguous executable paths and missing executables are refused;
  those applications must be removed through Windows Installed apps. Store apps
  are outside this registry-based list.
- Leftover candidates are exact display-name/publisher folder matches under user
  AppData, not a determination that a folder is unused. They can contain personal
  data. There is no automatic leftover cleanup or heuristic recursive deletion.
- Unsupported desktop APIs return a clear `unsupported` error on other platforms.

## Manual acceptance gates

- [ ] Pin/unpin several application windows, including an already-pinned window;
  Quit restores only changes still owned by Oxbox.
- [ ] Snap across multi-monitor layouts with negative origins and 100%/150% DPI;
  verify application minimum-size behavior.
- [ ] Show desktop/restore a group; close windows with unsaved documents and verify
  normal application save prompts without forced termination.
- [ ] Focus shade is transparent/click-through, follows a moved/resized window,
  stops on minimize/close, and leaves no overlay after Quit.
- [ ] Try a safe resolution, allow timeout, keep another, and Quit with a change
  pending. Record rollback behavior on the tested GPU/display.
- [ ] Eject test removable media successfully; an open-file/busy volume is refused.
- [ ] Switch between installed power profiles after confirmation; restore the
  original profile for the test machine.
- [ ] Hidden-file visibility persists and protected-file visibility remains
  unchanged. On macOS, Finder restarts only after confirmation.
- [ ] Polling stops on leaving CPU Temperature. Missing sensors show the supported
  limitation; available readings agree with an OS/hardware monitor.
- [ ] Save, reload, edit, delete and launch a local application/file set, including
  spaces and literal metacharacters in arguments.
- [ ] Review/start a disposable test application's uninstaller. Cancel its prompts
  safely. Scan exact-name candidate folders; confirm a selected fixture moves to
  Trash and can be restored.
- [ ] Build macOS/Linux and verify supported tools or explicit unavailable states.

## API references used

- [ChangeDisplaySettingsExW and CDS_TEST](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsexw)
- [EnumDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw)
- [Storage media ejection control](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/general-storage-io-control-codes)
- [Shell setting masks](https://learn.microsoft.com/en-us/windows/win32/shell/ssf-constants)

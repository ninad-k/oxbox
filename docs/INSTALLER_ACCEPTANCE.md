# Native Windows installer acceptance

The MSI and NSIS packages remain unsigned, unpublished review artifacts. Package extraction, table/source
inspection and compiler success do not establish successful installation, login startup, upgrade or removal.
Keep the release draft until the relevant gates in [RELEASE.md](RELEASE.md) pass.

## Reproducible inspection without installation

Use the existing Node runtime, Windows Installer read-only database API and cached WiX `dark.exe`.
The following commands do not execute the app, installers or MSI custom actions; they do not write startup
registry values. They create a fresh report/extraction directory below `target/installer-verification/`.

```powershell
node --test scripts/build-native.test.mjs scripts/verify-native-installers.test.mjs

# Use the exact reviewed EXE digest, not a hash obtained from an unreviewed replacement.
node scripts/verify-native-installers.mjs --expected-exe-sha256 <reviewed-sha256>
```

The verifier defaults to the final native binary under `target/native-dist/x86_64-pc-windows-msvc/release/`
and downloaded review assets under `target/native-release-draft/`. Optional `--draft`, `--native`,
`--nsis-source` and `--wix-tools` arguments select an explicit review set. The draft must contain its
`SHA256SUMS.txt` and both installers. Manifest names must be safe basenames without duplicates.

The verifier checks each manifest digest before invoking inspection tools, pins the EXE digest, opens the
MSI database read-only, extracts its cabinet with WiX, and compares the five payload files with the exact
native EXE, installed marker, notices and project licenses. It checks the cleanup action's command, scope
and ordering, and checks the no-automatic-shutdown property. For NSIS it compares the generated compiler
output with the review installer and checks the generated source/helper lifecycle guards. The JSON report
explicitly records that NSIS payload extraction and installer execution were not performed.

Inspect `report.json`, `msi-tables.json`, `installer.wxs`, `payload/` and `dark.log`. A failed check is not
an acceptance result. An archive extractor's output is not proof of the installer's executed behavior.
The original icon/version/PE-import checks and exact published-review hashes are recorded separately in
[VALIDATION.md](VALIDATION.md).

## Lifecycle correction found during this review

The previous generated NSIS helper offered Restart Manager force-shutdown, including silent operation.
That can bypass application cleanup after its timeout. The revised packaging helper retains running-file
detection and refuses continuation until the user has explicitly quit Oxbox from the tray; closing the
window is insufficient. Unknown generated helper layouts or detection failures stop packaging/installation.
The MSI fragment sets `MSIRESTARTMANAGERCONTROL=DisableShutdown`, retaining files-in-use detection while
disabling automatic shutdown. Microsoft's documentation describes both
[Restart Manager force shutdown](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/ne-restartmanager-rm_shutdown_type)
and the [MSI property](https://learn.microsoft.com/en-us/windows/win32/msi/msirestartmanagercontrol).

The new regression tests failed before these changes and pass afterward. The no-install verifier was also
run against build B's old MSI/NSIS: all five extracted MSI payload hashes and manifest/compiler-output hashes
matched, and it correctly reported the missing MSI shutdown property and missing NSIS running-app guard.
The replacement installer compiler/table checks must be recorded after the final approved EXE is packaged.
No build B installer was executed to demonstrate the force-shutdown risk.

## Available isolation and extraction tooling

On 2026-11-22 the bounded read-only inventory found a non-elevated Windows Home/Core host, no
`WindowsSandbox.exe`, Hyper-V PowerShell module or `vmms` service, and no VMware, VirtualBox or QEMU
executables or VM images in the inspected standard locations. Optional-feature queries required elevation.
This establishes no ready clean Windows test environment here; it is not proof that virtualization is
impossible. No features were enabled and no VM/tool/image was downloaded.

Cached WiX supports MSI extraction. Windows `tar` 3.8.9 rejected the NSIS installer as an unrecognized
archive. No 7-Zip executable was found in PATH or inspected standard/cache locations. The preinstalled
Inno Setup 7 archive libraries were queried for supported formats and contain no NSIS handler. NSIS itself
does not provide an extract-only installer switch; the project describes
[separate extraction tools](https://nsis.sourceforge.io/Can_I_decompile_an_existing_installer%3F).
Independent NSIS payload extraction therefore remains open. Do not run the installer merely to inspect it.

## Clean-machine protocol — pending

Run this protocol inside a disposable, consenting Windows 10/11 VM or clean machine. Use a snapshot and
dedicated test accounts, transfer the pinned installers read-only, and record OS/build, account privilege,
source commit, every asset digest, screenshots, exit codes and installer logs. Disable VM networking for
the offline-prerequisite test; do not change the host's security settings. Use a new snapshot for each
installer and reinstall scenario. No existing host data or real login policy belongs in these fixtures.

| Check | Procedure and expected result | Status |
| --- | --- | --- |
| Fresh install | Install to a path containing spaces. Verify the five payload files, original app/shortcut icons, version and Add/Remove Programs entry. No WebView2 or VC redistributable download is requested. | Pending |
| Account/scope | Verify NSIS current-user operation. Verify MSI machine-install elevation and execution identity; it must not launch an elevated app unexpectedly. | Pending |
| Fresh first launch | Launch the installed copy with a fresh account/profile. Verify normal launcher, conversion and Quit; only the approved current-user login entry is added. | Pending |
| Login-to-tray | Sign out/in inside the VM. Verify a single quiet tray process, shortcut/tray activation, second-instance restoration and clean Quit. A manually supplied `--autostart` flag does not prove actual login registration. | Pending |
| Startup choices | Turn startup off, then upgrade/reinstall; verify it stays off. Separately verify an existing OS-disabled startup entry and prior profile are preserved. | Pending |
| Running app | Keep a harmless native view open. Attempt install, upgrade and uninstall. NSIS must require tray Quit without killing the app; MSI must not automatically shut it down. Do not bypass files-in-use warnings and count that as a pass. | Pending |
| Upgrade | Upgrade from a pinned earlier native version with favorites/settings and startup choices. Verify data survives, one installation remains, and upgrades do not run full-uninstall startup cleanup. | Pending |
| Owned startup removal | Full uninstall as the same test user removes only the exact installed EXE's quoted `--autostart` command before deleting the EXE. | Pending |
| Repurposed startup value | In the disposable account only, replace the Oxbox Run value with a harmless different path/arguments. Full uninstall must preserve it. Record that this fixture deliberately changes only the VM registry. | Pending |
| Other user/elevation | Repeat with separate installing, running and uninstalling accounts. Cleanup is current-user HKCU only; determine and document any remaining entries for other accounts rather than claiming global removal. | Pending |
| Data preservation | Uninstall preserves app data by default; NSIS app-data deletion occurs only after its explicit checkbox/confirmation. User-created files outside app storage remain untouched. | Pending |
| Reinstall/cancel/failure | Cancel install/uninstall and exercise rollback. Reinstall after full removal. Verify no broken shortcuts, orphaned startup command or unexpected process remains. | Pending |
| Offline and protection | Record installer/app-owned processes and endpoints with networking disabled. Keep Windows protections enabled; unsigned prompts, DNS/provider behavior and security review remain separately documented. | Pending |

Do not activate live power, desktop, pointer, recording or destructive tools merely to satisfy installer
acceptance. Their restoration/provider/hardware gates have their own consent and test protocols. Record
actual observations rather than changing these rows based on unit tests or extraction success.

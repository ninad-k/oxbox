# Native Windows installer acceptance

The MSI and NSIS packages remain unsigned, unpublished review artifacts. Package extraction, table/source
inspection and compiler success do not establish successful installation, login startup, upgrade or removal.
Keep the release draft until the relevant gates in [RELEASE.md](RELEASE.md) pass.

## Current community candidate `67f0715`

The latest six-asset draft uses the rebuilt source `67f0715452fb601394f45d3296675d3b85df3fb5`, exact
Rust 1.95.0 and EXE SHA-256 `6606f3d4e172e75bbebbe4201d92b5989fca15ab51b2769cac0893dbac412069`.
Both the local set and all six re-downloaded assets pass byte-size/hash checks and the no-install verifier:
`target/installer-verification/run-ndm3cr/report.json` and `run-nExJgA/report.json`. All five MSI payloads,
lifecycle/upgrade properties, generated NSIS guards and compiler-output identity match. PE metadata records
Oxbox 0.1.0, NotSigned and 24 Windows system imports, without WebView/VC runtime redistributable matches.
The Common Controls v6 manifest retains `asInvoker` / `uiAccess=false` and coexists with six icons,
the icon group and version resource; icon pixel/digest comparison was not repeated.

[The validation record](VALIDATION.md#second-feature-pass-candidate-67f0715) contains every current artifact
digest and its separate bounded native UI/runtime evidence. Native app execution used a copied EXE with
an isolated profile, not installation. Independent NSIS extraction and actual clean-machine install/login/
upgrade/uninstall remain pending. The lifecycle table below stays open; prior candidates remain historical.

## Reproducible inspection without installation

Use the existing Node runtime, Windows Installer read-only database API and cached WiX `dark.exe`.
The following commands do not execute the app, installers or MSI custom actions; they do not write startup
registry values. They create a fresh report/extraction directory below `target/installer-verification/`.

```powershell
node --test scripts/build-native.test.mjs scripts/verify-native-installers.test.mjs

# Use the exact reviewed EXE digest, not a hash obtained from an unreviewed replacement.
node scripts/verify-native-installers.mjs --draft <review-assets-directory> --native <reviewed-exe> --expected-exe-sha256 <reviewed-sha256>
```

The verifier defaults to the final native binary under `target/native-dist/x86_64-pc-windows-msvc/release/`
and downloaded review assets under `target/native-release-draft/`. Optional `--draft`, `--native`,
`--nsis-source` and `--wix-tools` arguments select an explicit review set. The draft must contain its
`SHA256SUMS.txt` and both installers. Use explicit paths to avoid selecting an older default review
directory; the current download receipt names `target/verification2-remote-download/assets/`.
Manifest names must be safe basenames without duplicates.

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
The replacement installer compiler/table checks are recorded below. No build B installer was executed to
demonstrate the force-shutdown risk.

## Replacement package inspection — 2026-11-22

Frozen functional source `54dd5936475952d63c8b91ba2ea3935a92aca3e0` was built once using
`rustc 1.95.0 (59807616e 2026-04-14)` and the production packaging wrapper. The release compilation completed
in 4 minutes 13 seconds; WiX MSI and the patched NSIS compiler both succeeded. The old build B review files
were preserved. The fresh six-asset review set is under `target/native-release-review/`.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `oxbox-native.exe` | 13,523,456 | `428b28603ec2c16d825e833250fba1cb1a1dd06d2d9d6ef1f78b6e4253c94273` |
| `Oxbox-native_0.1.0_x64_en-US.msi` | 5,709,824 | `52624b9ea3f76eb1657c3dcfbb548a59d9aef62b700ed4f0d395d76aaf77ac9d` |
| `Oxbox-native_0.1.0_x64-setup.exe` | 4,022,405 | `0eb8fa654ea4bae1432d838ea097b39edf7a15335a5d477e910db44000d791e1` |
| `THIRD_PARTY_NOTICES.txt` | 3,696,014 | `6f22422e0098e8dc70702a06d63a321627d695fd0e9fbb952a137c8a85c792d2` |
| `SHA256SUMS.txt` | 447 | `fe42c2a724237ddaa62f92616008c861d5241b541e12b3708304992911b38340` |

The no-install verification command passed:

```powershell
node scripts/verify-native-installers.mjs --draft target/native-release-review --expected-exe-sha256 428b28603ec2c16d825e833250fba1cb1a1dd06d2d9d6ef1f78b6e4253c94273
```

Its report at `target/installer-verification/run-0pbXaL/report.json` records all manifest hashes, all five
extracted MSI payload matches, `DisableShutdown`, the native cleanup command/action type 82 at sequence
3499 before RemoveFiles, and the full-uninstall/non-upgrade condition. The final NSIS compiler output
matches the review installer; its generated helper contains the explicit tray-Quit refusal and no
`RmShutdown`/force-shutdown call. The source preserves the upgrade/owned-startup cleanup guard.

The native EXE has 24 Windows system-DLL imports, no WebView2 loader or VC redistributable import, and
version/product metadata 0.1.0/Oxbox. Its six icon payloads and the setup EXE's six original icon payloads
match `src-tauri/icons/icon.ico`; MSI's extracted ProductIcon is an exact ICO hash match. All native resource
leaf contents match the previously inspected original brand/version resources. Raw resource-section hashes
differ because section layout RVAs changed; that does not alter those leaf contents. Inspection reports are
under `target/native-inspection/source-54dd593/`.

Both PE certificate directories are empty and the MSI has no digital-signature table: the artifacts remain
unsigned. The NSIS compiler reported its stock unused WebView label and unused install-mode language-string
warnings; compilation succeeded. No installer/application/custom action was executed during this inspection.
Independent NSIS extraction, actual installation/login/upgrade/uninstall, signing and human attribution
review remain pending. This record does not close any clean-machine protocol row below.

## Provider announcement follow-up candidate — 2026-11-22

The current review candidate supersedes the preceding `54dd593` candidate with functional source
`d8d1d1eb1c1f26cc54ebc80b623e0c9e09a17ad5`. The native change adds accessible provider result/error
announcements and clears old provider status on a new explicit check. The earlier EXE and all six review
assets were copied and hash-checked into `target/native-inspection/source-54dd593/` before replacement.

The same exact Rust 1.95.0 production wrapper rebuilt the native crate in 3 minutes 56 seconds and compiled
both installers successfully, including the guarded NSIS helper. The two stock NSIS warnings described
above remain. The current six-file review stage is `target/native-release-review/`.

| Current artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `oxbox-native.exe` | 13,524,480 | `af363cd668e412f111bdfb37de39738ff35089e993585b19725932ba2de74b26` |
| `Oxbox-native_0.1.0_x64_en-US.msi` | 5,709,824 | `e2f9ab781123b37957bbc6ab2ed50edd30cdae296b52698ba3db86ccaafe3c1d` |
| `Oxbox-native_0.1.0_x64-setup.exe` | 4,024,370 | `38f8eb7e5c72770d25c8e99bbeef3b0f5eb7457a5bbcc790dc99215d40e28360` |
| `THIRD_PARTY_NOTICES.txt` | 3,696,014 | `6f22422e0098e8dc70702a06d63a321627d695fd0e9fbb952a137c8a85c792d2` |
| `SHA256SUMS.txt` | 447 | `fb7142dcea1b5effad32ab62547edaf8ee15d0aca3340e11a1d1eeac716d87c3` |

The current no-install command passed:

```powershell
node scripts/verify-native-installers.mjs --draft target/native-release-review --expected-exe-sha256 af363cd668e412f111bdfb37de39738ff35089e993585b19725932ba2de74b26
```

The report is `target/installer-verification/run-h5X6EI/report.json`; artifact, resource and PE reports are
under `target/native-inspection/source-d8d1d1e/`. All five extracted MSI payload hashes match the pinned
EXE and resource inputs. The MSI shutdown property, exact cleanup action/order/scope, NSIS running-app
guard, and NSIS compiler-output identity pass. The 24 native PE imports remain Windows system DLLs with
no WebView2 loader or VC redistributable import. All native brand/version resource contents match the
preceding candidate, all six native/setup icon payloads match the original ICO, and the MSI ProductIcon
is an exact ICO match. Both PE certificate directories and the MSI digital-signature table remain absent.

This inspection executed no app, installer or MSI custom action. Independent NSIS extraction and every
clean-machine protocol row remain pending. Provider announcement semantics are source/headless-test
evidence; they do not establish a completed native screen-reader acceptance pass.

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

import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyManifest, verifyMsiTables, verifyNsisSources } from './verify-native-installers.mjs';
import { patchNsisUninstall, patchNsisRunningCheck } from './build-native.mjs';

test('installer checksum manifests reject duplicate, traversal and malformed asset names', () => {
  const digest = 'a'.repeat(64);
  assert.deepEqual(verifyManifest(`${digest}  Oxbox-native_0.1.0_x64-setup.exe\n`), [{ name: 'Oxbox-native_0.1.0_x64-setup.exe', sha256: digest }]);
  for (const name of ['../a', 'a/b', 'a\\b', 'C:a', '.', 'a\u0000b']) {
    assert.throws(() => verifyManifest(`${digest}  ${name}\n`), /filename/);
  }
  assert.throws(() => verifyManifest(`${digest}  app.exe\n${digest}  APP.EXE\n`), /duplicate/);
  assert.throws(() => verifyManifest('not a checksum'), /checksum/);
});

const msi = () => ({
  Properties: [{ Property: 'MSIRESTARTMANAGERCONTROL', Value: 'DisableShutdown' }],
  Files: ['oxbox-native.exe', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_NOTICES.txt', 'oxbox-installed.json'].map((name, i) => ({ Id: i ? `resource${i}` : 'Path', Name: name, Bytes: 20 })),
  CustomActions: [{ Action: 'OxboxNativeUninstallCleanup', Type: 82, Source: 'Path', Target: '--uninstall-cleanup' }],
  Sequence: [{ Action: 'OxboxNativeUninstallCleanup', Condition: 'REMOVE="ALL" AND NOT UPGRADINGPRODUCTCODE', Sequence: 3499 }, { Action: 'RemoveFiles', Condition: '', Sequence: 3500 }],
});

test('MSI acceptance checks protect cleanup scope, ordering, exact payload and graceful lifecycle', () => {
  assert.deepEqual(verifyMsiTables(msi()), []);
  const forceShutdown = msi(); forceShutdown.Properties = [];
  assert.match(verifyMsiTables(forceShutdown).join('\n'), /automatic shutdown/);
  const lateCleanup = msi(); lateCleanup.Sequence[0].Sequence = 4000;
  assert.match(verifyMsiTables(lateCleanup).join('\n'), /before RemoveFiles/);
  const upgrade = msi(); upgrade.Sequence[0].Condition = 'REMOVE="ALL"';
  assert.match(verifyMsiTables(upgrade).join('\n'), /full uninstall outside upgrades/);
  const wrongAction = msi(); wrongAction.CustomActions[0].Target = '--autostart';
  assert.match(verifyMsiTables(wrongAction).join('\n'), /cleanup command/);
  const extra = msi(); extra.Files.push({ Id: 'extra', Name: 'WebView2Loader.dll', Bytes: 10 });
  assert.match(verifyMsiTables(extra).join('\n'), /payload/);
});

test('NSIS acceptance requires patched executable cleanup and non-forcing files-in-use guard', () => {
  const original = 'Section Uninstall\n  Delete "$INSTDIR\\${MAINBINARYNAME}.exe"\n  ${If} $UpdateMode <> 1\n    DeleteRegValue HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Run" "${PRODUCTNAME}"\n  ${EndIf}\nSectionEnd\n';
  const guard = '!macro CheckIfAppIsRunning executablePath productName\n  RmShutdown RmForceShutdown\n!macroend\n';
  const safe = patchNsisUninstall(original);
  assert.deepEqual(verifyNsisSources(safe, patchNsisRunningCheck(guard)), []);
  assert.match(verifyNsisSources(original, guard).join('\n'), /startup|running-app/);
  assert.match(verifyNsisSources(safe, guard).join('\n'), /running-app/);
  assert.match(verifyNsisSources(safe.replace('--uninstall-cleanup', '--autostart'), patchNsisRunningCheck(guard)).join('\n'), /cleanup/);
});

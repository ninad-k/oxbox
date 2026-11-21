import test from 'node:test';
import assert from 'node:assert/strict';
import { assertNativeGraph, nativeBuildEnvironment, parseOptions, bundleConfiguration, windowsResourceDefinition, portableResources, patchNsisUninstall } from './build-native.mjs';

const metadata = (nodes, extra = []) => ({
  workspace_members: ['native', 'legacy'],
  packages: [
    { id: 'native', name: 'oxbox-native' },
    { id: 'legacy', name: 'oxbox' },
    { id: 'core', name: 'oxbox-core' },
    { id: 'egui', name: 'eframe' },
    { id: 'webview', name: 'wry' },
    ...extra,
  ],
  resolve: { nodes },
});
const node = (id, ...dependencies) => ({ id, deps: dependencies.map(pkg => ({ pkg, dep_kinds: [{ kind: null }] })) });

test('legacy WebView packages may remain locked but must be unreachable from the native binary', () => {
  const graph = metadata([node('native', 'core', 'egui'), node('core'), node('egui'), node('legacy', 'webview'), node('webview')]);
  assert.deepEqual([...assertNativeGraph(graph)].sort(), ['oxbox-core', 'oxbox-native', 'eframe'].sort());
  graph.resolve.nodes.find(n => n.id === 'core').deps.push({ pkg: 'webview', dep_kinds: [{ kind: null }] });
  assert.throws(() => assertNativeGraph(graph), /wry/);
});

test('native graph validation includes build dependencies and rejects incomplete graphs', () => {
  const graph = metadata([node('native', 'core'), node('core')], [{ id: 'tauri', name: 'tauri-build' }]);
  graph.resolve.nodes[0].deps.push({ pkg: 'tauri', dep_kinds: [{ kind: 'build' }] });
  graph.resolve.nodes.push(node('tauri'));
  assert.throws(() => assertNativeGraph(graph), /tauri-build/);
  graph.resolve.nodes[0].deps = [{ pkg: 'missing', dep_kinds: [{ kind: null }] }];
  assert.throws(() => assertNativeGraph(graph), /incomplete/);
});

test('packaging accepts existing native formats and rejects target/format mistakes before executing tools', () => {
  assert.deepEqual(parseOptions(['--bundle'], 'win32').formats, ['msi', 'nsis']);
  assert.equal(parseOptions(['--target', 'x86_64-pc-windows-msvc'], 'win32').target, 'x86_64-pc-windows-msvc');
  assert.deepEqual(parseOptions(['--bundle'], 'darwin').formats, ['app', 'dmg']);
  assert.deepEqual(parseOptions(['--bundle'], 'linux').formats, ['tar.gz']);
  assert.throws(() => parseOptions(['--bundles', 'deb'], 'linux'), /tar.gz/);
  assert.throws(() => parseOptions(['--target', '../../bad'], 'win32'), /target/);
  assert.throws(() => parseOptions(['--skip-build'], 'win32'), /--bundle/);
});

test('MSVC static CRT flag respects encoded flags and preserves unrelated compiler settings', () => {
  const plain = nativeBuildEnvironment({ RUSTFLAGS: '-Cdebuginfo=1' }, 'x86_64-pc-windows-msvc');
  assert.equal(plain.RUSTFLAGS, '-Cdebuginfo=1 -Ctarget-feature=+crt-static');
  const encoded = nativeBuildEnvironment({ CARGO_ENCODED_RUSTFLAGS: '-Cdebuginfo=1', RUSTFLAGS: 'ignored' }, 'x86_64-pc-windows-msvc');
  assert.equal(encoded.CARGO_ENCODED_RUSTFLAGS, '-Cdebuginfo=1\x1f-Ctarget-feature=+crt-static');
  assert.equal(nativeBuildEnvironment({ CARGO_ENCODED_RUSTFLAGS: '' }, 'x86_64-pc-windows-msvc').CARGO_ENCODED_RUSTFLAGS, '-Ctarget-feature=+crt-static');
  assert.equal(nativeBuildEnvironment({ RUSTFLAGS: 'keep' }, 'x86_64-unknown-linux-gnu').RUSTFLAGS, 'keep');
});

test('native Windows bundle cannot acquire a WebView prerequisite or updater bootstrapper', () => {
  const input = {
    productName: 'Oxbox', mainBinaryName: 'oxbox-native', identifier: 'dev.oxbox.desktop',
    bundle: { windows: { webviewInstallMode: { type: 'skip' }, wix: { fragmentPaths: ['packaging/native/uninstall.wxs'], componentRefs: ['OxboxNativeUninstallCleanupMarker'] }, nsis: { installerIcon: 'src-tauri/icons/icon.ico', uninstallerIcon: 'src-tauri/icons/icon.ico' } }, createUpdaterArtifacts: false, resources: { 'docs/THIRD_PARTY_NOTICES.txt': 'THIRD_PARTY_NOTICES.txt' }, icon: ['src-tauri/icons/icon.ico'] },
  };
  const output = bundleConfiguration(input, 'D:/source', '0.1.0');
  assert.equal(output.version, '0.1.0');
  assert.equal(output.bundle.windows.webviewInstallMode.type, 'skip');
  assert.equal(output.bundle.createUpdaterArtifacts, false);
  assert.ok(output.bundle.windows.wix.fragmentPaths[0].replaceAll('\\', '/').endsWith('/packaging/native/uninstall.wxs'));
  assert.equal(input.bundle.windows.wix.fragmentPaths[0], 'packaging/native/uninstall.wxs');
  assert.ok(output.bundle.windows.nsis.installerIcon.replaceAll('\\', '/').endsWith('/src-tauri/icons/icon.ico'));
  assert.ok(output.bundle.windows.nsis.uninstallerIcon.replaceAll('\\', '/').endsWith('/src-tauri/icons/icon.ico'));
  assert.equal(input.version, undefined);
  input.bundle.windows.webviewInstallMode.type = 'downloadBootstrapper';
  assert.throws(() => bundleConfiguration(input, 'D:/source', '0.1.0'), /WebView/);
});

test('Windows resource version validates numeric limits and preserves prerelease display identity', () => {
  const source = windowsResourceDefinition('1.2.3-beta.1', 'C:\\Oxbox source\\icon.ico');
  assert.match(source, /FILEVERSION 1,2,3,0/);
  assert.match(source, /VALUE "ProductVersion", "1\.2\.3-beta\.1\\0"/);
  assert.match(source, /1 ICON "C:\\\\Oxbox source\\\\icon\.ico"/);
  assert.throws(() => windowsResourceDefinition('1.2.70000', 'icon.ico'), /version/);
  assert.throws(() => windowsResourceDefinition('1.2.3\n4 ICON "bad"', 'icon.ico'), /version/);
});
test('portable Linux archives omit installed-only autorun policy while preserving licenses', () => {
  const resources = { 'packaging/native/installed.json': 'oxbox-installed.json', 'LICENSE-MIT': 'LICENSE-MIT', 'docs/THIRD_PARTY_NOTICES.txt': 'THIRD_PARTY_NOTICES.txt' };
  assert.deepEqual(portableResources(resources), [['LICENSE-MIT', 'LICENSE-MIT'], ['docs/THIRD_PARTY_NOTICES.txt', 'THIRD_PARTY_NOTICES.txt']]);
});
test('NSIS uninstall cleans only an owned startup command before deleting the executable and skips upgrades', () => {
  const source = 'Section Uninstall\n  Delete "$INSTDIR\\${MAINBINARYNAME}.exe"\n  ${If} $UpdateMode <> 1\n    DeleteRegValue HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Run" "${PRODUCTNAME}"\n  ${EndIf}\nSectionEnd\n';
  const output = patchNsisUninstall(source);
  assert.ok(output.indexOf('--uninstall-cleanup') < output.indexOf('Delete "$INSTDIR'));
  assert.match(output, /\$UpdateMode <> 1/);
  assert.doesNotMatch(output, /DeleteRegValue HKCU/);
  assert.equal(patchNsisUninstall(output), output);
  assert.throws(() => patchNsisUninstall('unknown future template'), /template/);
});

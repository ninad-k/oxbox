import test from 'node:test';
import assert from 'node:assert/strict';
import { selectNativeApplication, noticeNpmRoots, activePackageKeys, parseNoticeOptions } from './license-inventory.mjs';

test('production notices select the native workspace package while legacy remains available', () => {
  const native = { id: 'native', name: 'oxbox-native' };
  const legacy = { id: 'legacy', name: 'oxbox' };
  const graph = { workspace_members: ['native', 'legacy'], packages: [legacy, native] };
  assert.equal(selectNativeApplication(graph), native);
  assert.throws(() => selectNativeApplication({ ...graph, workspace_members: ['legacy'] }), /native workspace package/);
  assert.throws(() => selectNativeApplication({ workspace_members: ['legacy'], packages: [legacy] }), /native workspace package/);
});

test('native notices reject ambiguous native package selection', () => {
  const graph = { workspace_members: ['one', 'two'], packages: [{ id: 'one', name: 'oxbox-native' }, { id: 'two', name: 'oxbox-native' }] };
  assert.throws(() => selectNativeApplication(graph), /exactly one/);
});

test('copied Lucide icon sources retain conservative npm notice traversal without unused legacy API roots', () => {
  assert.deepEqual(noticeNpmRoots({ dependencies: { '@tauri-apps/api': '^2', '@lucide/svelte': '^1.77.0' }, devDependencies: { svelte: '^5', '@sveltejs/kit': '^3' } }), ['@lucide/svelte']);
  assert.throws(() => noticeNpmRoots({ dependencies: {} }), /Lucide icon source/);
});

test('active target tree excludes inactive optional packages from broad Cargo metadata', () => {
  assert.deepEqual([...activePackageKeys('oxbox-native v0.1.0 (D:/project)\neframe v0.37.0\neframe v0.37.0 (*)\n')], ['oxbox-native@0.1.0', 'eframe@0.37.0']);
  assert.throws(() => activePackageKeys('unexpected tree formatting'), /package line/);
});

test('alternate notice output leaves the checked-in Windows destination as the default', () => {
  assert.deepEqual(parseNoticeOptions([]), { check: false, target: 'x86_64-pc-windows-msvc', output: 'docs/THIRD_PARTY_NOTICES.txt' });
  assert.deepEqual(parseNoticeOptions(['--target', 'aarch64-apple-darwin', '--output', 'target/notices-macos.txt', '--check']), { check: true, target: 'aarch64-apple-darwin', output: 'target/notices-macos.txt' });
  assert.throws(() => parseNoticeOptions(['--output']), /output/);
  assert.throws(() => parseNoticeOptions(['--target', '../../invalid']), /target triple/);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { snapshotPackage } from './verify-vendor.mjs';
import { selectNativeApplication, noticeNpmRoots, activePackageKeys, parseNoticeOptions, loadVerifiedPatchedSources, cargoSourceDetails, cargoSourceLines } from './license-inventory.mjs';

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

async function patchedFixture(t) {
  const root = await mkdtemp(path.join(tmpdir(), 'oxbox-patched-notices-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const directory = 'vendor/glib-0.18.5';
  const packageRoot = path.join(root, directory);
  await mkdir(path.join(packageRoot, 'src'), { recursive: true });
  await mkdir(path.join(root, 'vendor', 'patches'));
  await writeFile(path.join(packageRoot, 'Cargo.toml'), '[package]\nname="glib"\nversion="0.18.5"\n');
  await writeFile(path.join(packageRoot, 'LICENSE'), 'Generated MIT fixture notice.\n');
  await writeFile(path.join(packageRoot, 'src', 'lib.rs'), 'pub const GENERATED: bool = true;\n');
  const patch = Buffer.from('--- a/src/lib.rs\n+++ b/src/lib.rs\nGenerated fixture patch.\n');
  const patchPath = 'vendor/patches/glib-0.18.5.patch';
  await writeFile(path.join(root, patchPath), patch);
  const record = {
    name: 'glib', version: '0.18.5', directory,
    archive: { url: 'https://static.crates.io/crates/glib/glib-0.18.5.crate', sha256: 'a'.repeat(64) },
    baseVcs: { repository: 'https://github.com/gtk-rs/gtk-rs-core', revision: 'b'.repeat(40), path: 'glib' },
    upstreamPatches: [{ revision: 'c'.repeat(40), url: 'https://github.com/gtk-rs/gtk-rs-core/commit/' + 'c'.repeat(40), appliedScope: 'Generated source fixture only.' }],
    appliedPatch: patchPath, ...await snapshotPackage(packageRoot),
  };
  const provenance = {
    schemaVersion: 1, packages: [record],
    artifacts: [{ path: patchPath, bytes: patch.length, sha256: createHash('sha256').update(patch).digest('hex') }],
  };
  await writeFile(path.join(root, 'vendor', 'PROVENANCE.json'), JSON.stringify(provenance));
  return {
    root, record, provenance,
    pkg: { name: 'glib', version: '0.18.5', source: null, manifest_path: path.join(packageRoot, 'Cargo.toml'), repository: record.baseVcs.repository },
  };
}

test('patched Cargo notices identify modified source and bind upstream reconstruction and exact hashes', async (t) => {
  const { root, pkg, record } = await patchedFixture(t);
  const inventory = await loadVerifiedPatchedSources(root);
  const details = cargoSourceDetails(pkg, inventory);
  const text = cargoSourceLines(details).join('\n');
  assert.match(text, /Patched source directory:/);
  assert.ok(text.includes('https://github.com/ninad-k/oxbox/tree/codex/native-renderer/vendor/glib-0.18.5'));
  assert.match(text, /Original unmodified upstream archive:/);
  assert.ok(text.includes(record.archive.url));
  assert.ok(text.includes(record.archive.sha256));
  assert.ok(text.includes(record.treeSha256));
  assert.ok(text.includes(record.upstreamPatches[0].url));
  assert.match(text, /Applied downstream patch SHA-256: [a-f0-9]{64}/);
  assert.match(text, /Patch provenance SHA-256: [a-f0-9]{64}/);
  assert.ok(!text.includes(root));
  assert.ok(!text.includes('Versioned package source: ' + record.archive.url));
});

test('patched Cargo notices refuse mismatched name version source and manifest admission', async (t) => {
  const { root, pkg } = await patchedFixture(t);
  const inventory = await loadVerifiedPatchedSources(root);
  for (const replacement of [
    { name: 'another-package' }, { version: '0.18.6' },
    { source: 'registry+https://github.com/rust-lang/crates.io-index' },
    { manifest_path: path.join(root, 'outside', 'Cargo.toml') },
    { manifest_path: path.join(root, 'vendor', 'glib-0.18.5', '..', 'unapproved', 'Cargo.toml') },
  ]) assert.throws(() => cargoSourceDetails({ ...pkg, ...replacement }, inventory), /patched.*identity|patched.*manifest/i);
});

test('patched Cargo notices refuse changed package bytes or missing applied-patch provenance', async (t) => {
  const { root, pkg, provenance } = await patchedFixture(t);
  await writeFile(path.join(root, 'vendor', 'glib-0.18.5', 'src', 'lib.rs'), 'Changed fixture source.\n');
  await assert.rejects(loadVerifiedPatchedSources(root), /source changed/i);
  await writeFile(path.join(root, 'vendor', 'glib-0.18.5', 'src', 'lib.rs'), 'pub const GENERATED: bool = true;\n');
  provenance.packages[0].appliedPatch = '../outside.patch';
  await writeFile(path.join(root, 'vendor', 'PROVENANCE.json'), JSON.stringify(provenance));
  await assert.rejects(loadVerifiedPatchedSources(root), /relative path|applied.patch/i);
  assert.equal(pkg.name, 'glib');
});

test('unmodified Cargo registry notices retain the original versioned source label', () => {
  const details = cargoSourceDetails({ name: 'serde', version: '1.0.229', source: 'registry+https://github.com/rust-lang/crates.io-index' });
  assert.deepEqual(cargoSourceLines(details), ['Versioned package source: https://crates.io/api/v1/crates/serde/1.0.229/download']);
});

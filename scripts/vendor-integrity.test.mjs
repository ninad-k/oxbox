import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm, rename, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { snapshotPackage, verifyPackage, validateRelativePath, verifyVendor } from './verify-vendor.mjs';

async function fixture(t) {
  const directory = await mkdtemp(path.join(tmpdir(), 'oxbox-vendor-integrity-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const root = path.join(directory, 'vendor', 'example-1.2.3');
  await mkdir(path.join(root, 'src'), { recursive: true });
  await writeFile(path.join(root, 'Cargo.toml'), '[package]\nname="example"\nversion="1.2.3"\n');
  await writeFile(path.join(root, 'src', 'lib.rs'), 'pub const EXAMPLE: u8 = 7;\n');
  await writeFile(path.join(root, 'LICENSE'), 'Generated fixture license text.\n');
  return { directory, root };
}

test('vendor integrity deterministically accepts the exact recorded bytes', async (t) => {
  const { root } = await fixture(t);
  const recorded = await snapshotPackage(root);
  assert.equal(recorded.files.length, 3);
  assert.deepEqual(recorded, await snapshotPackage(root));
  assert.deepEqual(await verifyPackage(root, recorded), { files: 3, treeSha256: recorded.treeSha256 });
});

test('vendor integrity refuses edited, extra, removed and renamed source files', async (t) => {
  const { root } = await fixture(t);
  const recorded = await snapshotPackage(root);
  const source = path.join(root, 'src', 'lib.rs');
  await writeFile(source, 'pub const EXAMPLE: u8 = 8;\n');
  await assert.rejects(verifyPackage(root, recorded), /changed.*src\/lib\.rs/);
  await writeFile(source, 'pub const EXAMPLE: u8 = 7;\n');
  await writeFile(path.join(root, 'extra'), 'unexpected');
  await assert.rejects(verifyPackage(root, recorded), /unexpected.*extra/);
  await rm(path.join(root, 'extra'));
  await rename(source, path.join(root, 'src', 'renamed.rs'));
  await assert.rejects(verifyPackage(root, recorded), /missing.*src\/lib\.rs/);
});

test('vendor integrity rejects escaping metadata and inconsistent tree digests', async (t) => {
  const { root } = await fixture(t);
  const recorded = await snapshotPackage(root);
  for (const invalid of ['', '.', '../file', 'src/../file', '/file', 'C:/file', 'src\\file', 'src//file', 'src/\nfile']) {
    assert.throws(() => validateRelativePath(invalid), /relative path/);
  }
  assert.equal(validateRelativePath('src/.generated.rs'), 'src/.generated.rs');
  await assert.rejects(verifyPackage(root, { ...recorded, treeSha256: '0'.repeat(64) }), /tree digest/);
  await assert.rejects(verifyPackage(root, { ...recorded, files: [...recorded.files, recorded.files[0]] }), /duplicate/i);
});

test('vendor integrity rejects a symlink or Windows junction instead of following it', async (t) => {
  const { directory, root } = await fixture(t);
  const outside = path.join(directory, 'outside');
  await mkdir(outside);
  await writeFile(path.join(outside, 'outside.txt'), 'Do not read this through a source link.');
  await symlink(outside, path.join(root, 'redirect'), process.platform === 'win32' ? 'junction' : 'dir');
  await assert.rejects(snapshotPackage(root), /symbolic link/);
});

test('vendor verifier checks provenance and applied-patch artifacts as well as package sources', async (t) => {
  const { directory, root } = await fixture(t);
  const recorded = await snapshotPackage(root);
  const data = {
    schemaVersion: 1,
    packages: [{
      name: 'example', version: '1.2.3', directory: 'vendor/example-1.2.3',
      archive: { url: 'https://static.crates.io/crates/example/example-1.2.3.crate', sha256: 'a'.repeat(64) },
      baseVcs: { revision: 'b'.repeat(40), path: 'example' },
      upstreamPatches: [{ revision: 'c'.repeat(40), url: 'https://github.com/example/project/commit/' + 'c'.repeat(40) }],
      ...recorded,
    }],
    artifacts: [],
  };
  const metadata = path.join(directory, 'vendor', 'PROVENANCE.json');
  await writeFile(metadata, JSON.stringify(data));
  const report = await verifyVendor(directory);
  assert.equal(report.packages[0].files, 3);
  data.packages[0].baseVcs.path = '';
  await writeFile(metadata, JSON.stringify(data));
  assert.equal((await verifyVendor(directory)).packages[0].files, 3);
  data.packages[0].directory = '../outside';
  await writeFile(metadata, JSON.stringify(data));
  await assert.rejects(verifyVendor(directory), /relative path|package directory/);
  data.packages[0].directory = 'vendor/example-1.2.3';
  const patchBytes = Buffer.from('Generated fixture patch.\n');
  await writeFile(path.join(directory, 'vendor', 'example.patch'), patchBytes);
  data.artifacts = [{ path: 'vendor/example.patch', sha256: createHash('sha256').update(patchBytes).digest('hex'), bytes: patchBytes.length }];
  await writeFile(metadata, JSON.stringify(data));
  assert.equal((await verifyVendor(directory)).artifacts, 1);
  await writeFile(path.join(directory, 'vendor', 'example.patch'), 'Changed fixture patch.\n');
  await assert.rejects(verifyVendor(directory), /Patch artifact changed/);
  data.artifacts = [{ path: 'vendor/absent.patch', sha256: 'd'.repeat(64), bytes: 5 }];
  await writeFile(metadata, JSON.stringify(data));
  await assert.rejects(verifyVendor(directory), /absent\.patch/);
});

test('vendor verifier checks a fourth approved package instead of stopping at the original three', async (t) => {
  const { directory, root } = await fixture(t);
  const packages = [];
  for (const name of ['example', 'second', 'third', 'fourth']) {
    const source = name === 'example' ? root : path.join(directory, 'vendor', `${name}-1.2.3`);
    if (name !== 'example') {
      await mkdir(source);
      await writeFile(path.join(source, 'Cargo.toml'), `[package]\nname="${name}"\nversion="1.2.3"\n`);
      await writeFile(path.join(source, 'LICENSE'), 'Generated fixture license.\n');
    }
    packages.push({
      name, version: '1.2.3', directory: `vendor/${name}-1.2.3`,
      archive: { url: `https://static.crates.io/crates/${name}/${name}-1.2.3.crate`, sha256: 'a'.repeat(64) },
      baseVcs: { revision: 'b'.repeat(40), path: '' },
      upstreamPatches: [{ revision: 'c'.repeat(40), url: 'https://github.com/example/project/commit/' + 'c'.repeat(40) }],
      ...await snapshotPackage(source),
    });
  }
  await writeFile(path.join(directory, 'vendor', 'PROVENANCE.json'), JSON.stringify({ schemaVersion: 1, packages, artifacts: [] }));
  assert.equal((await verifyVendor(directory)).packages.length, 4);
  await writeFile(path.join(directory, 'vendor', 'fourth-1.2.3', 'LICENSE'), 'Unexpected replacement.\n');
  await assert.rejects(verifyVendor(directory), /source changed/i);
});

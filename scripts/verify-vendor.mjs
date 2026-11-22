import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const sha256Pattern = /^[a-f0-9]{64}$/;
const revisionPattern = /^[a-f0-9]{40}$/;
const maximumFiles = 20_000;

/** Validate a repository-relative POSIX path before resolving it on any host. */
export function validateRelativePath(value) {
  if (typeof value !== 'string' || !value || /[\\:\x00-\x1f\x7f]/.test(value)
    || value.split('/').some((part) => !part || part === '.' || part === '..')) {
    throw new Error(`Invalid relative path: ${JSON.stringify(value)}`);
  }
  return value;
}

async function digestFile(filename) {
  const hash = createHash('sha256');
  let bytes = 0;
  for await (const chunk of createReadStream(filename)) {
    hash.update(chunk);
    bytes += chunk.length;
  }
  return { sha256: hash.digest('hex'), bytes };
}

function treeDigest(files) {
  const hash = createHash('sha256');
  for (const file of files) hash.update(`${file.sha256} ${file.bytes} ${JSON.stringify(file.path)}\n`, 'utf8');
  return hash.digest('hex');
}

function validateFiles(files) {
  if (!Array.isArray(files) || files.length === 0 || files.length > maximumFiles) {
    throw new Error('Invalid source file manifest length');
  }
  const seen = new Set();
  let previous = '';
  for (const file of files) {
    validateRelativePath(file.path);
    if (seen.has(file.path.toLowerCase())) throw new Error(`Duplicate source path: ${file.path}`);
    if (file.path < previous) throw new Error('Source manifest must be sorted by relative path');
    if (!sha256Pattern.test(file.sha256) || !Number.isSafeInteger(file.bytes) || file.bytes < 0) {
      throw new Error(`Invalid source checksum record: ${file.path}`);
    }
    previous = file.path;
    seen.add(file.path.toLowerCase());
  }
}

/** Hash every regular source file, refusing symlinks/junctions and special files. */
export async function snapshotPackage(root) {
  const files = [];
  const seen = new Set();
  async function walk(directory, relative) {
    const metadata = await lstat(directory);
    if (metadata.isSymbolicLink()) throw new Error(`Refusing symbolic link in vendor source: ${relative || '.'}`);
    if (!metadata.isDirectory()) throw new Error(`Expected a vendor source directory: ${relative || '.'}`);
    const children = await readdir(directory);
    children.sort();
    for (const child of children) {
      const entry = relative ? `${relative}/${child}` : child;
      validateRelativePath(entry);
      if (seen.has(entry.toLowerCase())) throw new Error(`Duplicate source path: ${entry}`);
      seen.add(entry.toLowerCase());
      const filename = path.join(directory, child);
      const info = await lstat(filename);
      if (info.isSymbolicLink()) throw new Error(`Refusing symbolic link in vendor source: ${entry}`);
      if (info.isDirectory()) await walk(filename, entry);
      else if (info.isFile()) {
        if (files.length === maximumFiles) throw new Error('Vendor source exceeds file-count limit');
        files.push({ path: entry, ...await digestFile(filename) });
      } else throw new Error(`Refusing special file in vendor source: ${entry}`);
    }
  }
  await walk(root, '');
  files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  validateFiles(files);
  return { treeSha256: treeDigest(files), files };
}

/** Compare exact package paths and bytes; no network or package execution occurs. */
export async function verifyPackage(root, recorded) {
  validateFiles(recorded.files);
  if (!sha256Pattern.test(recorded.treeSha256) || treeDigest(recorded.files) !== recorded.treeSha256) {
    throw new Error('Recorded source tree digest does not match its file manifest');
  }
  const actual = await snapshotPackage(root);
  const found = new Map(actual.files.map((file) => [file.path, file]));
  for (const expected of recorded.files) {
    const current = found.get(expected.path);
    if (!current) throw new Error(`Vendor source missing: ${expected.path}`);
    if (current.sha256 !== expected.sha256 || current.bytes !== expected.bytes) {
      throw new Error(`Vendor source changed: ${expected.path}`);
    }
    found.delete(expected.path);
  }
  if (found.size) throw new Error(`Vendor source has unexpected file: ${found.keys().next().value}`);
  if (actual.treeSha256 !== recorded.treeSha256) throw new Error('Vendor source tree digest changed');
  return { files: actual.files.length, treeSha256: actual.treeSha256 };
}

/** Verify all approved packages and their recorded patch/provenance artifacts offline. */
export async function verifyVendor(workspaceRoot) {
  const vendorDirectory = await lstat(path.join(workspaceRoot, 'vendor'));
  if (!vendorDirectory.isDirectory() || vendorDirectory.isSymbolicLink()) {
    throw new Error('Refusing a non-directory or symbolic link vendor root');
  }
  const metadataFile = path.join(workspaceRoot, 'vendor', 'PROVENANCE.json');
  const info = await lstat(metadataFile);
  if (!info.isFile() || info.isSymbolicLink() || info.size > 4 * 1024 * 1024) {
    throw new Error('Invalid or oversized vendor provenance file');
  }
  const data = JSON.parse(await readFile(metadataFile, 'utf8'));
  if (data.schemaVersion !== 1 || !Array.isArray(data.packages) || data.packages.length === 0
    || data.packages.length > 32 || !Array.isArray(data.artifacts) || data.artifacts.length > 128) {
    throw new Error('Invalid vendor provenance schema');
  }
  const seen = new Set();
  const reports = [];
  for (const item of data.packages) {
    if (!/^[a-z0-9][a-z0-9-]*$/.test(item.name) || !/^\d+\.\d+\.\d+$/.test(item.version)) {
      throw new Error('Invalid vendor package identity');
    }
    validateRelativePath(item.directory);
    if (item.directory !== `vendor/${item.name}-${item.version}` || seen.has(item.directory)) {
      throw new Error('Invalid or duplicate vendor package directory');
    }
    seen.add(item.directory);
    if (item.archive?.url !== `https://static.crates.io/crates/${item.name}/${item.name}-${item.version}.crate`
      || !sha256Pattern.test(item.archive?.sha256) || !revisionPattern.test(item.baseVcs?.revision)) {
      throw new Error(`Invalid upstream source record: ${item.name}`);
    }
    // Cargo records an empty path_in_vcs when the package is the repository root.
    if (item.baseVcs.path !== '') validateRelativePath(item.baseVcs.path);
    if (!Array.isArray(item.upstreamPatches) || !item.upstreamPatches.length) throw new Error('Missing upstream patch record');
    for (const patch of item.upstreamPatches) {
      if (!revisionPattern.test(patch.revision) || !/^https:\/\/github\.com\/[^/]+\/[^/]+\/commit\/[a-f0-9]{40}$/.test(patch.url)
        || !patch.url.endsWith(patch.revision)) throw new Error('Invalid immutable upstream patch record');
    }
    reports.push({ name: item.name, version: item.version, ...await verifyPackage(path.join(workspaceRoot, item.directory), item) });
  }
  const artifactPaths = new Set();
  for (const artifact of data.artifacts) {
    validateRelativePath(artifact.path);
    if (!artifact.path.startsWith('vendor/') || artifactPaths.has(artifact.path.toLowerCase())
      || artifact.path === 'vendor/PROVENANCE.json' || !sha256Pattern.test(artifact.sha256)
      || !Number.isSafeInteger(artifact.bytes) || artifact.bytes < 0) throw new Error('Invalid patch artifact record');
    artifactPaths.add(artifact.path.toLowerCase());
    const filename = path.join(workspaceRoot, artifact.path);
    const parts = artifact.path.split('/');
    for (let index = 1; index <= parts.length; index++) {
      const current = path.join(workspaceRoot, ...parts.slice(0, index));
      if ((await lstat(current)).isSymbolicLink()) throw new Error(`Refusing symbolic link in patch artifact: ${artifact.path}`);
    }
    const metadata = await lstat(filename);
    if (!metadata.isFile()) throw new Error(`Expected regular patch artifact: ${artifact.path}`);
    const actual = await digestFile(filename);
    if (actual.sha256 !== artifact.sha256 || actual.bytes !== artifact.bytes) throw new Error(`Patch artifact changed: ${artifact.path}`);
  }
  return { schemaVersion: 1, packages: reports, artifacts: data.artifacts.length };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 2) {
    console.error('Usage: node scripts/verify-vendor.mjs');
    process.exitCode = 1;
  } else {
    try {
      const workspace = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
      console.log(JSON.stringify(await verifyVendor(workspace), null, 2));
    } catch (error) {
      console.error(`Vendor integrity verification failed: ${error.message}`);
      process.exitCode = 1;
    }
  }
}

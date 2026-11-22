import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { verifyVendor, validateRelativePath } from './verify-vendor.mjs';

/** Production notice roots; legacy packages can remain locked without shipping. */
export function selectNativeApplication(metadata) {
  const candidates = metadata.packages.filter(pkg => pkg.name === 'oxbox-native' && metadata.workspace_members.includes(pkg.id));
  if (!candidates.length) throw new Error('The Oxbox native workspace package was not found.');
  if (candidates.length !== 1) throw new Error('Expected exactly one Oxbox native workspace package.');
  return candidates[0];
}

/** Preserve the source notices for icon vectors copied into the native host. */
export function noticeNpmRoots(manifest) {
  if (!Object.hasOwn(manifest.dependencies ?? {}, '@lucide/svelte')) throw new Error('The Lucide icon source dependency is missing.');
  return ['@lucide/svelte'];
}

/** Cargo metadata can include inactive optional edges; tree selects actual features. */
export function activePackageKeys(tree) {
  const selected = new Set();
  for (const line of tree.split(/\r?\n/).filter(line => line.trim())) {
    const match = /^([A-Za-z0-9_-]+) v([^\s]+)(?: .*)?$/.exec(line);
    if (!match) throw new Error('Unrecognized Cargo tree package line.');
    selected.add(`${match[1]}@${match[2]}`);
  }
  if (!selected.size) throw new Error('The active Cargo package tree was empty.');
  return selected;
}

export function parseNoticeOptions(args) {
  const options = { check: false, target: 'x86_64-pc-windows-msvc', output: 'docs/THIRD_PARTY_NOTICES.txt' };
  for (let index = 0; index < args.length; index++) {
    if (args[index] === '--check') options.check = true;
    else if (args[index] === '--target') options.target = args[++index];
    else if (args[index] === '--output') {
      options.output = args[++index];
      if (!options.output || options.output.startsWith('--') || options.output.includes('\0')) throw new Error('An output filename is required.');
    } else throw new Error('Usage: node scripts/generate-third-party-notices.mjs [--check] [--target TARGET] [--output PATH]');
  }
  if (!options.target || !/^[a-z0-9_-]+$/.test(options.target)) throw new Error('A valid Rust target triple is required.');
  return options;
}

export async function loadVerifiedPatchedSources(workspaceRoot) {
  const root = fs.realpathSync(workspaceRoot);
  const filename = path.join(root, 'vendor', 'PROVENANCE.json');
  const bytes = fs.readFileSync(filename);
  const verified = await verifyVendor(root);
  if (!fs.readFileSync(filename).equals(bytes)) throw new Error('Patched source provenance changed during verification.');
  const data = JSON.parse(bytes.toString('utf8'));
  const artifacts = new Map(data.artifacts.map(item => [item.path, item]));
  const verifiedPackages = new Map(verified.packages.map(item => [`${item.name}@${item.version}`, item]));
  const records = new Map();
  for (const item of data.packages) {
    const identity = `${item.name}@${item.version}`;
    const checked = verifiedPackages.get(identity);
    if (!checked || checked.treeSha256 !== item.treeSha256 || checked.files !== item.files.length) {
      throw new Error('Patched source identity does not match the verified provenance.');
    }
    validateRelativePath(item.appliedPatch);
    const artifact = artifacts.get(item.appliedPatch);
    if (!artifact || item.appliedPatch !== `vendor/patches/${item.name}-${item.version}.patch`) {
      throw new Error('Missing or mismatched applied-patch provenance.');
    }
    if (!/^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(item.baseVcs.repository)) {
      throw new Error('Patched source upstream repository is not a public GitHub source reference.');
    }
    for (const fix of item.upstreamPatches) {
      if (fix.appliedScope !== undefined && (typeof fix.appliedScope !== 'string'
        || fix.appliedScope.length > 2048 || /[\x00-\x1f\x7f]/.test(fix.appliedScope))) {
        throw new Error('Invalid applied-patch description.');
      }
    }
    const manifestPath = path.join(root, item.directory, 'Cargo.toml');
    if (path.relative(manifestPath, fs.realpathSync(manifestPath)) !== '') {
      throw new Error('Patched source manifest does not match its recorded directory.');
    }
    records.set(identity, { ...item, manifestPath, patchArtifact: artifact });
  }
  return {
    root,
    provenanceSha256: crypto.createHash('sha256').update(bytes).digest('hex'),
    records,
  };
}

const projectSource = 'https://github.com/ninad-k/oxbox';
// A stable branch locator plus independent content hashes avoids Git HEAD-dependent
// generated notices whose own commit would otherwise change their contents.
const projectRef = 'codex/native-renderer';
const sourceUrl = (kind, relative) => `${projectSource}/${kind}/${projectRef}/${validateRelativePath(relative).split('/').map(encodeURIComponent).join('/')}`;
const isWithin = (directory, filename) => {
  const relative = path.relative(directory, filename);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
};

/** Bind Cargo's selected path package to verified source before describing it. */
export function cargoSourceDetails(pkg, inventory) {
  const recorded = inventory?.records.get(`${pkg.name}@${pkg.version}`);
  const manifest = typeof pkg.manifest_path === 'string' && path.isAbsolute(pkg.manifest_path) ? path.resolve(pkg.manifest_path) : null;
  if (recorded) {
    if (pkg.source != null) throw new Error('Patched Cargo package identity resolved to an unpatched source.');
    if (!manifest || path.relative(recorded.manifestPath, manifest) !== '') {
      throw new Error('Patched Cargo manifest does not match its verified package identity.');
    }
    const basePath = recorded.baseVcs.path ? `/${recorded.baseVcs.path.split('/').map(encodeURIComponent).join('/')}` : '';
    return {
      source: sourceUrl('tree', recorded.directory),
      patched: true,
      additionalLines: [
        'Source locator scope: project branch URLs are mutable locators; exact source bytes are bound by the tree/provenance hashes below.',
        `Modified source tree SHA-256: ${recorded.treeSha256}`,
        `Patch provenance: ${sourceUrl('blob', 'vendor/PROVENANCE.json')}`,
        `Patch provenance SHA-256: ${inventory.provenanceSha256}`,
        `Original unmodified upstream archive: ${recorded.archive.url}`,
        `Original upstream archive SHA-256: ${recorded.archive.sha256}`,
        `Published upstream source revision: ${recorded.baseVcs.repository}/tree/${recorded.baseVcs.revision}${basePath}`,
        `Applied downstream patch: ${sourceUrl('blob', recorded.appliedPatch)}`,
        `Applied downstream patch SHA-256: ${recorded.patchArtifact.sha256}`,
        ...recorded.upstreamPatches.flatMap(fix => [
          `Upstream change reference: ${fix.url}`,
          ...(fix.appliedScope ? [`Applied change scope: ${fix.appliedScope}`] : []),
        ]),
      ],
    };
  }
  if (inventory && manifest && isWithin(path.join(inventory.root, 'vendor'), manifest)) {
    throw new Error('Patched Cargo package identity is not present in the verified provenance.');
  }
  const repository = typeof pkg.repository === 'string' ? pkg.repository.replace(/^git\+/, '').replace(/\.git$/, '') : '';
  const source = pkg.source?.startsWith('registry+')
    ? `https://crates.io/api/v1/crates/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}/download`
    : /^https?:\/\//i.test(repository) ? repository : 'Local or non-registry source — maintainer source review required';
  return { source, patched: false, additionalLines: [] };
}

export function cargoSourceLines(details) {
  return [`${details.patched ? 'Patched source directory' : 'Versioned package source'}: ${details.source}`, ...details.additionalLines];
}

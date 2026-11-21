#!/usr/bin/env node
/**
 * Generate a deterministic, conservative Windows dependency notice inventory.
 * Requires the existing Node/Cargo toolchains and installed frozen UI dependencies.
 * No packages are downloaded by this script and no lockfile is modified.
 */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
let check = false;
let target = 'x86_64-pc-windows-msvc';
for (let index = 0; index < args.length; index++) {
  if (args[index] === '--check') check = true;
  else if (args[index] === '--target') target = args[++index];
  else throw new Error('Usage: node scripts/generate-third-party-notices.mjs [--check] [--target TARGET]');
}
if (!target || !/^[a-z0-9_-]+$/.test(target)) throw new Error('A valid Rust target triple is required.');
const output = path.join(root, 'docs', 'THIRD_PARTY_NOTICES.txt');
const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const slash = value => value.split(path.sep).join('/');
const readJson = filename => JSON.parse(fs.readFileSync(filename, 'utf8'));
const checksum = filename => crypto.createHash('sha256').update(fs.readFileSync(path.join(root, filename))).digest('hex');
const within = (directory, filename) => {
  const relative = path.relative(directory, filename);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
};
const publicUrl = value => {
  const text = typeof value === 'string' ? value.replace(/^git\+/, '').replace(/\.git$/, '') : '';
  return /^https?:\/\//i.test(text) ? text : '';
};

function licenseFiles(directory, specified) {
  const base = fs.realpathSync(directory);
  const found = new Set();
  const add = filename => {
    if (!fs.existsSync(filename) || !fs.statSync(filename).isFile()) return;
    const resolved = fs.realpathSync(filename);
    if (!within(base, resolved)) throw new Error('A dependency license points outside its package directory.');
    found.add(resolved);
  };
  if (specified) add(path.resolve(base, specified));
  const recognized = /^(licen[cs]e|copying|copyright|notice|authors)(?:[._-].*)?$/i;
  const scan = (directoryName, insideLegalDirectory = false, depth = 0) => {
    if (depth > 5) return;
    for (const entry of fs.readdirSync(directoryName, { withFileTypes: true })) {
      const filename = path.join(directoryName, entry.name);
      if (entry.isFile() && (recognized.test(entry.name) || insideLegalDirectory)) add(filename);
      else if (entry.isDirectory() && (insideLegalDirectory || /^(licen[cs]es?|notices?|legal)$/i.test(entry.name))) scan(filename, true, depth + 1);
    }
  };
  scan(base);
  return [...found].sort((a, b) => compare(slash(path.relative(base, a)), slash(path.relative(base, b)))).map(filename => {
    const bytes = fs.readFileSync(filename);
    if (bytes.length > 2 * 1024 * 1024 || bytes.includes(0)) throw new Error('A dependency notice is not a bounded text document.');
    return { name: slash(path.relative(base, filename)), text: new TextDecoder('utf-8', { fatal: true }).decode(bytes) };
  });
}

let metadata;
try {
  metadata = JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--offline', '--format-version', '1', '--filter-platform', target], {
    cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  }));
} catch {
  throw new Error('Locked offline Cargo metadata failed. Install/fetch the approved dependencies first; the generator does not update them.');
}
const packages = new Map(metadata.packages.map(pkg => [pkg.id, pkg]));
const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
const application = metadata.packages.find(pkg => pkg.name === 'oxbox' && metadata.workspace_members.includes(pkg.id));
if (!application) throw new Error('The Oxbox desktop workspace package was not found.');
const uses = new Map();
const queue = [[application.id, 'runtime']];
while (queue.length) {
  const [id, use] = queue.shift();
  const usage = uses.get(id) ?? new Set();
  if (usage.has(use)) continue;
  usage.add(use); uses.set(id, usage);
  for (const dependency of nodes.get(id)?.deps ?? []) {
    const pkg = packages.get(dependency.pkg);
    if (!pkg) throw new Error('The Cargo dependency graph is incomplete.');
    for (const kind of dependency.dep_kinds) {
      if (kind.kind === 'dev') continue;
      const build = use === 'build' || kind.kind === 'build' || pkg.targets.some(target => target.kind.includes('proc-macro'));
      queue.push([dependency.pkg, build ? 'build' : 'runtime']);
    }
  }
}
const entries = [];
for (const [id, usage] of uses) {
  if (metadata.workspace_members.includes(id)) continue;
  const pkg = packages.get(id);
  const registry = pkg.source?.startsWith('registry+');
  const source = registry ? `https://crates.io/api/v1/crates/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}/download` : publicUrl(pkg.repository);
  entries.push({
    ecosystem: 'Cargo', name: pkg.name, version: pkg.version,
    license: pkg.license || 'UNSPECIFIED — review required', source: source || 'Local or non-registry source — maintainer source review required',
    repository: publicUrl(pkg.repository), usage: [...usage].sort(compare).join(' + '),
    files: licenseFiles(path.dirname(pkg.manifest_path), pkg.license_file),
  });
}

function resolveNpmPackage(name, fromDirectory) {
  let directory = fromDirectory;
  while (true) {
    const manifest = path.join(directory, 'node_modules', ...name.split('/'), 'package.json');
    if (fs.existsSync(manifest)) return fs.realpathSync(manifest);
    const parent = path.dirname(directory);
    if (parent === directory) return null;
    directory = parent;
  }
}
const uiManifest = readJson(path.join(root, 'package.json'));
// These two frameworks contribute browser runtime code despite appearing as devDependencies.
const uiRoots = [...new Set([...Object.keys(uiManifest.dependencies ?? {}), 'svelte', '@sveltejs/kit'])].sort(compare);
const npmQueue = uiRoots.map(name => ({ name, from: root, optional: false }));
const npmVisited = new Set();
const absentOptional = new Set();
while (npmQueue.length) {
  const next = npmQueue.shift();
  const manifest = resolveNpmPackage(next.name, next.from);
  if (!manifest) {
    if (next.optional) { absentOptional.add(next.name); continue; }
    throw new Error(`Installed UI dependency is missing: ${next.name}. Run pnpm install --frozen-lockfile.`);
  }
  if (npmVisited.has(manifest)) continue;
  npmVisited.add(manifest);
  const pkg = readJson(manifest);
  const directory = path.dirname(manifest);
  const declaredLicense = typeof pkg.license === 'string' ? pkg.license : pkg.license?.type;
  const license = declaredLicense || (pkg.licenses ?? []).map(item => typeof item === 'string' ? item : item.type).filter(Boolean).join(' OR ') || 'UNSPECIFIED — review required';
  const specified = /^SEE LICEN[CS]E IN (.+)$/i.exec(license)?.[1];
  const repository = publicUrl(typeof pkg.repository === 'string' ? pkg.repository : pkg.repository?.url);
  entries.push({ ecosystem: 'npm', name: pkg.name, version: pkg.version, license, repository,
    source: `https://registry.npmjs.org/${encodeURIComponent(pkg.name)}/-/${pkg.name.split('/').at(-1)}-${encodeURIComponent(pkg.version)}.tgz`,
    usage: 'UI runtime/framework package inventory (may include build helpers)', files: licenseFiles(directory, specified),
  });
  const children = { ...(pkg.dependencies ?? {}), ...(pkg.optionalDependencies ?? {}) };
  for (const name of Object.keys(children).sort(compare)) npmQueue.push({ name, from: directory, optional: Object.hasOwn(pkg.optionalDependencies ?? {}, name) });
  for (const name of Object.keys(pkg.peerDependencies ?? {}).sort(compare)) {
    if (!Object.hasOwn(children, name)) npmQueue.push({ name, from: directory, optional: Boolean(pkg.peerDependenciesMeta?.[name]?.optional) });
  }
}

entries.sort((a, b) => compare(a.ecosystem, b.ecosystem) || compare(a.name, b.name) || compare(a.version, b.version));
const distinct = [...new Map(entries.map(entry => [`${entry.ecosystem}:${entry.name}@${entry.version}`, entry])).values()];
const supplementsDirectory = path.join(root, 'docs', 'third-party-licenses');
const supplementsFile = path.join(supplementsDirectory, 'sources.json');
const supplements = fs.existsSync(supplementsFile) ? readJson(supplementsFile) : {};
for (const entry of distinct) {
  const supplement = supplements[`${entry.ecosystem}:${entry.name}@${entry.version}`];
  entry.review = supplement?.review;
  entry.selectedLicense = supplement?.selectedLicense;
  for (const file of supplement?.files ?? []) {
    const filename = path.resolve(supplementsDirectory, file.file);
    if (!within(supplementsDirectory, filename) || !within(supplementsDirectory, fs.realpathSync(filename))) throw new Error('A notice supplement points outside its checked-in directory.');
    const bytes = fs.readFileSync(filename);
    if (crypto.createHash('sha256').update(bytes).digest('hex') !== file.sha256 || !publicUrl(file.url)) throw new Error('A notice supplement has invalid provenance or changed content.');
    entry.files.push({ name: `upstream/${path.basename(filename)}`, origin: file.url, text: new TextDecoder('utf-8', { fatal: true }).decode(bytes) });
  }
}
const missing = distinct.filter(entry => entry.files.length === 0);
const attributionReviews = distinct.filter(entry => entry.review);
const lines = [
  'OXBOX — THIRD-PARTY DEPENDENCY NOTICES',
  'Generated by: node scripts/generate-third-party-notices.mjs',
  `Rust target: ${target}`,
  `Cargo.lock SHA-256: ${checksum('Cargo.lock')}`,
  `pnpm-lock.yaml SHA-256: ${checksum('pnpm-lock.yaml')}`,
  '',
  'This is a reproducible dependency-source inventory, not a completed legal audit.',
  'Cargo normal/build dependencies for the desktop target are included; build-only tools are labeled.',
  'The npm inventory covers installed UI dependencies and Svelte/SvelteKit framework sources, including',
  'package-level build helpers and installed optional dependencies. It is a conservative superset of the',
  'modules in the browser bundle. Regenerate from frozen lockfiles and installed dependencies.',
  'License expressions and upstream license/copyright/NOTICE texts below are preserved from package sources.',
  'For notices omitted from published package archives, checked-in upstream supplements retain their',
  'revision-pinned source URLs and verified SHA-256 provenance in docs/third-party-licenses/sources.json.',
  'Source URLs identify the exact original package version. Downstream modifications require their own',
  'corresponding-source review; MPL-2.0-covered packages have explicit source retrieval links below.',
  'Operating-system runtimes and installer-toolchain redistributables require a separate notice review.',
  'FFmpeg and Tesseract are user-installed optional providers and are not redistributed by Oxbox.',
  `Dependency packages inventoried: ${distinct.length}`,
  `Packages without notice texts after upstream supplements: ${missing.length}`,
  `Packages with an unresolved upstream attribution review: ${attributionReviews.length}`,
  ...(absentOptional.size ? [`Absent optional UI packages (not inventoried): ${[...absentOptional].sort(compare).join(', ')}`] : []),
  '',
];
for (const entry of distinct) {
  lines.push('='.repeat(88), `${entry.ecosystem}: ${entry.name} ${entry.version}`, `Use: ${entry.usage}`, `Declared license: ${entry.license}`, `Versioned package source: ${entry.source}`);
  if (entry.selectedLicense) lines.push(`License option selected for this inventory: ${entry.selectedLicense}`);
  if (entry.repository) lines.push(`Upstream repository: ${entry.repository}`);
  if (/MPL-2\.0/.test(entry.license)) lines.push(`MPL covered-source retrieval: ${entry.source}`);
  lines.push('');
  if (entry.review) lines.push(`MAINTAINER REVIEW REQUIRED: ${entry.review}`, '');
  if (!entry.files.length) lines.push('REVIEW REQUIRED: no recognized license/NOTICE file was included in the installed package.', 'Consult the versioned source archive and upstream repository before distribution.', '');
  for (const file of entry.files) {
    lines.push(`----- BEGIN ${file.name} -----`, ...(file.origin ? [`Upstream notice source: ${file.origin}`] : []), file.text, `----- END ${file.name} -----`, '');
  }
}
const content = `${lines.join('\n')}\n`;
const privatePaths = [root, process.env.USERPROFILE, process.env.HOME].filter(Boolean);
if (privatePaths.some(value => content.includes(value) || content.includes(value.replaceAll('\\', '/')))) throw new Error('A local workspace or profile path would be exposed in the generated notice.');
if (check) {
  if (!fs.existsSync(output) || fs.readFileSync(output, 'utf8') !== content) throw new Error('docs/THIRD_PARTY_NOTICES.txt is stale. Regenerate it from the current frozen dependencies.');
} else {
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, content, 'utf8');
}
process.stdout.write(`${check ? 'Verified' : 'Generated'} docs/THIRD_PARTY_NOTICES.txt: ${distinct.length} packages, ${missing.length} missing notice texts, ${attributionReviews.length} upstream attribution reviews.\n`);
if (missing.length) process.stdout.write(`Review required: ${missing.map(entry => `${entry.ecosystem}:${entry.name}@${entry.version}`).join(', ')}\n`);
if (missing.length) process.exitCode = 1;
if (attributionReviews.length) process.stdout.write(`Upstream attribution review: ${attributionReviews.map(entry => `${entry.ecosystem}:${entry.name}@${entry.version}`).join(', ')}\n`);

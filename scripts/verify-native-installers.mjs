#!/usr/bin/env node
/** Inspect packages without executing installers, the app, or any MSI custom action. */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { patchNsisRunningCheck, patchNsisUninstall } from './build-native.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const payload = ['oxbox-native.exe', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_NOTICES.txt', 'oxbox-installed.json'];

export function verifyManifest(text) {
  const seen = new Set();
  return text.split(/\r?\n/).filter(line => line.trim()).map(line => {
    const match = /^([a-fA-F0-9]{64})  (.+)$/.exec(line);
    if (!match) throw new Error('Invalid SHA256SUMS checksum line.');
    const [, digest, name] = match;
    if (!/^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/.test(name) || name === '.' || name === '..') {
      throw new Error('Checksum asset filename must be a plain, safe basename.');
    }
    if (seen.has(name.toLowerCase())) throw new Error('Checksum manifest contains a duplicate asset.');
    seen.add(name.toLowerCase());
    return { name, sha256: digest.toLowerCase() };
  });
}

export function verifyMsiTables(report) {
  const errors = [];
  if (!report.Properties?.some(row => row.Property === 'MSIRESTARTMANAGERCONTROL' && row.Value === 'DisableShutdown')) {
    errors.push('MSI automatic shutdown is not disabled; a running app could lose owned-state cleanup.');
  }
  const files = report.Files ?? [];
  if (files.map(file => file.Name).sort().join('|') !== [...payload].sort().join('|') || files.some(file => Number(file.Bytes) <= 0)) {
    errors.push('MSI payload must contain exactly the native executable, installed marker, licenses and notices.');
  }
  const cleanup = report.CustomActions?.find(action => action.Action === 'OxboxNativeUninstallCleanup');
  if (!cleanup || Number(cleanup.Type) !== 82 || cleanup.Source !== 'Path' || cleanup.Target !== '--uninstall-cleanup') {
    errors.push('MSI cleanup command is not the reviewed immediate, ignored-return native File action.');
  }
  const sequence = report.Sequence?.find(row => row.Action === 'OxboxNativeUninstallCleanup');
  const remove = report.Sequence?.find(row => row.Action === 'RemoveFiles');
  if (!sequence || !remove || Number(sequence.Sequence) >= Number(remove.Sequence)) errors.push('MSI cleanup must run before RemoveFiles.');
  if (sequence?.Condition.replace(/\s+/g, ' ').trim() !== 'REMOVE="ALL" AND NOT UPGRADINGPRODUCTCODE') {
    errors.push('MSI cleanup must apply only to full uninstall outside upgrades.');
  }
  return errors;
}

export function verifyNsisSources(installer, utilities) {
  const errors = [];
  const normalized = installer.replaceAll('\r\n', '\n');
  try {
    if (patchNsisUninstall(normalized) !== normalized) errors.push('NSIS exact-owned startup cleanup patch is missing.');
    const command = `ExecWait '"$INSTDIR\\\${MAINBINARYNAME}.exe" --uninstall-cleanup' $0`;
    const cleanup = normalized.indexOf(command);
    const remove = normalized.indexOf('  Delete "$INSTDIR\\${MAINBINARYNAME}.exe"', normalized.indexOf('Section Uninstall\n'));
    if (cleanup < 0 || remove < 0 || cleanup >= remove || !normalized.slice(0, cleanup).endsWith('  ${If} $UpdateMode <> 1\n    ')) {
      errors.push('NSIS cleanup command/order/upgrade guard differs from the reviewed command.');
    }
  } catch (error) { errors.push(`NSIS startup cleanup: ${error.message}`); }
  try {
    if (patchNsisRunningCheck(utilities) !== utilities.replaceAll('\r\n', '\n')) errors.push('NSIS running-app safety guard is missing.');
  } catch (error) { errors.push(`NSIS running-app guard: ${error.message}`); }
  for (const setting of ['INSTALLWEBVIEW2MODE', 'MINIMUMWEBVIEW2VERSION']) {
    const value = new RegExp(`^!define ${setting} "([^"]*)"`, 'm').exec(normalized)?.[1];
    if (value != null && value !== '' && value !== 'skip') errors.push(`NSIS ${setting} enables an unexpected WebView prerequisite.`);
  }
  return errors;
}

function ordinary(filename, maxBytes = 64 * 1024 * 1024) {
  const stat = fs.lstatSync(filename);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maxBytes) throw new Error(`Expected a bounded ordinary file: ${path.basename(filename)}.`);
}
async function sha256(filename) {
  ordinary(filename);
  const hash = crypto.createHash('sha256');
  for await (const bytes of fs.createReadStream(filename)) hash.update(bytes);
  return hash.digest('hex');
}
function workspaceDirectory(directory) {
  const relative = path.relative(root, directory);
  if (relative.startsWith('..') || path.isAbsolute(relative)) throw new Error('Inspection output must stay inside this workspace.');
  for (let current = directory; current !== path.dirname(current); current = path.dirname(current)) {
    if (fs.existsSync(current) && fs.lstatSync(current).isSymbolicLink()) throw new Error('Inspection output cannot traverse a symlink or junction.');
    if (current === root) break;
  }
}

export async function main(args = process.argv.slice(2)) {
  if (process.platform !== 'win32') throw new Error('MSI inspection requires Windows and the already installed WiX dark.exe.');
  const options = {};
  for (let i = 0; i < args.length; i++) {
    if (!['--draft', '--native', '--expected-exe-sha256', '--nsis-source', '--wix-tools'].includes(args[i]) || !args[i + 1]) {
      throw new Error('Use --expected-exe-sha256 HASH [--draft DIR] [--native EXE] [--nsis-source DIR] [--wix-tools DIR].');
    }
    options[args[i].slice(2)] = args[++i];
  }
  const expected = options['expected-exe-sha256']?.toLowerCase();
  if (!/^[a-f0-9]{64}$/.test(expected ?? '')) throw new Error('Pin --expected-exe-sha256 to the reviewed native executable digest.');
  const draft = path.resolve(root, options.draft ?? 'target/native-release-draft');
  const native = path.resolve(root, options.native ?? 'target/native-dist/x86_64-pc-windows-msvc/release/oxbox-native.exe');
  const source = path.resolve(root, options['nsis-source'] ?? path.join(path.dirname(native), 'nsis/x64'));
  const tools = path.resolve(options['wix-tools'] ?? path.join(process.env.LOCALAPPDATA, 'tauri/WixTools314'));
  const manifestPath = path.join(draft, 'SHA256SUMS.txt');
  ordinary(manifestPath, 64 * 1024);
  const assets = verifyManifest(fs.readFileSync(manifestPath, 'utf8'));
  const msi = assets.filter(asset => asset.name.endsWith('.msi'));
  const nsis = assets.filter(asset => asset.name.endsWith('-setup.exe'));
  if (msi.length !== 1 || nsis.length !== 1) throw new Error('A Windows review draft needs exactly one MSI and one NSIS installer.');
  const errors = [];
  for (const asset of assets) {
    const actual = await sha256(path.join(draft, asset.name));
    if (actual !== asset.sha256) errors.push(`Checksum mismatch: ${asset.name}.`);
  }
  if (await sha256(native) !== expected) errors.push('Native executable does not match the pinned review digest.');
  // Never pass unverified packages to even a non-executing inspection tool.
  if (errors.length) throw new Error(errors.join('\n'));
  const parent = path.join(root, 'target/installer-verification');
  workspaceDirectory(parent);
  fs.mkdirSync(parent, { recursive: true });
  const out = fs.mkdtempSync(path.join(parent, 'run-'));
  const dark = path.join(tools, 'dark.exe');
  ordinary(dark);
  execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-File', path.join(root, 'scripts/inspect-native-msi.ps1'),
    '-Msi', path.join(draft, msi[0].name), '-Dark', dark, '-OutDirectory', out], { cwd: root, stdio: 'inherit' });
  const tables = JSON.parse(fs.readFileSync(path.join(out, 'msi-tables.json'), 'utf8'));
  errors.push(...verifyMsiTables(tables));
  const input = new Map([
    ['oxbox-native.exe', native], ['oxbox-installed.json', path.join(root, 'packaging/native/installed.json')],
    ...['LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_NOTICES.txt'].map(name => [name, path.join(draft, name)]),
  ]);
  const extracted = [];
  for (const file of tables.Files) {
    if (!input.has(file.Name) || !/^[a-zA-Z_][a-zA-Z0-9_.]*$/.test(file.Id)) continue;
    const filename = path.join(out, 'payload/File', file.Id);
    const digest = await sha256(filename);
    if (digest !== await sha256(input.get(file.Name))) errors.push(`MSI extracted payload mismatch: ${file.Name}.`);
    extracted.push({ name: file.Name, sha256: digest });
  }
  const installer = path.join(source, 'installer.nsi');
  const utilities = path.join(source, 'utils.nsh');
  ordinary(installer, 4 * 1024 * 1024); ordinary(utilities, 4 * 1024 * 1024);
  errors.push(...verifyNsisSources(fs.readFileSync(installer, 'utf8'), fs.readFileSync(utilities, 'utf8')));
  if (await sha256(path.join(source, 'nsis-output.exe')) !== nsis[0].sha256) errors.push('NSIS compiler output differs from the reviewed installer.');
  const report = {
    schema: 1, status: errors.length ? 'failed' : 'passed-static-checks', expectedExecutableSha256: expected,
    assets, extractedMsiPayload: extracted, errors,
    installerExecuted: false, msiCustomActionsExecuted: false, nsisPayloadIndependentlyExtracted: false,
    remaining: ['Independent NSIS payload extraction', 'Clean-machine install/login/upgrade/uninstall', 'Signing and human license review'],
  };
  fs.writeFileSync(path.join(out, 'report.json'), `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
  process.stdout.write(`Inspection report: ${path.relative(root, path.join(out, 'report.json'))}\n`);
  if (errors.length) throw new Error(errors.join('\n'));
  process.stdout.write('Static installer checks passed; no installer or native application was executed.\n');
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
}

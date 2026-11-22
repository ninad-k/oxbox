import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const script = join(root, 'scripts', 'observe-native-endpoints.ps1');
const supported = process.platform === 'win32';
const fixtureContents = 'Generated observer validation fixture; never execute.';
const hash = createHash('sha256').update(fixtureContents).digest('hex');

function invoke(executable, output, expectedHash = hash) {
  const result = spawnSync('powershell.exe', ['-NoProfile', '-File', script,
    '-ExpectedExecutable', executable, '-OutputDirectory', output,
    '-ExpectedSha256', expectedHash, '-DurationSeconds', '30'],
  { encoding: 'utf8', timeout: 15_000, windowsHide: true });
  assert.equal(result.error, undefined);
  assert.notEqual(result.status, 0, 'negative validation must stop before observing');
  return `${result.stdout}\n${result.stderr}`;
}

test('observer rejects non-workspace and UNC executable paths before filesystem probes', { skip: !supported }, () => {
  for (const executable of [String.raw`C:\outside-fixture\oxbox-native.exe`, String.raw`\\oxbox-fixture.invalid\share\oxbox-native.exe`]) {
    assert.match(invoke(executable, join(root, 'target', 'native-privacy-qa', 'unused-negative-fixture')), /executable must be.*target/i);
  }
});

test('observer refuses outside output, existing evidence and a mismatched hash without sampling', { skip: !supported }, () => {
  const target = join(root, 'target', 'native-privacy-qa');
  mkdirSync(target, { recursive: true });
  const fixture = mkdtempSync(join(target, 'guard-fixture-'));
  try {
    const executable = join(fixture, 'oxbox-native.exe');
    writeFileSync(executable, fixtureContents);
    assert.match(invoke(executable, join(root, 'observer-outside-fixture')), /output directory must be.*target.native-privacy-qa/i);
    assert.match(invoke(executable, fixture), /fresh output directory/i);
    assert.match(invoke(executable, join(fixture, 'never-created'), '0'.repeat(64)), /hash does not match/i);
  } finally {
    // Every removed path was created inside this exact generated fixture directory.
    assert.ok(fixture.startsWith(`${target}\\`));
    rmSync(fixture, { recursive: true, force: true });
  }
});

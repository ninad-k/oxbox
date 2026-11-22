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

test('observer validity rejects marker/query failures, incomplete windows, missing target and changed hash', { skip: !supported }, () => {
  // Parse only the pure policy function from the actual script. Dot-sourcing
  // that one AST extent never runs its sampler, hashes or P/Invoke setup.
  const program = `
    $tokens = $null; $parseErrors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($env:OXBOX_OBSERVER_TEST_SCRIPT, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count -ne 0) { throw 'Observer source must parse.' }
    $definition = $ast.Find({ param($node)
      $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Test-ObservationValidity'
    }, $true)
    if ($null -eq $definition) { throw 'Missing pure observer validity policy.' }
    . ([scriptblock]::Create($definition.Extent.Text))
    $results = @(
      (Test-ObservationValidity -CompletedNormally $true -SampleFailures 0 -MarkerFailures 0 -TargetObserved $true -HashUnchanged $true),
      (Test-ObservationValidity -CompletedNormally $true -SampleFailures 1 -MarkerFailures 0 -TargetObserved $true -HashUnchanged $true),
      (Test-ObservationValidity -CompletedNormally $true -SampleFailures 0 -MarkerFailures 1 -TargetObserved $true -HashUnchanged $true),
      (Test-ObservationValidity -CompletedNormally $false -SampleFailures 0 -MarkerFailures 0 -TargetObserved $true -HashUnchanged $true),
      (Test-ObservationValidity -CompletedNormally $true -SampleFailures 0 -MarkerFailures 0 -TargetObserved $false -HashUnchanged $true),
      (Test-ObservationValidity -CompletedNormally $true -SampleFailures 0 -MarkerFailures 0 -TargetObserved $true -HashUnchanged $false)
    )
    $results | ConvertTo-Json -Compress
  `;
  const result = spawnSync('powershell.exe', ['-NoProfile', '-EncodedCommand', Buffer.from(program, 'utf16le').toString('base64')], {
    encoding: 'utf8', timeout: 15_000, windowsHide: true,
    env: { ...process.env, OXBOX_OBSERVER_TEST_SCRIPT: script },
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), [true, false, false, false, false, false]);
});

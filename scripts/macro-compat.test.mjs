import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { assertMacroResult, assertMacroSource } from './check-macro-compat.mjs';

test('macro evidence requires a real valid-runtime marker and expected compile diagnostics', () => {
  assert.doesNotThrow(() => assertMacroResult('valid', { status: 0, stdout: 'OXBOX_MACRO_VALID_PASS\n', stderr: '' }));
  assert.throws(() => assertMacroResult('valid', { status: 0, stdout: '', stderr: '' }), /valid.*marker/i);
  assert.doesNotThrow(() => assertMacroResult('missing_named_argument', { status: 101, stdout: '', stderr: 'error: fl!() the following arguments have not been specified: `place`' }));
  assert.throws(() => assertMacroResult('missing_named_argument', { status: 0, stdout: '', stderr: '' }), /unexpectedly succeeded/i);
});

test('macro evidence refuses build panics and unrelated failures instead of counting them as expected errors', () => {
  for (const stderr of ['error: proc macro panicked', "thread 'main' panicked", 'error: could not find Cargo.toml', 'error: failed to download dependency']) {
    assert.throws(() => assertMacroResult('unknown_message', { status: 101, stdout: '', stderr }), /panic|expected diagnostic/i);
  }
  assert.throws(() => assertMacroResult('unknown_message', { status: null, error: new Error('timed out'), stdout: '', stderr: '' }), /process failed/i);
});

test('mixed expected diagnostics cannot hide attribute or derive macro panics', () => {
  for (const panic of ['error: custom attribute panicked', 'error: proc-macro derive panicked']) {
    const stderr = `error: fl!() message_id does not exist\n${panic}\n`;
    assert.throws(() => assertMacroResult('unknown_message', { status: 101, stdout: '', stderr }), /panic/i);
    assert.throws(() => assertMacroResult('valid', { status: 0, stdout: 'OXBOX_MACRO_VALID_PASS\n', stderr: panic }), /panic/i);
  }
  assert.doesNotThrow(() => assertMacroResult('unknown_message', {
    status: 101, stdout: '',
    stderr: 'warning: generated source comment mentions panicked and quotes `error: custom attribute panicked`\n// error: proc-macro derive panicked\nerror: fl!() message_id does not exist',
  }));
});

test('the patched macro source must bind to the exact reviewed vendor directory', () => {
  const expected = path.resolve('vendor/i18n-embed-fl-0.9.4');
  const other = path.resolve('target/another-checkout/vendor/i18n-embed-fl-0.9.4');
  const tree = directory => `i18n-embed-fl v0.9.4 (proc-macro) (${directory})\n`;
  assert.doesNotThrow(() => assertMacroSource(tree(expected), 'patched', expected));
  assert.throws(() => assertMacroSource(tree(other), 'patched', expected), /directory|source selection/i);
  if (process.platform === 'win32') {
    assert.doesNotThrow(() => assertMacroSource(tree(expected.toUpperCase()), 'patched', expected));
  } else {
    assert.throws(() => assertMacroSource(tree(expected.toUpperCase()), 'patched', expected), /directory|source selection/i);
  }
});

test('macro evidence distinguishes the original registry package from the approved path package', () => {
  assert.doesNotThrow(() => assertMacroSource('i18n-embed-fl v0.9.4\n', 'original'));
  assert.throws(() => assertMacroSource('i18n-embed-fl v0.9.4\n', 'patched'), /source selection/i);
  const patched = 'i18n-embed-fl v0.9.4 (D:\\project\\vendor\\i18n-embed-fl-0.9.4)\n';
  assert.doesNotThrow(() => assertMacroSource(patched, 'patched'));
  assert.throws(() => assertMacroSource(patched, 'original'), /source selection/i);
});

test('macro source evidence accepts Cargo proc-macro kind annotations separately from source paths', () => {
  assert.doesNotThrow(() => assertMacroSource('i18n-embed-fl v0.9.4 (proc-macro)\n', 'original'));
  assert.doesNotThrow(() => assertMacroSource('i18n-embed-fl v0.9.4 (proc-macro) (/project/vendor/i18n-embed-fl-0.9.4)\n', 'patched'));
  assert.throws(() => assertMacroSource('i18n-embed-fl v0.9.4 (proc-macro) (/project/unapproved)\n', 'patched'), /source selection/i);
  assert.throws(() => assertMacroSource('i18n-embed-fl v0.9.4 (/project/proc-macro)\n', 'original'), /source selection/i);
});

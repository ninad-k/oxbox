import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripVTControlCharacters } from 'node:util';

const diagnostics = {
  unknown_message: /message_id.*does not exist/i,
  missing_named_argument: /following arguments have not been specified:.*place/i,
  unexpected_named_argument: /argument.*extra.*does not exist in the fluent message/i,
  malformed_input: /unable to parse args input|expected (?:an )?expression/i,
};

/** Judge generated compiler outcomes without treating crashes/setup failures as evidence. */
export function assertMacroResult(name, result) {
  if (result.error || result.status === null) throw new Error(`Macro fixture process failed: ${name}`);
  const output = stripVTControlCharacters(`${result.stdout ?? ''}\n${result.stderr ?? ''}`);
  const panic = /^\s*(?:error(?:\[[^\]\r\n]+\])?:\s*(?:(?:proc[ -]macro(?:\s+derive)?|custom\s+(?:attribute|derive))\s+panicked\b|internal compiler error\b)|thread\b[^\r\n]*\bpanicked\b|internal compiler error\b)/im;
  if (panic.test(output)) {
    throw new Error(`Macro fixture produced a build panic: ${name}`);
  }
  const text = output.replace(/\s+/g, ' ');
  if (name === 'valid') {
    if (result.status !== 0 || !text.includes('OXBOX_MACRO_VALID_PASS')) {
      throw new Error('Valid macro fixture failed or its valid-runtime marker is missing.');
    }
    return;
  }
  if (!Object.hasOwn(diagnostics, name)) throw new Error('Unknown macro compatibility fixture.');
  if (result.status === 0) throw new Error(`Invalid macro fixture unexpectedly succeeded: ${name}`);
  if (!diagnostics[name].test(text)) throw new Error(`Macro fixture did not emit the expected diagnostic: ${name}`);
}

/** Keep original and patched comparison runs distinct without disclosing local paths. */
export function assertMacroSource(tree, expected, expectedDirectory) {
  if (!['original', 'patched'].includes(expected)) throw new Error('Expected original or patched source selection.');
  const line = tree.trim().replaceAll('\\', '/')
    .replace(/^(i18n-embed-fl v0\.9\.4) \(proc-macro\)(?= |$)/, '$1');
  const selected = /^i18n-embed-fl v0\.9\.4(?: \(([^\r\n]+)\))?$/.exec(line);
  if (!selected) {
    throw new Error('Unexpected macro package source selection.');
  }
  const sourceDirectory = selected[1];
  const directoryKey = directory => {
    const resolved = path.resolve(directory);
    return process.platform === 'win32' ? resolved.toLowerCase() : resolved;
  };
  const directoryText = process.platform === 'win32' ? sourceDirectory?.toLowerCase() : sourceDirectory;
  const patched = typeof directoryText === 'string' && /\/vendor\/i18n-embed-fl-0\.9\.4$/.test(directoryText);
  if (patched !== (expected === 'patched') || (expected === 'original' && line.includes(' ('))) {
    throw new Error('Macro package source selection does not match this comparison run.');
  }
  if (expected === 'patched' && expectedDirectory !== undefined &&
      (typeof expectedDirectory !== 'string' || !expectedDirectory ||
       directoryKey(sourceDirectory) !== directoryKey(expectedDirectory))) {
    throw new Error('Macro package source directory differs from the exact reviewed vendor directory.');
  }
}

function run(expected) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const manifest = path.join(root, 'tests', 'macro-compat', 'Cargo.toml');
  const common = ['--manifest-path', manifest, '--locked', '--offline'];
  const invoke = args => spawnSync('cargo', ['+1.95.0', ...args], {
    cwd: root, encoding: 'utf8', timeout: 300_000, maxBuffer: 4 * 1024 * 1024,
    windowsHide: true,
  });
  const selected = invoke(['tree', ...common, '-p', 'i18n-embed-fl', '--depth', '0', '--prefix', 'none', '--format', '{p}']);
  if (selected.error || selected.status !== 0) throw new Error('Locked offline macro source selection failed. Resolve this isolated workspace first.');
  assertMacroSource(selected.stdout, expected, path.join(root, 'vendor', 'i18n-embed-fl-0.9.4'));
  const results = [];
  for (const name of ['valid', ...Object.keys(diagnostics)]) {
    const result = invoke([name === 'valid' ? 'run' : 'check', ...common, '--bin', name]);
    try {
      assertMacroResult(name, result);
    } catch (error) {
      throw new Error(`${error.message}\n${`${result.stdout ?? ''}\n${result.stderr ?? ''}`.slice(-12_000)}`);
    }
    results.push({ fixture: name, outcome: name === 'valid' ? 'runtime passed' : 'expected compile diagnostic; no build panic' });
  }
  console.log(JSON.stringify({ package: 'i18n-embed-fl 0.9.4', source: expected, generatedOnly: true, results }, null, 2));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  try {
    if (args.length !== 2 || args[0] !== '--expect' || !['original', 'patched'].includes(args[1])) {
      throw new Error('Usage: node scripts/check-macro-compat.mjs --expect original|patched (invokes serial offline Cargo commands)');
    }
    run(args[1]);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}

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

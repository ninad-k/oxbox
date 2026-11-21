/** Helpers for the Verify Checksum view. Mirrors `digests_match` in oxbox-core. */

export function normalizeDigest(s: string): string {
  return s.replace(/\s+/g, '').toLowerCase();
}

export function digestsMatch(computed: string, expected: string): boolean {
  const want = normalizeDigest(expected);
  return want.length > 0 && normalizeDigest(computed) === want;
}

/** Last path segment, for either separator. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

import { describe, expect, it } from 'vitest';
import { digestsMatch, fileName } from './checksum';

describe('digestsMatch', () => {
  it('ignores case and whitespace', () => {
    expect(digestsMatch('abcdef', '  ABC DEF\n')).toBe(true);
  });
  it('rejects different digests and empty input', () => {
    expect(digestsMatch('abcdef', 'abcdee')).toBe(false);
    expect(digestsMatch('abcdef', '   ')).toBe(false);
  });
});

describe('fileName', () => {
  it('handles Windows and POSIX paths', () => {
    expect(fileName('C:\\Users\\me\\setup.exe')).toBe('setup.exe');
    expect(fileName('/home/me/archive.tar.gz')).toBe('archive.tar.gz');
  });
});

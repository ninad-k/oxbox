import { expect, it } from 'vitest';
import { formatBytes } from './system-api';
it('formats fractional rates, binary units, and absent values', () => {
  expect(formatBytes(0.5)).toBe('1 B');
  expect(formatBytes(1024)).toBe('1.0 KiB');
  expect(formatBytes(Number.NaN)).toBe('0 B');
  expect(formatBytes(-1)).toBe('0 B');
});

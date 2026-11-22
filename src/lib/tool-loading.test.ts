import { describe, expect, it } from 'vitest';
import { describeToolLoadError } from './tool-loading';

describe('tool loading recovery', () => {
  it('offers explicit recovery for a failed browser module download', () => {
    for (const message of [
      'Failed to fetch dynamically imported module: http://127.0.0.1:4174/_app/immutable/chunks/example.js',
      'error loading dynamically imported module: http://localhost/module.js',
      'Importing a module script failed.',
    ]) {
      const result = describeToolLoadError(new TypeError(message));
      expect(result.reload).toBe(true);
      expect(result.message).toContain('preview');
      expect(result.message).toContain('unsaved');
    }
  });

  it('preserves unexpected errors rather than claiming every failure is a stale build', () => {
    for (const error of [new Error('Invalid component'), new TypeError('Cannot read properties of undefined'), 'unknown']) {
      const result = describeToolLoadError(error);
      expect(result.reload).toBe(false);
      expect(result.message).toContain(String(error));
    }
  });
});

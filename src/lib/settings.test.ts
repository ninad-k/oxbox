import { describe, it, expect } from 'vitest';
import { sanitizePreferences } from './settings';
describe('preference recovery', () => {
  it('recovers invalid stores and filters untrusted lists', () => {
    expect(sanitizePreferences(null).theme).toBe('system');
    expect(sanitizePreferences({theme: 'neon', favorites: ['a', 'a', null, 2, 'b']}).favorites).toEqual(['a','b']);
    expect(sanitizePreferences({worldZones: []}).worldZones).toEqual([]);
  });
});

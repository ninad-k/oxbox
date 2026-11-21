import { describe, expect, it } from 'vitest';
import { defaultView, favoriteShortcut } from './launcher';
import { filterTools } from './search';
import { tools } from './registry';

describe('personal dashboard', () => {
  it('starts with only explicitly saved favorites, including an empty dashboard', () => {
    expect(defaultView).toBe('favorites');
    expect(filterTools(tools, { view: defaultView, query: '', favorites: [] })).toEqual([]);
    expect(filterTools(tools, { view: defaultView, query: '', favorites: ['qr-code'] }).map(t => t.id)).toEqual(['qr-code']);
  });
  it('opens favorite slots only for the dedicated Alt+number chord', () => {
    const modifiers = { alt: true, ctrl: false, meta: false, shift: false };
    expect(favoriteShortcut('Digit2', modifiers, ['qr-code', 'world-time'])).toBe('world-time');
    expect(favoriteShortcut('Digit3', modifiers, ['qr-code'])).toBeNull();
    expect(favoriteShortcut('Digit0', modifiers, ['qr-code'])).toBeNull();
    expect(favoriteShortcut('Digit1', { ...modifiers, alt: false }, ['qr-code'])).toBeNull();
    expect(favoriteShortcut('Digit1', { ...modifiers, ctrl: true }, ['qr-code'])).toBeNull();
    expect(favoriteShortcut('Digit1', { ...modifiers, shift: true }, ['qr-code'])).toBeNull();
  });
  it('supports accessible digit keys only when the physical code is unavailable', () => {
    const modifiers = { alt: true, ctrl: false, meta: false, shift: false };
    const ids = Array.from({ length: 9 }, (_, index) => `favorite-${index + 1}`);
    for (const code of ['', 'Unidentified']) {
      for (let digit = 1; digit <= 9; digit++) {
        expect(favoriteShortcut(code, modifiers, ids, String(digit))).toBe(`favorite-${digit}`);
      }
      for (const key of ['', '0', '10', 'ArrowUp', '１']) expect(favoriteShortcut(code, modifiers, ids, key)).toBeNull();
      expect(favoriteShortcut(code, { ...modifiers, alt: false }, ids, '1')).toBeNull();
      expect(favoriteShortcut(code, { ...modifiers, ctrl: true }, ids, '1')).toBeNull();
      expect(favoriteShortcut(code, { ...modifiers, meta: true }, ids, '1')).toBeNull();
      expect(favoriteShortcut(code, { ...modifiers, shift: true }, ids, '1')).toBeNull();
    }
  });
  it('preserves physical digit precedence and rejects numpad or other known codes', () => {
    const modifiers = { alt: true, ctrl: false, meta: false, shift: false };
    const ids = ['qr-code', 'world-time'];
    expect(favoriteShortcut('Digit2', modifiers, ids, '1')).toBe('world-time');
    for (const code of ['Numpad1', 'KeyQ', 'Digit0', 'Digit10', 'Space', 'unidentified']) {
      expect(favoriteShortcut(code, modifiers, ids, '1')).toBeNull();
    }
    expect(favoriteShortcut('', modifiers, [], '1')).toBeNull();
  });
});

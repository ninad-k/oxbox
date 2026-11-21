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
});

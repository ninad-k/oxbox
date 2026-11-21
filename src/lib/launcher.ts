import type { View } from './search';
export const defaultView: View = 'favorites';
export function favoriteShortcut(code: string, modifiers: {alt: boolean; ctrl: boolean; meta: boolean; shift: boolean}, ids: readonly string[], key = ''): string | null {
  if (!modifiers.alt || modifiers.ctrl || modifiers.meta || modifiers.shift) return null;
  const digit = /^Digit[1-9]$/.test(code)
    ? code.slice(-1)
    : (code === '' || code === 'Unidentified') && /^[1-9]$/.test(key) ? key : null;
  return digit === null ? null : ids[Number(digit) - 1] ?? null;
}

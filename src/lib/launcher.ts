import type { View } from './search';
export const defaultView: View = 'favorites';
export function favoriteShortcut(code: string, modifiers: {alt: boolean; ctrl: boolean; meta: boolean; shift: boolean}, ids: readonly string[]): string | null {
  if (!modifiers.alt || modifiers.ctrl || modifiers.meta || modifiers.shift || !/^Digit[1-9]$/.test(code)) return null;
  return ids[Number(code.slice(-1)) - 1] ?? null;
}

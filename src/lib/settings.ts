export type Theme = 'system' | 'light' | 'dark';
export interface Preferences { theme: Theme; favorites: string[]; worldZones: string[] }
export const defaults: Preferences = { theme: 'system', favorites: [], worldZones: ['UTC', 'Asia/Dubai', 'Europe/London', 'America/New_York'] };
export function sanitizePreferences(input: unknown): Preferences {
  const value = input && typeof input === 'object' ? input as Partial<Preferences> : {};
  const strings = (x: unknown, fallback: string[]) => Array.isArray(x)
    ? [...new Set(x.filter((s): s is string => typeof s === 'string' && s.length > 0 && s.length < 150))].slice(0, 200) : [...fallback];
  return { theme: value.theme === 'dark' || value.theme === 'light' ? value.theme : 'system',
    favorites: strings(value.favorites, defaults.favorites), worldZones: strings(value.worldZones, defaults.worldZones) };
}

/**
 * Favourite tools, kept in this machine's webview storage.
 * TODO(phase 1): move to tauri-plugin-store together with the other settings.
 */
const KEY = 'oxbox.favorites';

function load(): string[] {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

class Favorites {
  ids = $state<string[]>(load());

  has(id: string): boolean {
    return this.ids.includes(id);
  }

  toggle(id: string): void {
    this.ids = this.has(id) ? this.ids.filter((x) => x !== id) : [...this.ids, id];
    try {
      globalThis.localStorage?.setItem(KEY, JSON.stringify(this.ids));
    } catch {
      // Storage can be unavailable (private mode, quota); favourites then last for the session.
    }
  }
}

export const favorites = new Favorites();

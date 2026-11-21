import { settings } from './settings.svelte';
class Favorites {
  get ids() { return settings.data.favorites; }
  has(id: string) { return this.ids.includes(id); }
  toggle(id: string) {
    void settings.update(current => ({favorites: current.favorites.includes(id) ? current.favorites.filter(x => x !== id) : [...current.favorites, id]})).catch(() => {});
  }
}
export const favorites = new Favorites();

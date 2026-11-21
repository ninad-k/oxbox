import { load, type Store } from '@tauri-apps/plugin-store';
import { inDesktopApp } from './ipc';
import { sanitizePreferences, type Preferences } from './settings';
const browserKey = 'oxbox.settings';
export class Settings {
  data = $state<Preferences>(sanitizePreferences(null));
  ready = $state(false);
  error = $state('');
  private store?: Store;
  private initialization?: Promise<void>;
  private writes = Promise.resolve();
  private persisted = sanitizePreferences(null);
  private revision = 0;
  init(): Promise<void> { return this.initialization ??= this.initialize(); }
  private async initialize() {
    try {
      let raw: unknown;
      if (inDesktopApp()) {
        this.store = await load('settings.json', {autoSave: false, defaults: {}});
        raw = await this.store.get('preferences');
      } else { raw = JSON.parse(localStorage.getItem(browserKey) ?? 'null'); }
      const data = sanitizePreferences(raw);
      const legacy = localStorage.getItem('oxbox.favorites');
      if (raw == null && legacy) data.favorites = sanitizePreferences({favorites: JSON.parse(legacy)}).favorites;
      this.data = data;
      this.persisted = sanitizePreferences(data);
      if (legacy && raw == null) {
        if (this.store) await this.saveNativePreferences(data);
        else localStorage.setItem(browserKey, JSON.stringify(data));
        localStorage.removeItem('oxbox.favorites');
      }
    } catch (e) { this.error = `Could not load settings: ${String(e)}`; }
    this.ready = true; this.applyTheme();
  }
  applyTheme() {
    if (typeof document === 'undefined') return;
    if (this.data.theme === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = this.data.theme;
  }
  private async saveNativePreferences(snapshot: Preferences) {
    if (!this.store) throw new Error('The desktop settings store is unavailable. Restart Oxbox and try again.');
    const previous = await this.store.get('preferences');
    try { await this.store.set('preferences', snapshot); await this.store.save(); }
    catch (error) {
      // Null preserves the original absent/default semantics without requiring a wider delete permission.
      try { await this.store.set('preferences', previous ?? null); await this.store.save(); }
      catch (recovery) { throw new Error(`${String(error)}; restoring the previous settings also failed: ${String(recovery)}`); }
      throw error;
    }
  }
  async update(patch: Partial<Preferences> | ((current: Preferences) => Partial<Preferences>)): Promise<void> {
    await this.init();
    // Resolve list edits after loading and against the latest optimistic value.
    const current = $state.snapshot(this.data);
    this.data = sanitizePreferences({...current, ...(typeof patch === 'function' ? patch(current) : patch)});
    const revision = ++this.revision;
    this.applyTheme();
    const snapshot = $state.snapshot(this.data);
    const write = this.writes.catch(() => {}).then(async () => {
      if (this.store) await this.saveNativePreferences(snapshot);
      else if (!inDesktopApp()) localStorage.setItem(browserKey, JSON.stringify(snapshot));
      else throw new Error('The desktop settings store is unavailable. Restart Oxbox and try again.');
      this.persisted = sanitizePreferences(snapshot);
    });
    this.writes = write;
    try { await write; if (revision === this.revision) this.error = ''; }
    catch (e) {
      if (revision === this.revision) {
        this.data = sanitizePreferences(this.persisted);
        this.applyTheme();
        this.error = `Could not save settings: ${String(e)}`;
      }
      throw e;
    }
  }
}
export const settings = new Settings();

import { beforeEach, expect, it, vi } from 'vitest';
const mocks = vi.hoisted(() => ({ load: vi.fn(), desktop: vi.fn(() => true) }));
vi.mock('@tauri-apps/plugin-store', () => ({ load: mocks.load }));
vi.mock('./ipc', () => ({ inDesktopApp: mocks.desktop }));
import { Settings } from './settings.svelte';

beforeEach(() => {
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key, value), removeItem: (key: string) => values.delete(key) });
  mocks.load.mockReset(); mocks.desktop.mockReturnValue(true);
});

it('applies rapid favorite edits to loaded preferences without losing either change', async () => {
  let resolve!: (value: unknown) => void;
  const loaded = new Promise(resolveValue => { resolve = resolveValue; });
  const store = { get: vi.fn(() => loaded), set: vi.fn(), save: vi.fn() };
  mocks.load.mockResolvedValue(store);
  const settings = new Settings();
  const first = settings.update(current => ({ favorites: [...current.favorites, 'first'] }));
  const second = settings.update(current => ({ favorites: [...current.favorites, 'second'] }));
  resolve({ theme: 'dark', favorites: ['existing'] });
  await Promise.all([first, second]);
  expect(settings.data.favorites).toEqual(['existing', 'first', 'second']);
  expect(store.set.mock.calls.at(-1)).toEqual(['preferences', expect.objectContaining({ favorites: ['existing', 'first', 'second'], theme: 'dark' })]);
});

it('migrates legacy favorites only after the native store saves successfully', async () => {
  localStorage.setItem('oxbox.favorites', '["old"]');
  const store = { get: vi.fn(), set: vi.fn(), save: vi.fn().mockRejectedValue(new Error('disk full')) };
  mocks.load.mockResolvedValue(store);
  const settings = new Settings();
  await settings.init();
  expect(localStorage.getItem('oxbox.favorites')).toBe('["old"]');
  expect(settings.error).toContain('disk full');
});

it('does not silently claim desktop settings were saved after the native store fails', async () => {
  mocks.load.mockRejectedValue(new Error('store unavailable'));
  const settings = new Settings();
  await expect(settings.update({ theme: 'light' })).rejects.toThrow('desktop settings store');
  expect(settings.error).toContain('Could not save settings');
  expect(localStorage.getItem('oxbox.settings')).toBeNull();
});

it('rolls back failed preference writes in the native cache and visible theme', async () => {
  let cached: unknown={theme:'dark',favorites:['existing'],worldZones:['UTC']};
  const store={get:vi.fn(async()=>cached),set:vi.fn(async(_key:string,value:unknown)=>{cached=value;}),save:vi.fn().mockRejectedValueOnce(new Error('disk full')).mockResolvedValue(undefined)};
  mocks.load.mockResolvedValue(store);
  const settings=new Settings();await settings.init();
  await expect(settings.update({theme:'light'})).rejects.toThrow('disk full');
  expect(settings.data.theme).toBe('dark');
  expect(cached).toEqual(expect.objectContaining({theme:'dark'}));
  await settings.update(current=>({favorites:[...current.favorites,'new']}));
  expect(cached).toEqual(expect.objectContaining({theme:'dark',favorites:['existing','new']}));
});

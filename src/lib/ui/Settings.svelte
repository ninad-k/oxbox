<script lang="ts">
  import { onMount } from 'svelte';
  import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { api, inDesktopApp } from '#lib/api.ts';
  import { settings } from '#lib/settings.svelte.ts';
  import type { Theme } from '#lib/settings.ts';
  import Notice from './Notice.svelte';
  let { onback }: {onback: () => void} = $props();
  let login = $state(false), busy = $state(false), shortcut = $state(''), message = $state(''), error = $state('');
  onMount(() => {
    let disposed = false;
    void settings.init();
    if (inDesktopApp()) Promise.all([isEnabled(), api.shortcutStatus()]).then(([enabled, status]) => {
      if (disposed) return; login = enabled; shortcut = status.shortcut; error = status.error ?? '';
    }).catch(e => { if (!disposed) error = String(e); });
    return () => { disposed = true; };
  });
  async function changeTheme(theme: Theme) {
    try { await settings.update({theme}); message = 'Theme saved.'; error = ''; }
    catch (e) { error = String(e); }
  }
  async function changeLogin(checked: boolean) {
    busy = true; error = ''; message = '';
    try { if (checked) await enable(); else await disable(); login = await isEnabled(); message = 'Launch at login updated.'; }
    catch (e) { error = String(e); }
    finally { busy = false; }
  }
  async function saveShortcut() {
    busy = true; error = ''; message = '';
    try { shortcut = await api.configureShortcut(shortcut); message = 'Global shortcut saved.'; }
    catch (e) { error = String(e); }
    finally { busy = false; }
  }
</script>
<section class="settings" aria-busy={busy}>
  <div class="row"><button class="btn" onclick={onback}>Back to tools</button><h1>Settings</h1></div>
  <label class="field"><span class="field-label">Appearance</span>
    <select class="input" value={settings.data.theme} onchange={e => changeTheme(e.currentTarget.value as Theme)}>
      <option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option>
    </select>
  </label>
  <label><input type="checkbox" checked={login} disabled={busy || !inDesktopApp()} onchange={e => changeLogin(e.currentTarget.checked)} /> Launch Oxbox at login</label>
  <form class="field" onsubmit={e => {e.preventDefault(); void saveShortcut();}}>
    <label class="field"><span class="field-label">Global launcher shortcut</span><input class="input" bind:value={shortcut} placeholder="CommandOrControl+Alt+Space" disabled={!inDesktopApp()} /></label>
    <p>Use modifier names and a key, for example Control+Alt+Space or Super+Alt+Space on macOS.</p>
    <button class="btn" disabled={busy || !shortcut.trim() || !inDesktopApp()}>Save shortcut</button>
  </form>
  <Notice kind="info">Settings and favorites stay on this computer. Clipboard history is kept in memory. Oxbox sends no telemetry.</Notice>
  {#if !inDesktopApp()}<Notice kind="info">Launch at login and global shortcuts are available in the desktop app.</Notice>{/if}
  {#if message}<Notice kind="success">{message}</Notice>{/if}
  {#if error || settings.error}<Notice kind="error">{error || settings.error}</Notice>{/if}
</section>
<style>.settings{display:grid;gap:var(--s-5);padding:var(--s-6);max-width:720px}.row{display:flex;align-items:center;gap:var(--s-4)}p{font-size:var(--text-sm);color:var(--text-2)}</style>

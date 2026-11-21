<script lang="ts">
  import { onMount } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { mediaApi, type ClipboardSnapshot } from '#lib/media-api.ts';
  import Notice from '#lib/ui/Notice.svelte';

  let snapshot = $state<ClipboardSnapshot | null>(null);
  let query = $state('');
  let busy = $state(false);
  let error = $state('');
  let message = $state('');
  let requestId = 0;
  let refreshing = false;
  let mounted = false;

  async function refresh() {
    if (refreshing) return;
    refreshing = true;
    const id = ++requestId;
    try { const next = await mediaApi.clipboardSnapshot(query); if (id === requestId && mounted) { snapshot = next; error = ''; } }
    catch (cause) { if (id === requestId && mounted) error = String(cause); }
    finally { refreshing = false; }
  }

  async function perform(work: () => Promise<void>, success = '') {
    busy = true; error = ''; message = '';
    try { await work(); if (mounted) { message = success; await refresh(); } }
    catch (cause) { if (mounted) error = String(cause); }
    finally { if (mounted) busy = false; }
  }

  async function setPaused(paused: boolean) {
    if (!paused && snapshot?.paused) {
      const accepted = await ask('Clipboard history can contain passwords and other sensitive text or images. Start recording locally in memory? History is lost when Oxbox quits.', { title: 'Start clipboard history?', kind: 'warning' });
      if (!accepted) return;
    }
    await perform(() => mediaApi.clipboardSetPaused(paused), paused ? 'Clipboard recording paused.' : 'Clipboard recording started.');
  }

  async function remove(id?: number) {
    const accepted = await ask(id === undefined ? 'Clear all clipboard history, including pinned entries? This does not clear the system clipboard.' : 'Remove this entry from clipboard history?', { title: 'Remove clipboard history?', kind: 'warning' });
    if (accepted) await perform(() => id === undefined ? mediaApi.clipboardClear() : mediaApi.clipboardDelete(id), 'History updated.');
  }

  onMount(() => {
    mounted = true;
    void refresh();
    const interval = setInterval(() => { if (!busy) void refresh(); }, 1000);
    return () => { mounted = false; requestId++; clearInterval(interval); };
  });
  $effect(() => { query; if (mounted) void refresh(); });
</script>

<div class="tool" aria-busy={busy}>
  <Notice kind="info">History is paused until you choose Start. It stays in memory, with a limit of 200 entries or 50 MB including pinned entries. It is never saved to disk. Windows content marked as concealed is skipped; other apps may expose secrets without a privacy marker.</Notice>
  <div class="row">
    <button class="btn btn-primary" disabled={busy || !snapshot} onclick={() => setPaused(!snapshot?.paused)}>{snapshot?.paused ? 'Start recording' : 'Pause recording'}</button>
    <button class="btn" disabled={busy || !snapshot?.totalItems} onclick={() => remove()}>Clear history</button>
    <span>{snapshot ? `${snapshot.paused ? 'Paused' : 'Recording'} · ${snapshot.totalItems} entries · ${(snapshot.bytes / 1024 / 1024).toFixed(1)} MB` : 'Loading history…'}</span>
  </div>
  <label class="field"><span class="field-label">Search history</span><input class="input" type="search" bind:value={query} placeholder="Search copied text" /></label>
  {#if snapshot?.items.length === 0}<Notice kind="info">{query ? 'No entries match your search.' : 'No clipboard history yet. Start recording, then copy text or an image in another app.'}</Notice>{/if}
  {#each snapshot?.items ?? [] as item (item.id)}
    <article class="card item">
      <div class="row"><strong>{item.kind === 'image' ? 'Image' : 'Text'}{item.pinned ? ' · Pinned' : ''}</strong><time datetime={new Date(item.capturedAt).toISOString()}>{new Date(item.capturedAt).toLocaleTimeString()}</time><span>{Math.ceil(item.bytes / 1024)} KB</span></div>
      <p class="preview">{item.preview}</p>
      <div class="row"><button class="btn btn-primary" disabled={busy} onclick={() => perform(() => mediaApi.clipboardCopy(item.id), 'Copied. Switch to another app and paste.')}>Copy back</button><button class="btn" disabled={busy} onclick={() => perform(() => mediaApi.clipboardPin(item.id, !item.pinned))}>{item.pinned ? 'Unpin' : 'Pin'}</button><button class="btn" disabled={busy} onclick={() => remove(item.id)}>Remove</button></div>
    </article>
  {/each}
  {#if message}<Notice kind="success">{message}</Notice>{/if}
  {#if snapshot?.error}<Notice kind="error">{snapshot.error}</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>

<style>
  .tool { display: grid; gap: var(--s-4); }
  .row { display: flex; gap: var(--s-3); align-items: center; flex-wrap: wrap; }
  .row span, time { color: var(--text-3); font-size: var(--text-sm); }
  .item { display: grid; gap: var(--s-3); padding: var(--s-4); }
  .preview { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 12em; overflow-y: auto; }
</style>

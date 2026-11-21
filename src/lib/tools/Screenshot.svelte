<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { inDesktopApp } from '#lib/ipc.ts';
  import { mediaApi, type CaptureEvent, type CaptureResult, type CaptureTargetInfo } from '#lib/media-api.ts';
  import Notice from '#lib/ui/Notice.svelte';

  let targets = $state<CaptureTargetInfo[]>([]);
  let target = $state('');
  let mode = $state<'target' | 'area'>('target');
  let delay = $state(0);
  let outputDir = $state<string | null>(null);
  let result = $state<CaptureResult | null>(null);
  let busy = $state(false);
  let error = $state('');
  let requestId = 0;

  async function refresh() {
    const id = ++requestId;
    busy = true; error = '';
    try {
      const next = await mediaApi.captureTargets();
      if (id !== requestId) return;
      targets = next;
      if (!next.some((item) => `${item.kind}:${item.id}` === target)) target = next[0] ? `${next[0].kind}:${next[0].id}` : '';
    } catch (cause) { if (id === requestId) error = String(cause); }
    finally { if (id === requestId) busy = false; }
  }

  async function chooseFolder() {
    const id = requestId;
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Screenshot output folder' });
      if (id === requestId && typeof selected === 'string') outputDir = selected;
    } catch (cause) { if (id === requestId) error = String(cause); }
  }

  async function capture() {
    const id = ++requestId;
    busy = true; error = ''; result = null;
    let areaOpened = false;
    try {
      if (mode === 'area') { await mediaApi.startAreaCapture(delay, outputDir); areaOpened = true; return; }
      const selected = targets.find((item) => `${item.kind}:${item.id}` === target);
      if (!selected) throw new Error('Choose a display or window.');
      const next = await mediaApi.captureScreenshot({ kind: selected.kind, id: selected.id }, delay, outputDir);
      if (id === requestId) result = next;
    } catch (cause) { if (id === requestId) error = String(cause); }
    finally { if (id === requestId && !areaOpened) busy = false; }
  }

  onMount(() => {
    let stopped = false;
    let unlisten: (() => void) | undefined;
    if (inDesktopApp()) {
      void refresh();
      void listen<CaptureEvent>('screenshot://complete', ({ payload }) => {
        if (stopped) return;
        result = payload.result; error = payload.error ?? ''; busy = false;
      }).then((stop) => { if (stopped) stop(); else unlisten = stop; }).catch((cause) => { error = String(cause); });
    } else error = 'Open Oxbox as the desktop app to capture the screen.';
    return () => { stopped = true; requestId++; unlisten?.(); };
  });
</script>

<div class="tool" aria-busy={busy}>
  <Notice kind="info">Captures stay on this computer. Oxbox hides its launcher before capturing. Some apps and Wayland sessions restrict capture.</Notice>
  <label class="field"><span class="field-label">Capture mode</span><select class="input" bind:value={mode} disabled={busy}><option value="target">Display or window</option><option value="area">Select an area</option></select></label>
  {#if mode === 'target'}
    <label class="field"><span class="field-label">Display or window</span><select class="input" bind:value={target} disabled={busy}>{#each targets as item}<option value="{item.kind}:{item.id}">{item.kind === 'monitor' ? 'Display' : 'Window'}: {item.name} ({item.width} × {item.height})</option>{/each}</select></label>
    <button class="btn" onclick={refresh} disabled={busy}>Refresh displays and windows</button>
  {/if}
  <label class="field"><span class="field-label">Delay</span><select class="input" bind:value={delay} disabled={busy}>{#each [0, 3, 5, 10] as seconds}<option value={seconds}>{seconds} seconds</option>{/each}</select></label>
  <div class="row"><p class="path">{outputDir ?? 'Pictures / Oxbox'}</p><button class="btn" onclick={chooseFolder} disabled={busy}>Choose output folder</button></div>
  <button class="btn btn-primary" onclick={capture} disabled={busy || (mode === 'target' && !target)}>{busy ? 'Capturing…' : mode === 'area' ? 'Select area' : 'Capture'}</button>
  {#if result}<Notice kind="success">Saved {result.width} × {result.height} capture to {result.path}. {result.clipboardError ? 'The file was saved, but copying to the clipboard failed.' : 'Copied to the clipboard.'}</Notice>{/if}
  {#if result?.clipboardError}<Notice kind="error">{result.clipboardError}</Notice>{/if}
  <button class="btn" onclick={() => mediaApi.openScreenshotFolder().catch((cause) => { error = String(cause); })}>Open Pictures / Oxbox</button>
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>

<style>
  .tool { display: grid; gap: var(--s-4); }
  .row { display: flex; align-items: center; gap: var(--s-3); flex-wrap: wrap; }
  .path { overflow-wrap: anywhere; color: var(--text-2); }
  .btn { justify-self: start; }
</style>

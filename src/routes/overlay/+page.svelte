<script lang="ts">
  import { onMount } from 'svelte';
  import { mediaApi } from '#lib/media-api.ts';
  import Notice from '#lib/ui/Notice.svelte';

  let start = $state<{ x: number; y: number } | null>(null);
  let end = $state<{ x: number; y: number } | null>(null);
  let busy = $state(false);
  let error = $state('');
  let frameUrl = $state('');
  const selection = $derived(start && end ? {
    x: Math.min(start.x, end.x), y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x), height: Math.abs(end.y - start.y),
  } : null);

  function begin(event: PointerEvent) {
    if (event.button !== 0 || busy) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    start = { x: event.clientX, y: event.clientY };
    end = start;
  }

  async function submit() {
    if (busy) return;
    if (!selection || selection.width < 2 || selection.height < 2) { start = null; end = null; return; }
    busy = true;
    try { await mediaApi.completeAreaCapture(selection); }
    catch (cause) { error = String(cause); busy = false; }
  }

  async function finish(event: PointerEvent) {
    if (!start || busy) return;
    end = { x: event.clientX, y: event.clientY };
    await submit();
  }

  function keyboard(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.preventDefault(); void cancel(); return; }
    if (event.key === 'Enter' && selection) { event.preventDefault(); void submit(); return; }
    if (!event.key.startsWith('Arrow') || busy) return;
    event.preventDefault();
    if (!start || !end) {
      start = { x: Math.round(innerWidth / 4), y: Math.round(innerHeight / 4) };
      end = { x: Math.round(innerWidth * 3 / 4), y: Math.round(innerHeight * 3 / 4) };
      return;
    }
    const step = event.shiftKey ? 10 : 1;
    const dx = event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0;
    const dy = event.key === 'ArrowDown' ? step : event.key === 'ArrowUp' ? -step : 0;
    if (event.altKey) {
      end = { x: Math.max(start.x + 2, Math.min(innerWidth, end.x + dx)), y: Math.max(start.y + 2, Math.min(innerHeight, end.y + dy)) };
    } else {
      const x = Math.max(-Math.min(start.x, end.x), Math.min(innerWidth - Math.max(start.x, end.x), dx));
      const y = Math.max(-Math.min(start.y, end.y), Math.min(innerHeight - Math.max(start.y, end.y), dy));
      start = { x: start.x + x, y: start.y + y }; end = { x: end.x + x, y: end.y + y };
    }
  }

  async function cancel() {
    try { await mediaApi.cancelAreaCapture(); }
    catch (cause) { error = String(cause); }
  }

  onMount(() => {
    let disposed = false;
    void mediaApi.areaCaptureFrame().then((bytes) => {
      if (!disposed) frameUrl = URL.createObjectURL(new Blob([bytes], { type: 'image/png' }));
    }).catch((cause) => { if (!disposed) error = String(cause); });
    return () => { disposed = true; if (frameUrl) URL.revokeObjectURL(frameUrl); };
  });
</script>

<svelte:window onkeydown={keyboard} />

<main class="overlay" role="application" aria-label="Select an area. Drag a rectangle, or use arrow keys to move and Alt plus arrows to resize. Enter captures; Escape cancels." aria-busy={busy}
  onpointerdown={begin} onpointermove={(event) => { if (start && !busy) end = { x: event.clientX, y: event.clientY }; }} onpointerup={finish}>
  {#if frameUrl}<img class="frame" src={frameUrl} alt="Screen at capture time" draggable="false" />{/if}
  <div class="hint card">
    <p>{busy ? 'Saving capture…' : 'Drag, or use arrows. Alt + arrows resizes; Enter captures; Esc cancels.'}</p>
    <button class="btn" onclick={(event) => { event.stopPropagation(); void cancel(); }} onpointerdown={(event) => event.stopPropagation()}>Cancel</button>
    {#if error}<Notice kind="error">{error}</Notice>{/if}
  </div>
  {#if selection}
    <div class="selection" style:left="{selection.x}px" style:top="{selection.y}px" style:width="{selection.width}px" style:height="{selection.height}px">
      <output>{Math.round(selection.width)} × {Math.round(selection.height)}</output>
    </div>
  {/if}
</main>

<style>
  :global(html), :global(body) { background: transparent !important; }
  .overlay { position: fixed; inset: 0; cursor: crosshair; user-select: none; touch-action: none; background: var(--accent-soft); }
  .frame { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .hint { position: absolute; top: var(--s-6); left: var(--s-6); display: flex; align-items: center; gap: var(--s-4); padding: var(--s-4); z-index: 1; cursor: default; }
  .selection { position: absolute; border: 2px solid var(--accent); background: transparent; pointer-events: none; }
  output { position: absolute; top: var(--s-2); left: var(--s-2); padding: var(--s-1) var(--s-2); border-radius: var(--r-sm); color: var(--text); background: var(--surface); white-space: nowrap; }
</style>

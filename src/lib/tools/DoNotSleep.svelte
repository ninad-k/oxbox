<script lang="ts">
  import { onMount } from 'svelte';
  import { systemApi, type AwakeStatus } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let status = $state<AwakeStatus>({ active: false, display: false, until: null });
  let display = $state(false);
  let mode = $state('indefinite');
  let minutes = $state(25);
  let until = $state('');
  let busy = $state(false);
  let loading = $state(true);
  let error = $state('');
  let statusError = $state('');
  let requestId = 0;
  onMount(() => {
    let stopped = false; let timer: ReturnType<typeof setTimeout>;
    async function poll() { const id = requestId; try { const s = await systemApi.awakeStatus(); if (!stopped && id === requestId) { status = s; statusError = ''; } } catch (e) { if (!stopped && id === requestId) statusError = e instanceof Error ? e.message : String(e); } finally { if (!stopped) { loading = false; timer = setTimeout(poll, 1000); } } }
    void poll(); return () => { stopped = true; requestId++; clearTimeout(timer); };
  });
  async function change(start: boolean) {
    const id = ++requestId; busy = true; error = '';
    try { if (start) { const s = await systemApi.awakeStart(display, mode === 'duration' ? minutes : null, mode === 'until' ? Math.floor(Date.parse(until) / 1000) : null); if (id === requestId) status = s; } else { await systemApi.awakeStop(); if (id === requestId) status = { active: false, display: false, until: null }; } }
    catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); }
    finally { if (id === requestId) busy = false; }
  }
</script>
<div class="tool" aria-busy={busy || loading}>
  <Notice kind={status.active ? 'success' : 'info'}>{status.active ? `Keeping the system awake${status.display ? ' and the display on' : ''}${status.until ? ` until ${new Date(status.until * 1000).toLocaleString()}` : ''}.` : loading ? 'Reading keep-awake status…' : 'Do Not Sleep is off.'}</Notice>
  <label class="field"><span class="field-label">Keep awake</span><select class="input" bind:value={mode} disabled={status.active || busy}><option value="indefinite">Until I stop it</option><option value="duration">For a duration</option><option value="until">Until a time</option></select></label>
  {#if mode === 'duration'}<label class="field"><span class="field-label">Minutes</span><input class="input" type="number" min="1" max="44640" step="1" bind:value={minutes} disabled={status.active || busy} /></label>{/if}
  {#if mode === 'until'}<label class="field"><span class="field-label">End time (local)</span><input class="input" type="datetime-local" bind:value={until} disabled={status.active || busy} /></label>{/if}
  <label><input type="checkbox" bind:checked={display} disabled={status.active || busy} /> Keep the display on too</label>
  <div><button class="btn btn-primary" disabled={busy || loading || (!status.active && mode === 'until' && !Number.isFinite(Date.parse(until)))} onclick={() => change(!status.active)}>{busy ? 'Updating…' : status.active ? 'Stop keeping awake' : 'Start keeping awake'}</button></div>
  <p>Stopping or quitting Oxbox releases the awake request. The operating system may still obey explicit sleep, lid, or battery policies.</p>
  {#if error || statusError}<Notice kind="error">{error || statusError}</Notice>{/if}
</div>
<style>.tool { display: grid; gap: var(--s-4); } p { color: var(--text-2); }</style>

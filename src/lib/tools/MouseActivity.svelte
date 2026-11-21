<script lang="ts">
  import { onMount } from 'svelte';
  import { mouseActivityApi, activityLabel, type MouseActivityStatus } from '#lib/mouse-activity-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let status = $state<MouseActivityStatus>({ active: false, phase: 'stopped', idleSeconds: 0, movements: 0, remainingSeconds: null, error: null });
  let idleSeconds = $state(60), intervalSeconds = $state(30), capped = $state(true), durationMinutes = $state(60);
  let loading = $state(true), busy = $state(false), error = $state(''), statusError = $state('');
  let requestId = 0;
  const valid = $derived(Number.isInteger(idleSeconds) && idleSeconds >= 30 && idleSeconds <= 3600 && Number.isInteger(intervalSeconds) && intervalSeconds >= 5 && intervalSeconds <= 300 && (!capped || (Number.isInteger(durationMinutes) && durationMinutes >= 1 && durationMinutes <= 1440)));
  onMount(() => {
    let stopped = false; let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      if (busy) { timer = setTimeout(poll, 500); return; }
      const id = requestId;
      try { const next = await mouseActivityApi.status(); if (!stopped && id === requestId) { status = next; statusError = ''; } }
      catch (cause) { if (!stopped && id === requestId) statusError = cause instanceof Error ? cause.message : String(cause); }
      finally { if (!stopped) { loading = false; timer = setTimeout(poll, 500); } }
    }
    void poll(); return () => { stopped = true; requestId++; clearTimeout(timer); };
  });
  async function change(start: boolean) {
    const id = ++requestId; busy = true; error = '';
    try {
      const next = start ? await mouseActivityApi.start({ idleSeconds, intervalSeconds, durationMinutes: capped ? durationMinutes : null }) : await mouseActivityApi.stop();
      if (id === requestId) status = next;
    } catch (cause) { if (id === requestId) error = cause instanceof Error ? cause.message : String(cause); }
    finally { if (id === requestId) busy = false; }
  }
</script>
<div class="tool" aria-busy={loading || busy}>
  <Notice kind={status.active ? 'success' : 'info'}>{loading ? 'Reading activity status…' : activityLabel(status)}</Notice>
  <p>Windows only. After the idle delay, move the pointer by one pixel at each interval. No clicks, typing or scrolling. Real input suspends movement. Start is always explicit.</p>
  <label class="field"><span class="field-label">Wait after real input (seconds)</span><input class="input" type="number" min="30" max="3600" step="1" bind:value={idleSeconds} disabled={status.active || busy} /></label>
  <label class="field"><span class="field-label">Movement interval while idle (seconds)</span><input class="input" type="number" min="5" max="300" step="1" bind:value={intervalSeconds} disabled={status.active || busy} /></label>
  <label><input type="checkbox" bind:checked={capped} disabled={status.active || busy} /> Stop after a duration</label>
  {#if capped}<label class="field"><span class="field-label">Duration (minutes)</span><input class="input" type="number" min="1" max="1440" step="1" bind:value={durationMinutes} disabled={status.active || busy} /></label>{/if}
  <div><button class="btn btn-primary" onclick={() => change(!status.active)} disabled={loading || busy || (!status.active && !valid)}>{busy ? 'Updating…' : status.active ? 'Stop Mouse Activity' : 'Start Mouse Activity'}</button></div>
  {#if status.active}<p>Idle for {status.idleSeconds} seconds · {status.movements} simulated movements{status.remainingSeconds === null ? '' : ` · stops in ${status.remainingSeconds} seconds`}.</p>{/if}
  <p>Stop is also available in the launcher banner and tray. Quitting ends this session; reopening Oxbox starts stopped. OS sleep and lock policies still apply.</p>
  {#if error || statusError || status.error}<Notice kind="error">{error || statusError || status.error}</Notice>{/if}
</div>
<style>.tool { display: grid; gap: var(--s-4); } p { color: var(--text-2); }</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { systemApi, type BreakStatus } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let minutes = $state(20); let eyeRest = $state(false); let pauseFullscreen = $state(true); let status = $state<BreakStatus | null>(null); let error = $state(''); let busy = $state(false); let loading = $state(true); let requestId = 0;
  onMount(() => { let stopped = false; let first = true; let timer: ReturnType<typeof setTimeout>; async function poll() { const id = requestId; try { const value = await systemApi.breakStatus(); if (!stopped && id === requestId) { status = value.status; if (first) { minutes = value.settings.minutes; eyeRest = value.settings.eyeRest; pauseFullscreen = value.settings.pauseFullscreen; first = false; } } } catch (e) { if (!stopped && id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (!stopped) { loading = false; timer = setTimeout(poll, 1000); } } } void poll(); return () => { stopped = true; requestId++; clearTimeout(timer); }; });
  async function configure(enabled: boolean) { const id = ++requestId; busy = true; error = ''; try { const value = await systemApi.breakConfigure({ minutes, eyeRest, pauseFullscreen, enabled }); if (id === requestId) status = value; } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; } }
  async function respond(snooze: boolean) { const id = ++requestId; busy = true; error = ''; try { if (snooze) await systemApi.breakSnooze(); else await systemApi.breakSkip(); const value = await systemApi.breakStatus(); if (id === requestId) status = value.status; } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; } }
</script>
<div class="tool" aria-busy={busy || loading}>
  <p>Break reminders continue while the launcher is hidden. Your interval and enabled state are remembered across restarts.</p>
  <label class="field"><span class="field-label">Remind every (minutes of activity)</span><input class="input" type="number" min="1" max="1440" step="1" bind:value={minutes} disabled={busy} /></label>
  <label><input type="checkbox" bind:checked={eyeRest} disabled={busy} /> 20-20-20 eye rest: look 20 feet away for 20 seconds</label>
  <label><input type="checkbox" bind:checked={pauseFullscreen} disabled={busy} /> Pause reminders during fullscreen applications (best effort)</label>
  <Notice kind="info">Idle and fullscreen detection currently applies on Windows; other platforms count elapsed time. Windows pauses after one minute without input.</Notice>
  <div class="actions"><button class="btn btn-primary" onclick={() => configure(true)} disabled={busy || loading}>{status?.enabled ? 'Save and restart interval' : 'Start reminders'}</button>{#if status?.enabled}<button class="btn" onclick={() => configure(false)} disabled={busy}>Stop reminders</button>{/if}</div>
  {#if status?.awaitingBreak}<Notice kind="success">Time for a break. {status.eyeRest ? 'Rest your eyes for 20 seconds.' : 'Stand up, stretch, and rest your eyes.'}</Notice><div class="actions"><button class="btn" onclick={() => respond(true)} disabled={busy}>Snooze 5 minutes</button><button class="btn" onclick={() => respond(false)} disabled={busy}>Continue / skip this break</button></div>{:else if status?.enabled}<Notice kind="info">{status.paused ? 'Paused while inactive or fullscreen.' : `Next reminder in ${Math.ceil(status.remainingSeconds / 60)} minutes.`}</Notice>{:else if !loading}<Notice kind="info">Break reminders are off.</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool { display: grid; gap: var(--s-4); } .actions { display: flex; flex-wrap: wrap; gap: var(--s-2); }</style>

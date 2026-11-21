<script lang="ts">
  import { onMount } from 'svelte';
  import { inDesktopApp } from '#lib/ipc.ts';
  import { systemApi, type SleepTimerStatus } from '#lib/system-api.ts';
  let timer = $state<SleepTimerStatus | null>(null); let error = $state(''); let busy = $state(false); let remaining = $state(0);
  onMount(() => { if (!inDesktopApp()) return; let stopped = false; let handle: ReturnType<typeof setTimeout>; async function poll() { try { const status = await systemApi.sleepStatus(); if (!stopped) { timer = status.timer; remaining = status.remainingSeconds; error = status.error ?? ''; } } catch (e) { if (!stopped) error = e instanceof Error ? e.message : String(e); } finally { if (!stopped) { handle = setTimeout(poll, 1000); } } } void poll(); return () => { stopped = true; clearTimeout(handle); }; });
  async function cancel() { busy = true; try { await systemApi.sleepCancel(); timer = null; error = ''; } catch (e) { error = e instanceof Error ? e.message : String(e); } finally { busy = false; } }
</script>
{#if timer}<aside class="banner" aria-label="Active Sleep Timer" aria-busy={busy}><span>Sleep Timer: {timer.action.replace('-', ' ')} in {Math.floor(remaining / 60)}m {remaining % 60}s</span><button class="btn" onclick={cancel} disabled={busy}>Cancel timer</button></aside>{/if}
{#if error}<p class="banner" role="alert">{error}</p>{/if}
<style>.banner { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: var(--s-3); padding: var(--s-3) var(--s-6); background: var(--accent-soft); border-bottom: 1px solid var(--border); }</style>

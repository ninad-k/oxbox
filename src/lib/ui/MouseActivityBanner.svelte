<script lang="ts">
  import { onMount } from 'svelte';
  import { inDesktopApp } from '#lib/ipc.ts';
  import { mouseActivityApi, type MouseActivityStatus } from '#lib/mouse-activity-api.ts';
  import Notice from './Notice.svelte';
  let status = $state<MouseActivityStatus | null>(null), busy = $state(false), error = $state(''), statusError = $state('');
  let mounted = false, generation = 0;
  onMount(() => {
    mounted = true;
    if (!inDesktopApp()) return () => { mounted = false; };
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      if (busy) { timer = setTimeout(poll, 500); return; }
      const id = generation;
      try { const next = await mouseActivityApi.status(); if (mounted && id === generation) { status = next; statusError = ''; } }
      catch (cause) { if (mounted && id === generation) statusError = cause instanceof Error ? cause.message : String(cause); }
      finally { if (mounted) timer = setTimeout(poll, 500); }
    }
    void poll(); return () => { mounted = false; generation++; clearTimeout(timer); };
  });
  async function stop() {
    const id = ++generation; busy = true; error = '';
    try { const next = await mouseActivityApi.stop(); if (mounted && id === generation) status = next; }
    catch (cause) { if (mounted && id === generation) error = cause instanceof Error ? cause.message : String(cause); }
    finally { if (mounted && id === generation) busy = false; }
  }
</script>
{#if status?.active}<div class="banner" aria-busy={busy}><Notice kind="info">Mouse Activity: {status.phase === 'simulating' ? 'simulating while idle' : status.phase === 'paused' ? 'paused by input' : 'waiting for idle'}.</Notice><button class="btn" onclick={stop} disabled={busy}>{busy ? 'Stopping…' : 'Stop Mouse Activity'}</button></div>{/if}
{#if error || statusError}<Notice kind="error">{error || statusError}</Notice>{/if}
{#if status?.error}<Notice kind="error">{status.error}</Notice>{/if}
<style>.banner { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-3); padding: var(--s-3); border-bottom: 1px solid var(--border); } .banner :global(.notice) { flex: 1; }</style>

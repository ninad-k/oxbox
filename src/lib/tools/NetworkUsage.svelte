<script lang="ts">
  import { onMount } from 'svelte';
  import { systemApi, formatBytes, type InterfaceUsage } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let interfaces = $state<InterfaceUsage[]>([]);
  let showVirtual = $state(false);
  let busy = $state(true);
  let error = $state('');
  let requestId = 0;
  const shown = $derived(interfaces.filter(i => showVirtual || !i.virtualInterface));
  const total = $derived(shown.reduce((sum, i) => ({ down: sum.down + i.downloadPerSecond, up: sum.up + i.uploadPerSecond }), { down: 0, up: 0 }));
  onMount(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      const id = ++requestId;
      try { const values = await systemApi.networkSample(); if (!stopped && id === requestId) { interfaces = values; error = ''; } }
      catch (e) { if (!stopped && id === requestId) error = e instanceof Error ? e.message : String(e); }
      finally { if (!stopped && id === requestId) { busy = false; timer = setTimeout(poll, 1000); } }
    }
    void poll(); return () => { stopped = true; requestId++; clearTimeout(timer); };
  });
</script>
<div class="tool" aria-busy={busy}>
  <p>Interface counters are read locally. Session totals begin when Network Usage is first opened and reset when Oxbox quits.</p>
  <div class="card totals"><strong>↓ {formatBytes(total.down)}/s</strong><strong>↑ {formatBytes(total.up)}/s</strong></div>
  <label><input type="checkbox" bind:checked={showVirtual} /> Show loopback and virtual interfaces</label>
  {#if error}<Notice kind="error">{error}</Notice>{/if}
  {#if busy}<Notice kind="info">Reading interface counters…</Notice>{:else if !shown.length && !error}<Notice kind="info">No matching network interfaces found.</Notice>{/if}
  {#each shown as item (item.name)}<section class="card interface"><h2>{item.name}</h2><div class="rates"><span>Download: {formatBytes(item.downloadPerSecond)}/s</span><span>Upload: {formatBytes(item.uploadPerSecond)}/s</span></div><p>Received: {formatBytes(item.received)} · Sent: {formatBytes(item.transmitted)}</p>{#if item.virtualInterface}<small>Virtual or loopback interface</small>{/if}</section>{/each}
</div>
<style>.tool { display: grid; gap: var(--s-4); } .totals, .rates { display: flex; flex-wrap: wrap; gap: var(--s-6); } .totals, .interface { padding: var(--s-4); } .totals { font-size: var(--text-xl); } .interface { display: grid; gap: var(--s-3); } h2 { font-size: var(--text-lg); } small, p { color: var(--text-2); }</style>

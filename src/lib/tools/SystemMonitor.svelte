<script lang="ts">
  import { onMount } from 'svelte';
  import { systemApi, formatBytes, type SystemSnapshot } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let snapshot = $state<SystemSnapshot | null>(null);
  let error = $state('');
  let busy = $state(true);
  let history = $state<number[]>([]);
  let order = $state<'cpu' | 'memory'>('cpu');
  let requestId = 0;
  const processes = $derived(snapshot ? order === 'cpu' ? snapshot.processesCpu : snapshot.processesMemory : []);
  const points = $derived(history.map((v, i) => `${i * 100 / 59},${100 - v}`).join(' '));
  onMount(() => {
    let timer: ReturnType<typeof setTimeout>;
    let stopped = false;
    async function poll() {
      const id = ++requestId;
      try { const value = await systemApi.systemSnapshot(); if (stopped || id !== requestId) return; snapshot = value; history = [...history.slice(-59), value.cpuPercent]; error = ''; }
      catch (e) { if (!stopped && id === requestId) error = String(e instanceof Error ? e.message : e); }
      finally { if (!stopped && id === requestId) { busy = false; timer = setTimeout(poll, 1000); } }
    }
    void poll();
    return () => { stopped = true; requestId++; clearTimeout(timer); };
  });
</script>

<div class="tool" aria-busy={busy}>
  <p>Local system metrics refresh while this view is open. CPU readings settle after the first sample.</p>
  {#if error}<Notice kind="error">{error}</Notice>{/if}
  {#if snapshot}
    <div class="stats">
      <div class="card stat"><span>CPU</span><strong>{snapshot.cpuPercent.toFixed(1)}%</strong></div>
      <div class="card stat"><span>Memory</span><strong>{formatBytes(snapshot.memoryUsed)} / {formatBytes(snapshot.memoryTotal)}</strong></div>
      <div class="card stat"><span>Swap</span><strong>{formatBytes(snapshot.swapUsed)} / {formatBytes(snapshot.swapTotal)}</strong></div>
    </div>
    <figure class="card chart"><figcaption>CPU history · last {history.length} samples</figcaption><svg viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="CPU usage history from zero to one hundred percent"><polyline {points} fill="none" stroke="var(--accent)" stroke-width="1" vector-effect="non-scaling-stroke" /></svg></figure>
    <details><summary>Logical CPU cores ({snapshot.cores.length})</summary><div class="cores">{#each snapshot.cores as value, i}<label>Core {i + 1}: {value.toFixed(1)}%<progress max="100" {value}></progress></label>{/each}</div></details>
    <section class="card section"><h2>Disks</h2>{#each snapshot.disks as disk}<div class="disk"><strong>{disk.mount || disk.name}</strong><span>{formatBytes(disk.total - disk.available)} used · {formatBytes(disk.available)} free</span><progress max={disk.total || 1} value={disk.total - disk.available} aria-label={`${disk.mount} space used`}></progress></div>{:else}<p>No disk information available.</p>{/each}</section>
    <section class="card section"><div class="row"><h2>Top processes</h2><label>Sort by <select class="input" bind:value={order}><option value="cpu">CPU</option><option value="memory">Memory</option></select></label></div><div class="table-wrap"><table><thead><tr><th>Process</th><th>PID</th><th>CPU</th><th>Memory</th></tr></thead><tbody>{#each processes as process (process.pid)}<tr><td>{process.name}</td><td>{process.pid}</td><td>{process.cpuPercent.toFixed(1)}%</td><td>{formatBytes(process.memory)}</td></tr>{/each}</tbody></table></div></section>
  {:else if busy}<Notice kind="info">Reading system information…</Notice>{/if}
</div>

<style>
  .tool { display: grid; gap: var(--s-4); } .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--s-3); } .stat, .section, .chart { padding: var(--s-4); } .stat { display: grid; gap: var(--s-2); } .stat span, figcaption { color: var(--text-2); } .section { display: grid; gap: var(--s-4); } h2 { font-size: var(--text-lg); } .chart { margin: 0; } .chart svg { width: 100%; height: 120px; margin-top: var(--s-3); } .cores { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: var(--s-3); padding-top: var(--s-3); } .cores label, .disk { display: grid; gap: var(--s-1); } progress { width: 100%; accent-color: var(--accent); } .row { display: flex; align-items: center; justify-content: space-between; gap: var(--s-3); } .row label { display: flex; align-items: center; gap: var(--s-2); } .table-wrap { overflow-x: auto; } table { width: 100%; border-collapse: collapse; text-align: left; } td, th { padding: var(--s-2); border-bottom: 1px solid var(--border); } td:first-child { word-break: break-word; }
</style>

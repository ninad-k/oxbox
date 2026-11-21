<script lang="ts">
  import type { JobProgress } from '#lib/jobs.ts';
  let {progress=null,oncancel}: {progress?:JobProgress|null; oncancel?:()=>void}=$props();
</script>
<div class="job-progress" aria-busy="true" role="status">
  <span>{progress?.message ?? 'Starting…'}</span>
  <progress aria-label="Job progress" max={progress?.total || 1} value={progress && progress.total>0 ? Math.min(progress.done,progress.total) : undefined}></progress>
  {#if progress && progress.total>0}<span>{Math.min(100,Math.round(progress.done/progress.total*100))}%</span>{/if}
  {#if oncancel}<button class="btn" onclick={oncancel}>Cancel</button>{/if}
</div>
<style>
  .job-progress{display:flex;align-items:center;flex-wrap:wrap;gap:var(--s-3);font-size:var(--text-sm)}
  progress{flex:1;min-width:var(--s-8);accent-color:var(--accent)}
</style>

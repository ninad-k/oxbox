<script lang="ts">
  import { onDestroy } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { fileApi, formatBytes, type CleanCategory } from '#lib/file-api.ts';
  import { createJob, type JobHandle, type JobProgress } from '#lib/jobs.ts';
  import Notice from '#lib/ui/Notice.svelte';
  import ProgressBar from '#lib/ui/ProgressBar.svelte';
  let categories=$state<CleanCategory[]|null>(null);
  let selected=$state<string[]>([]);
  let busy=$state(false);
  let error=$state('');
  let message=$state('');
  let progress=$state<JobProgress|null>(null);
  let job=$state<JobHandle|null>(null);
  let requestId=0;
  const reviewed=$derived(categories?.filter((c)=>!c.readOnly && selected.includes(c.id)) ?? []);
  const totalBytes=$derived(reviewed.reduce((sum,c)=>sum+c.bytes,0));
  const totalFiles=$derived(reviewed.reduce((sum,c)=>sum+c.files,0));
  onDestroy(()=>{requestId++;void job?.cancel().catch(()=>{});});
  async function scan() {
    if(busy)return;
    const id=++requestId;busy=true;error='';message='';categories=null;selected=[];progress=null;
    const running=createJob('clean-drive',(p)=>{if(id===requestId)progress=p;});job=running;
    try{const value=await running.run((jobId)=>fileApi.scanCleanDrive(jobId));if(id===requestId)categories=value;}
    catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}
    finally{if(id===requestId){busy=false;job=null;}}
  }
  async function clean() {
    if(busy || totalFiles===0)return;
    const ids=[...selected],files=totalFiles,bytes=totalBytes;const id=++requestId;
    busy=true;error='';message='';progress=null;
    try{
      const confirmed=await ask(`Move ${files} reviewed cache files (${formatBytes(bytes)}) to the Recycle Bin / Trash?\n\n${reviewed.map((c)=>`${c.name}\n${c.path}\n${c.files} files, ${formatBytes(c.bytes)}`).join('\n\n')}\n\nClose the affected applications first. Temporary files less than 24 hours old are kept.`,{title:'Review cleanup',kind:'warning'});
      if(!confirmed || id!==requestId)return;
      const running=createJob('clean-drive',(p)=>{if(id===requestId)progress=p;});job=running;
      const count=await running.run((jobId)=>fileApi.cleanDrive(jobId,ids,files,bytes,true));
      if(id===requestId){message=`Moved ${count} cache files to Trash. Scan again to refresh the totals.`;categories=null;selected=[];}
    }catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}
    finally{if(id===requestId){busy=false;job=null;}}
  }
</script>
<div class="tool" aria-busy={busy}>
  <Notice kind="info">Scan a fixed list of your user caches, review the totals, then choose what to clean. Files are moved to Trash. Close browsers and package managers before cleaning their caches.</Notice>
  <button class="btn btn-primary" disabled={busy} onclick={scan}>Scan user caches</button>
  {#if busy}<ProgressBar {progress} oncancel={job?()=>{void job?.cancel().catch((e)=>{error=String(e);});}:undefined} />{/if}
  {#if categories}
    {#each categories as category}<label class="card category"><input type="checkbox" disabled={busy || category.files===0 || category.readOnly} checked={selected.includes(category.id)} onchange={(e)=>{selected=e.currentTarget.checked?[...selected,category.id]:selected.filter((id)=>id!==category.id);}} /><span class="details"><strong>{category.name}</strong><span class="path">{category.path}</span><span>{category.files} files · {formatBytes(category.bytes)}{category.readOnly?' · Read only':''}{category.skipped?` · ${category.skipped} inaccessible entries skipped`:''}</span></span></label>{/each}
    {#if categories.every((c)=>c.files===0)}<Notice kind="success">No eligible cache files found.</Notice>{/if}
    <p>Selected: {totalFiles} files, {formatBytes(totalBytes)}.</p>
    <button class="btn" disabled={busy || totalFiles===0} onclick={clean}>Review and move selected files to Trash…</button>
  {/if}
  {#if message}<Notice kind="success">{message}</Notice>{/if}
  {#if error}<Notice kind="error">{error} Some reviewed files may already have been moved. Scan again before retrying.</Notice>{/if}
  <p class="note">The Recycle Bin / Trash is never emptied. User documents, downloads and unknown folders are outside the cleanup allow-list.</p>
</div>
<style>
  .tool{display:grid;gap:var(--s-4)}.category{display:flex;align-items:flex-start;gap:var(--s-3);padding:var(--s-4)}.details{display:grid;gap:var(--s-1);min-width:0}.path{font-size:var(--text-xs);color:var(--text-3);overflow-wrap:anywhere}.note{font-size:var(--text-sm);color:var(--text-2)}
</style>

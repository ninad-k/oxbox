<script lang="ts">
  import { onDestroy } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { fileApi, formatBytes, type DuplicateResult } from '#lib/file-api.ts';
  import { createJob, type JobHandle, type JobProgress } from '#lib/jobs.ts';
  import FileDrop from '#lib/ui/FileDrop.svelte';
  import Notice from '#lib/ui/Notice.svelte';
  import ProgressBar from '#lib/ui/ProgressBar.svelte';
  let roots=$state<string[]>([]);
  let includeHidden=$state(false);
  let result=$state<DuplicateResult|null>(null);
  let selected=$state<string[]>([]);
  let busy=$state(false);
  let error=$state('');
  let message=$state('');
  let progress=$state<JobProgress|null>(null);
  let job=$state<JobHandle|null>(null);
  let requestId=0;
  const selectedBytes=$derived(result?.groups.reduce((sum,g)=>sum+g.size*g.files.filter((f)=>selected.includes(f.path)).length,0) ?? 0);
  const removesEntireGroup=$derived(result?.groups.some((g)=>g.files.every((f)=>selected.includes(f.path))) ?? false);
  onDestroy(()=>{requestId++;void job?.cancel().catch(()=>{});});
  async function scan() {
    if(busy)return;
    const id=++requestId;busy=true;error='';message='';result=null;selected=[];progress=null;
    const running=createJob('find-duplicates',(p)=>{if(id===requestId)progress=p;});job=running;
    try {const value=await running.run((jobId)=>fileApi.findDuplicates(jobId,roots,includeHidden));if(id===requestId)result=value;}
    catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}
    finally{if(id===requestId){busy=false;job=null;}}
  }
  function autoSelect(keep:'oldest'|'newest') {
    selected=result?.groups.flatMap((group)=>[...group.files].sort((a,b)=>(keep==='oldest'?a.modifiedMs-b.modifiedMs:b.modifiedMs-a.modifiedMs)||a.path.localeCompare(b.path)).slice(1).map((f)=>f.path))??[];
  }
  function toggle(path:string,checked:boolean){selected=checked?[...selected,path]:selected.filter((p)=>p!==path);}
  async function trashSelected() {
    if(busy || selected.length===0 || removesEntireGroup)return;
    const paths=[...selected];const id=++requestId;busy=true;error='';message='';
    try {
      const confirmed=await ask(`Move ${paths.length} selected copies (${formatBytes(selectedBytes)}) to the Recycle Bin / Trash?\n\n${paths.slice(0,20).join('\n')}${paths.length>20?`\n…and ${paths.length-20} more`:''}\n\nOne file in each group will be kept.`,{title:'Review duplicate cleanup',kind:'warning'});
      if(!confirmed || id!==requestId)return;
      const count=await fileApi.trashFiles(paths,true);
      if(id!==requestId)return;
      message=`Moved ${count} files to Trash.`;
      result=null;selected=[];
    }catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}
    finally{if(id===requestId)busy=false;}
  }
</script>
<div class="tool" aria-busy={busy}>
  <FileDrop label="Drop folders to search for duplicate files" directory disabled={busy} ondrop={(paths)=>{roots=[...new Set([...roots,...paths])];result=null;selected=[];}} />
  {#each roots as root}<div class="row"><span class="path">{root}</span><button class="btn" disabled={busy} aria-label={`Remove ${root}`} onclick={()=>{roots=roots.filter((p)=>p!==root);result=null;selected=[];}}>Remove</button></div>{/each}
  <label class="row"><input type="checkbox" bind:checked={includeHidden} disabled={busy} />Include hidden files and folders</label>
  <button class="btn btn-primary" disabled={busy || roots.length===0} onclick={scan}>Find duplicates</button>
  {#if busy && job}<ProgressBar {progress} oncancel={()=>{void job?.cancel().catch((e)=>{error=String(e);});}} />{/if}
  {#if result}
    <Notice kind={result.groups.length?'info':'success'}>{result.groups.length} groups found in {result.scannedFiles} files. Duplicate copies use {formatBytes(result.wastedBytes)}.</Notice>
    {#if result.groups.length}
      <div class="row"><button class="btn" disabled={busy} onclick={()=>autoSelect('oldest')}>Keep oldest</button><button class="btn" disabled={busy} onclick={()=>autoSelect('newest')}>Keep newest</button><button class="btn" disabled={busy} onclick={()=>{selected=[];}}>Clear selection</button></div>
      {#each result.groups as group,index}<section class="card group" aria-label={`Duplicate group ${index+1}`}><strong>{formatBytes(group.size)} per file</strong>{#each group.files as file}<label class="file"><input type="checkbox" checked={selected.includes(file.path)} onchange={(e)=>toggle(file.path,e.currentTarget.checked)} disabled={busy} /><span class="path">{file.path}</span><span class="date">{file.modifiedMs?new Date(file.modifiedMs).toLocaleString():'Unknown date'}</span></label>{/each}</section>{/each}
      <button class="btn" disabled={busy || selected.length===0 || removesEntireGroup} onclick={trashSelected}>Move {selected.length} selected copies ({formatBytes(selectedBytes)}) to Trash…</button>
      {#if removesEntireGroup}<Notice kind="error">Keep at least one file in every group.</Notice>{/if}
    {/if}
  {:else if !busy && !message}<Notice kind="info">Choose folders, scan, then review copies before moving anything to Trash. Empty files and symbolic links are skipped.</Notice>{/if}
  {#if message}<Notice kind="success">{message} Scan again to refresh the results.</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>
  .tool{display:grid;gap:var(--s-4)}.row{display:flex;align-items:center;gap:var(--s-2);flex-wrap:wrap}.path{flex:1;overflow-wrap:anywhere;min-width:0}.group{display:grid;gap:var(--s-3);padding:var(--s-4)}.file{display:flex;align-items:center;gap:var(--s-3)}.date{font-size:var(--text-xs);color:var(--text-3)}
</style>

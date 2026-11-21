<script lang="ts">
  import { onDestroy } from 'svelte';
  import { ask, open, save } from '@tauri-apps/plugin-dialog';
  import { join } from '@tauri-apps/api/path';
  import { fileApi, type ArchiveFormat } from '#lib/file-api.ts';
  import { createJob, type JobHandle, type JobProgress } from '#lib/jobs.ts';
  import FileDrop from '#lib/ui/FileDrop.svelte';
  import Notice from '#lib/ui/Notice.svelte';
  import ProgressBar from '#lib/ui/ProgressBar.svelte';
  import Segmented from '#lib/ui/Segmented.svelte';
  import CopyButton from '#lib/ui/CopyButton.svelte';
  let mode=$state('create');
  let format=$state<ArchiveFormat>('zip');
  let chooseFolders=$state(false);
  let sources=$state<string[]>([]);
  let input=$state('');
  let folderName=$state('Extracted files');
  let maxMiB=$state(1024);
  let busy=$state(false);
  let error=$state('');
  let output=$state('');
  let progress=$state<JobProgress|null>(null);
  let job=$state<JobHandle|null>(null);
  let requestId=0;
  onDestroy(()=>{requestId++;void job?.cancel().catch(()=>{});});
  async function run() {
    if(busy)return;
    const id=++requestId;busy=true;error='';output='';progress=null;
    const running=createJob('archive',(p)=>{if(id===requestId)progress=p;});job=running;
    try {
      let value:string;
      if(mode==='create') {
        const extension=format==='zip'?'zip':'tar.gz';
        const path=await save({title:'Save a new archive',defaultPath:`Archive.${extension}`,filters:[{name:extension.toUpperCase(),extensions:[extension]}]});
        if(!path || id!==requestId)return;
        value=await running.run((jobId)=>fileApi.createArchive(jobId,sources,path,format));
      } else {
        if(!folderName.trim() || /[\\/:]/.test(folderName) || folderName==='.' || folderName==='..')throw new Error('Enter a new folder name without path separators.');
        if(!Number.isFinite(maxMiB) || maxMiB<1 || maxMiB>1048576)throw new Error('Choose an extraction limit from 1 to 1,048,576 MiB.');
        const parent=await open({title:'Choose the parent folder for extraction',directory:true,multiple:false});
        if(typeof parent!=='string' || id!==requestId)return;
        const destination=await join(parent,folderName.trim());
        if(!await ask(`Extract into a new folder:\n${destination}\n\nOutput is limited to ${maxMiB} MiB or 20× the archive size, whichever is smaller. Existing folders are never merged or overwritten.`,{title:'Review extraction',kind:'info'}))return;
        if(id!==requestId)return;
        value=await running.run((jobId)=>fileApi.extractArchive(jobId,input,destination,Math.floor(maxMiB*1024*1024)));
      }
      if(id===requestId)output=value;
    }catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}
    finally{if(id===requestId){busy=false;job=null;}}
  }
</script>
<div class="tool" aria-busy={busy}>
  <fieldset disabled={busy}><legend>Operation</legend><Segmented label="Archive operation" options={[{value:'create',label:'Create archive'},{value:'extract',label:'Extract archive'}]} bind:value={mode} /></fieldset>
  {#if mode==='create'}
    <label class="row"><input type="checkbox" bind:checked={chooseFolders} disabled={busy} />Choose folders in the file picker</label>
    <FileDrop label="Drop files or folders to archive" directory={chooseFolders} disabled={busy} ondrop={(paths)=>{sources=[...new Set([...sources,...paths])];output='';}} />
    {#each sources as source}<div class="row"><span class="path">{source}</span><button class="btn" disabled={busy} aria-label={`Remove ${source}`} onclick={()=>{sources=sources.filter((p)=>p!==source);}}>Remove</button></div>{/each}
    <label class="field"><span class="field-label">Archive format</span><select class="input" bind:value={format} disabled={busy}><option value="zip">ZIP</option><option value="tar-gz">TAR.GZ</option></select></label>
  {:else}
    <FileDrop label="Drop a ZIP, TAR, TAR.GZ or 7z archive" multiple={false} disabled={busy} ondrop={(paths)=>{if(paths[0]){input=paths[0];folderName=(input.split(/[\\/]/).pop() ?? 'Extracted files').replace(/\.(tar\.gz|tgz|zip|tar|7z)$/i,'')+' extracted';output='';}}} />
    {#if input}<p class="path">{input}</p>{/if}
    <label class="field"><span class="field-label">New folder name</span><input class="input" bind:value={folderName} disabled={busy} /></label>
    <label class="field"><span class="field-label">Maximum extracted size (MiB)</span><input class="input" type="number" min="1" max="1048576" bind:value={maxMiB} disabled={busy} /></label>
  {/if}
  <button class="btn btn-primary" disabled={busy || (mode==='create'?sources.length===0:!input)} onclick={run}>{mode==='create'?'Create archive…':'Choose destination and extract…'}</button>
  {#if busy}<ProgressBar {progress} oncancel={()=>{void job?.cancel().catch((e)=>{error=String(e);});}} />{/if}
  {#if output}<Notice kind="success">{mode==='create'?'Archive created':'Archive extracted'}: <span class="path">{output}</span></Notice><CopyButton text={output} />{/if}
  {#if error}<Notice kind="error">{error} If extraction started, its new folder may contain partial output; review it before trying again.</Notice>{/if}
  <Notice kind="info">Existing files are kept. Extraction rejects path traversal, links, duplicate names and oversized output. Choose a new destination for each extraction.</Notice>
</div>
<style>
  .tool{display:grid;gap:var(--s-4)}fieldset{border:0;padding:0;margin:0;display:grid;gap:var(--s-1)}legend{font-size:var(--text-sm);font-weight:600;color:var(--text-2);margin-bottom:var(--s-1)}.row{display:flex;align-items:center;gap:var(--s-2)}.path{overflow-wrap:anywhere;flex:1;min-width:0}
</style>

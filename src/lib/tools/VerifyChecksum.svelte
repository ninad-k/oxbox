<!-- Streaming checksum reference with file drop, progress and cancellation. -->
<script lang="ts">
  import { fileApi } from '#lib/file-api.ts';
  import { ToolError, type HashAlgorithm } from '#lib/api.ts';
  import { createJob, type JobHandle, type JobProgress } from '#lib/jobs.ts';
  import CopyButton from '#lib/ui/CopyButton.svelte';
  import FileDrop from '#lib/ui/FileDrop.svelte';
  import Notice from '#lib/ui/Notice.svelte';
  import ProgressBar from '#lib/ui/ProgressBar.svelte';
  import Segmented from '#lib/ui/Segmented.svelte';
  import { digestsMatch, fileName } from './checksum';

  const algorithms:{value:HashAlgorithm;label:string}[]=[{value:'sha256',label:'SHA-256'},{value:'sha1',label:'SHA-1'},{value:'md5',label:'MD5'},{value:'blake3',label:'BLAKE3'}];
  let path=$state('');
  let algorithm=$state<HashAlgorithm>('sha256');
  let digest=$state('');
  let expected=$state('');
  let busy=$state(false);
  let error=$state('');
  let progress=$state<JobProgress|null>(null);
  let job=$state<JobHandle|null>(null);
  let requestId=0;
  const comparison=$derived(digest && expected.trim()?digestsMatch(digest,expected):null);
  $effect(()=>{
    const selected=path;
    const algo=algorithm;
    const id=++requestId;
    if (!selected) return;
    busy=true;digest='';error='';progress=null;
    const running=createJob('verify-checksum',(p)=>{if(id===requestId)progress=p;});
    job=running;
    running.run((jobId)=>fileApi.checksumFileJob(jobId,selected,algo))
      .then((value)=>{if(id===requestId)digest=value;})
      .catch((e:unknown)=>{if(id===requestId)error=e instanceof ToolError?e.message:String(e);})
      .finally(()=>{if(id===requestId){busy=false;job=null;}});
    return ()=>{requestId++;void running.cancel().catch(()=>{});};
  });
</script>

<div class="tool" aria-busy={busy}>
  <FileDrop label="Drop a file to calculate its checksum" multiple={false} ondrop={(paths)=>{if(paths[0])path=paths[0];}} />
  {#if path}<div class="card selected"><strong>{fileName(path)}</strong><span class="path">{path}</span></div>{/if}
  <div class="field"><span class="field-label">Algorithm</span><Segmented label="Hash algorithm" options={algorithms} bind:value={algorithm} /></div>
  {#if busy}<ProgressBar {progress} oncancel={()=>{void job?.cancel().catch((e)=>{error=String(e);});}} />{/if}
  <div class="field">
    <div class="row"><span class="field-label" id="digest-label">Checksum</span><CopyButton text={digest} /></div>
    <output class="digest mono" aria-labelledby="digest-label">{digest || (busy?'Calculating…':'Choose a file to begin.')}</output>
  </div>
  <label class="field"><span class="field-label">Compare with (optional)</span><input class="input mono" bind:value={expected} placeholder="Paste the published checksum" /></label>
  {#if comparison===true}<Notice kind="success">Match. The file has the checksum you pasted.</Notice>{:else if comparison===false}<Notice kind="error">No match. Check the algorithm and the published checksum.</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
  <Notice kind="info">SHA-1 and MD5 are provided for compatibility. Prefer SHA-256 or BLAKE3 for integrity checks.</Notice>
</div>
<style>
  .tool{display:grid;gap:var(--s-5)}
  .selected{display:grid;gap:var(--s-1);padding:var(--s-4)}
  .path{font-size:var(--text-sm);color:var(--text-3);overflow-wrap:anywhere}
  .row{display:flex;align-items:center;justify-content:space-between}
  .digest{display:block;min-height:var(--s-8);padding:var(--s-3);border:1px solid var(--border);border-radius:var(--r-sm);background:var(--surface-sunken);word-break:break-all}
</style>

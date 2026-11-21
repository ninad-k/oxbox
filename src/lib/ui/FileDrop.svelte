<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { inDesktopApp } from '#lib/ipc.ts';
  import Notice from './Notice.svelte';
  let {label='Drop files here',directory=false,multiple=true,disabled=false,ondrop}: {label?:string;directory?:boolean;multiple?:boolean;disabled?:boolean;ondrop:(paths:string[])=>void}=$props();
  let hovering=$state(false);
  let error=$state('');
  let requestId=0;
  async function pick() {
    const id=++requestId;
    error='';
    if (!inDesktopApp()) {error='Open Oxbox as the desktop app to choose files.';return;}
    try {
      const result=await open({title:label,directory,multiple});
      if (result && id===requestId && !disabled) ondrop(typeof result==='string'?[result]:result);
    } catch(e) {if(id===requestId)error=e instanceof Error?e.message:String(e);}
  }
  onMount(()=>{
    if (!inDesktopApp()) return;
    let disposed=false;
    let stop:(()=>void)|undefined;
    getCurrentWebview().onDragDropEvent((event)=>{
      hovering=event.payload.type==='over' && !disabled;
      if (event.payload.type==='drop' && !disabled) {
        const paths=event.payload.paths;
        ondrop(multiple?paths:paths.slice(0,1));
      }
    }).then((unlisten)=>{if(disposed)unlisten();else stop=unlisten;}).catch((e)=>{error=String(e);});
    return ()=>{disposed=true;requestId++;stop?.();};
  });
</script>
<div class:hovering class="drop card" role="region" aria-label={label}>
  <span>{label}</span>
  <button class="btn" {disabled} onclick={pick}>Choose {directory?'folders':'files'}…</button>
</div>
{#if error}<Notice kind="error">{error}</Notice>{/if}
<style>
  .drop{display:flex;align-items:center;justify-content:space-between;gap:var(--s-4);padding:var(--s-4);border-style:dashed}
  .hovering{border-color:var(--accent);background:var(--accent-soft)}
</style>

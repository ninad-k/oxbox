<script lang="ts">
  import { onMount,onDestroy } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { roadmapSystemApi as desktop } from '#lib/roadmap-system-api.ts';
  import Notice from './Notice.svelte';
  let {kind}:{kind:'eject'|'power'}=$props();
  let choices=$state<{id:string;name:string;active:boolean}[]>([]),selected=$state('');let busy=$state(false),error=$state(''),message=$state('');let requestId=0;
  async function refresh(){const id=++requestId;busy=true;error='';try{const next=kind==='eject'?(await desktop.volumes()).map((v)=>({id:v.root,name:`${v.root} ${v.label}`,active:false})):(await desktop.powerProfiles()).map((p)=>({id:p.id,name:p.name,active:p.active}));if(id===requestId){choices=next;if(!next.some((c)=>c.id===selected))selected=next.find((c)=>c.active)?.id??next[0]?.id??'';}}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  async function run(){const choice=choices.find((c)=>c.id===selected);if(busy || !choice)return;const id=++requestId;busy=true;error='';message='';try{const question=kind==='eject'?`Safely eject ${choice.name}?\n\nSave and close files on this drive first. Oxbox will lock and dismount it and refuses to force a busy drive.`:`Switch to the installed power profile “${choice.name}”?\n\nThis changes Windows power settings and can affect battery life and performance.`;if(!await ask(question,{title:kind==='eject'?'Eject removable drive?':'Change power profile?',kind:'warning'}))return;if(id!==requestId)return;if(kind==='eject')await desktop.eject(choice.id,true);else await desktop.powerSet(choice.id,true);if(id===requestId){message=kind==='eject'?'The operating system accepted the ejection request.':`Activated ${choice.name}.`;choices=kind==='eject'?choices.filter((c)=>c.id!==choice.id):choices.map((c)=>({...c,active:c.id===choice.id}));}}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  onMount(()=>{void refresh();});onDestroy(()=>{requestId++;});
</script>
<div class="tool" aria-busy={busy}>
  <Notice kind="info">{kind==='eject'?'Lists removable and optical drive letters. Fixed disks and the Windows system drive are excluded. Busy drives are never forced.':'Choose an installed Windows power profile. Oxbox does not create plans or change individual power settings.'}</Notice>
  <button class="btn" disabled={busy} onclick={refresh}>Refresh {kind==='eject'?'removable drives':'power profiles'}</button>
  <label class="field"><span class="field-label">{kind==='eject'?'Drive':'Power profile'}</span><select class="input" bind:value={selected} disabled={busy}>{#each choices as choice}<option value={choice.id}>{choice.name}{choice.active?' · Active':''}</option>{/each}</select></label>
  <button class="btn btn-primary" disabled={busy || !choices.some((c)=>c.id===selected)} onclick={run}>{kind==='eject'?'Safely eject…':'Apply power profile…'}</button>
  {#if !busy && !choices.length && !error}<Notice kind="info">No {kind==='eject'?'supported removable drives':'installed power profiles'} found.</Notice>{/if}
  {#if message}<Notice kind="success">{message}</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}</style>

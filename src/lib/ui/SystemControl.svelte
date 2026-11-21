<script lang="ts">
  import {onMount,onDestroy} from 'svelte';
  import {roadmapTimeApi,type ControlCapability} from '#lib/roadmap-time-api.ts';
  import Notice from './Notice.svelte';
  let {control}:{control:'dnd'|'airplane'}=$props();
  let capability=$state<ControlCapability|null>(null),busy=$state(true),error=$state(''),message=$state('');let requestId=0;
  onMount(async()=>{const id=++requestId;try{const value=await(control==='dnd'?roadmapTimeApi.dndCapability():roadmapTimeApi.airplaneCapability());if(id===requestId)capability=value;}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}});
  onDestroy(()=>requestId++);
  async function open(){const id=++requestId;busy=true;error='';message='';try{await(control==='dnd'?roadmapTimeApi.dndOpenSettings():roadmapTimeApi.airplaneOpenSettings());if(id===requestId)message='The system settings launch request was sent. Choose the setting there; Oxbox cannot verify its state.';}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
</script>
<div class="tool" aria-busy={busy}>
  {#if capability}<Notice kind="info">{capability.message}</Notice><p>Current state: unknown. Changes are made by you in the operating system's controls.</p><div><button class="btn btn-primary" disabled={busy||!capability.available} onclick={open}>{capability.actionLabel}</button></div>{#if !capability.available}<Notice kind="info">Direct controls are not available on this operating system.</Notice>{/if}{:else if busy}<Notice kind="info">Checking available system controls…</Notice>{/if}
  {#if message}<Notice kind="success">{message}</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}p{color:var(--text-2)}</style>

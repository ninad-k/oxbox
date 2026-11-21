<script lang="ts">
  import { onDestroy } from 'svelte';
  import { roadmapSystemApi as desktop } from '#lib/roadmap-system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let busy=$state(false),message=$state(''),error=$state('');let requestId=0;
  async function run(restore:boolean){const id=++requestId;busy=true;error='';message='';try{const count=await desktop.showDesktop(restore);if(id===requestId)message=`${restore?'Restored':'Minimized'} ${count} application windows.`;}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  onDestroy(()=>{requestId++;});
</script>
<div class="tool" aria-busy={busy}><Notice kind="info">Minimize normal application windows to reveal your desktop. Oxbox remembers the group so you can restore it. Applications are kept running.</Notice><div class="row"><button class="btn btn-primary" disabled={busy} onclick={()=>run(false)}>Show desktop</button><button class="btn" disabled={busy} onclick={()=>run(true)}>Restore minimized group</button></div>{#if message}<Notice kind="success">{message}</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-3)}</style>

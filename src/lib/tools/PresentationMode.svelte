<script lang="ts">
  import {onMount} from 'svelte';
  import {roadmapTimeApi,type PresentationOptions,type PresentationStatus} from '#lib/roadmap-time-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let options=$state<PresentationOptions>({display:true,hideDesktop:true,autoExternal:false}),status=$state<PresentationStatus|null>(null),busy=$state(false),loading=$state(true),error=$state(''),message=$state('');let requestId=0;
  onMount(()=>{let disposed=false,first=true;let timer:ReturnType<typeof setTimeout>;async function poll(){const id=requestId;try{const value=await roadmapTimeApi.presentationStatus();if(!disposed&&id===requestId){status=value;if(first){options={...value.options};first=false;}}}catch(e){if(!disposed&&id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(!disposed){loading=false;timer=setTimeout(poll,1000);}}}void poll();return()=>{disposed=true;requestId++;clearTimeout(timer);};});
  async function act(action:'start'|'stop'|'save'){const id=++requestId;busy=true;error='';message='';try{const value=action==='start'?await roadmapTimeApi.presentationStart({...options}):action==='stop'?await roadmapTimeApi.presentationStop():await roadmapTimeApi.presentationConfigure({...options});if(id===requestId){status=value;options={...value.options};message=action==='save'?'Presentation preferences saved.':'';}}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  async function notifications(){try{await roadmapTimeApi.dndOpenSettings();message='Open Focus / Do Not Disturb in system settings and choose your notification preference there.';}catch(e){error=e instanceof Error?e.message:String(e);}}
</script>
<div class="tool" aria-busy={busy||loading}>
  <Notice kind={status?.active?'success':'info'}>{status?.active?`Presentation mode is on${status.automatic?' because an additional display is connected':''}.`:loading?'Reading presentation state…':'Presentation mode is off.'}</Notice>
  <label><input type="checkbox" bind:checked={options.display} disabled={busy||!!status?.active}/> Keep the display on</label>
  <label><input type="checkbox" bind:checked={options.hideDesktop} disabled={busy||!!status?.active}/> Hide desktop icons while presenting (Windows/macOS)</label>
  <label><input type="checkbox" bind:checked={options.autoExternal} disabled={busy||!!status?.active}/> Start automatically when more than one display is connected</label>
  <p>Automatic display checks run only when you enable this option. Stopping manually suppresses automatic restart until the additional display disconnects.</p>
  <div class="row"><button class="btn btn-primary" disabled={busy||loading} onclick={()=>act(status?.active?'stop':'start')}>{status?.active?'Stop presentation':'Start presentation'}</button><button class="btn" disabled={busy||loading||!!status?.active} onclick={()=>act('save')}>Save automatic preferences</button><button class="btn" disabled={busy} onclick={notifications}>Open notification controls</button></div>
  <Notice kind="info">Oxbox owns its own awake request and restores the desktop visibility it found. Existing Do Not Sleep settings stay separate. Do Not Disturb stays under your control in system settings and is not automatically enabled or restored.</Notice>
  {#if message}<Notice kind="success">{message}</Notice>{/if}{#if error||status?.error}<Notice kind="error">{error||status?.error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;flex-wrap:wrap;gap:var(--s-2)}p{color:var(--text-2)}</style>

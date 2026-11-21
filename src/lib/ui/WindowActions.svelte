<script lang="ts">
  import { onMount,onDestroy } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { roadmapSystemApi as desktop,type DesktopWindow,type SnapPlacement } from '#lib/roadmap-system-api.ts';
  import Notice from './Notice.svelte';
  let {kind}:{kind:'top'|'snap'|'focus'|'close'}=$props();
  let windows=$state<DesktopWindow[]>([]),selected=$state<number[]>([]),target=$state(0);
  let placement=$state<SnapPlacement>('left-half'),opacity=$state(60),active=$state(false);
  let busy=$state(false),error=$state(''),message=$state('');
  let requestId=0;let alive=true;
  const current=$derived(windows.find((w)=>w.id===target));
  const placements:{value:SnapPlacement;label:string}[]=[{value:'left-half',label:'Left half'},{value:'right-half',label:'Right half'},{value:'top-half',label:'Top half'},{value:'bottom-half',label:'Bottom half'},{value:'left-third',label:'Left third'},{value:'center-third',label:'Center third'},{value:'right-third',label:'Right third'},{value:'top-left',label:'Top left quarter'},{value:'top-right',label:'Top right quarter'},{value:'bottom-left',label:'Bottom left quarter'},{value:'bottom-right',label:'Bottom right quarter'},{value:'full',label:'Full work area'}];
  async function refresh(){const id=++requestId;busy=true;error='';try{const next=await desktop.windows();if(id===requestId){windows=next;if(!next.some((w)=>w.id===target))target=next[0]?.id??0;selected=selected.filter((id)=>next.some((w)=>w.id===id));}}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  async function action(){if(busy)return;const id=++requestId;busy=true;error='';message='';try{if(kind==='close'){const targets=windows.filter((w)=>selected.includes(w.id));if(!await ask(`Ask these ${targets.length} windows to close?\n\n${targets.map((w)=>`${w.title} (PID ${w.pid})`).join('\n')}\n\nApplications can show their normal save prompts. Oxbox never kills processes.`,{title:'Close selected windows?',kind:'warning'}))return;if(id!==requestId)return;const count=await desktop.closeApps(targets.map((w)=>({id:w.id,pid:w.pid})),true);if(id===requestId){message=`Sent a normal close request to ${count} windows.`;selected=[];}}
    else if(kind==='focus' && active){await desktop.focusStop();if(id===requestId){active=false;message='Focus dimming stopped.';}}
    else{if(!current)throw new Error('Choose a window first.');const chosen=current;if(kind==='top'){await desktop.windowTop(chosen.id,chosen.pid,!chosen.topmost);if(id===requestId){windows=windows.map((w)=>w.id===chosen.id?{...w,topmost:!chosen.topmost}:w);message=chosen.topmost?'Window unpinned.':'Window pinned above other windows.';}}
      else if(kind==='snap'){await desktop.snap(chosen.id,chosen.pid,placement);if(id===requestId)message='Window placement applied. Applications can enforce their own minimum size.';}
      else{await desktop.focusStart(chosen.id,chosen.pid,opacity/100);if(id===requestId){active=true;message='Focus dimming is active. Return to Oxbox to stop it.';}}}}
    catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  onMount(()=>{void refresh();if(kind!=='focus')return;const poll=()=>desktop.focusStatus().then((value)=>{if(alive)active=value;}).catch(()=>{});void poll();const timer=setInterval(poll,1000);return()=>clearInterval(timer);});
  onDestroy(()=>{alive=false;requestId++;});
</script>
<div class="tool" aria-busy={busy}>
  <button class="btn" disabled={busy} onclick={refresh}>Refresh application windows</button>
  {#if kind==='close'}
    {#each windows as window}<label class="card choice"><input type="checkbox" checked={selected.includes(window.id)} disabled={busy} onchange={(event)=>{selected=event.currentTarget.checked?[...selected,window.id]:selected.filter((id)=>id!==window.id);}} /><span>{window.title}<small>PID {window.pid}</small></span></label>{/each}
  {:else}
    <label class="field"><span class="field-label">Application window</span><select class="input" bind:value={target} disabled={busy || active}><option value={0}>Choose a window</option>{#each windows as window}<option value={window.id} disabled={kind==='focus' && window.minimized}>{window.title}{window.topmost?' · Pinned':''}{window.minimized?' · Minimized':''}</option>{/each}</select></label>
  {/if}
  {#if kind==='snap'}<label class="field"><span class="field-label">Placement on the window's current monitor</span><select class="input" bind:value={placement} disabled={busy}>{#each placements as option}<option value={option.value}>{option.label}</option>{/each}</select></label>{/if}
  {#if kind==='focus'}<label class="field"><span class="field-label">Surrounding dim level ({opacity}%)</span><input class="input" type="range" min="10" max="85" bind:value={opacity} disabled={busy || active} /></label>{/if}
  <button class="btn btn-primary" disabled={busy || (kind==='close'?selected.length===0:(!current && !active))} onclick={action}>{kind==='close'?'Close selected windows…':kind==='top'?(current?.topmost?'Unpin window':'Pin window on top'):kind==='snap'?'Move window':active?'Stop focus dimming':'Focus selected window'}</button>
  {#if !busy && !windows.length && !error}<Notice kind="info">No eligible application windows are open. Open an application and refresh.</Notice>{/if}
  {#if kind==='top'}<Notice kind="info">Pin changes made by Oxbox are restored on Quit where the window still exists.</Notice>{/if}
  {#if kind==='focus'}<Notice kind="info">The shade follows your selected window and allows mouse clicks through. Use the launcher shortcut to return here and stop. It stops if the target closes, is minimized, or Oxbox quits.</Notice>{/if}
  {#if message}<Notice kind="success">{message}</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.choice{display:flex;gap:var(--s-3);padding:var(--s-3)}small{display:block;font-size:var(--text-xs);color:var(--text-3)}</style>

<script lang="ts">
  import { onMount,onDestroy } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { roadmapSystemApi as desktop,type DesktopDisplay,type ResolutionStatus } from '#lib/roadmap-system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let displays=$state<DesktopDisplay[]>([]),device=$state(''),modeIndex=$state(0);
  let status=$state<ResolutionStatus>({pending:null,error:null,kept:null});
  let busy=$state(false),error=$state(''),message=$state('');let requestId=0;let alive=true;
  const current=$derived(displays.find((d)=>d.device===device));const selected=$derived(current?.modes[modeIndex]);
  const remaining=$derived(status.pending?.remainingSeconds ?? 0);
  async function refresh(){const id=++requestId;busy=true;error='';try{const next=await desktop.displays();if(id===requestId){displays=next;if(!next.some((d)=>d.device===device))device=next[0]?.device??'';const display=next.find((d)=>d.device===device);modeIndex=Math.max(0,display?.modes.findIndex((m)=>m.width===display.current.width && m.height===display.current.height && m.refresh===display.current.refresh)??0);}}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  async function apply(){if(!selected || busy)return;const mode={...selected},id=++requestId;busy=true;error='';message='';try{if(!await ask(`Change ${current?.name ?? mode.device} to ${mode.width} × ${mode.height} at ${mode.refresh || 'default'} Hz?\n\nChoose Keep within 15 seconds. Oxbox otherwise restores the previous mode automatically, including when this view is closed.`,{title:'Try display mode?',kind:'warning'}))return;if(id!==requestId)return;const next=await desktop.resolutionApply(mode,true);if(id===requestId)status=next;}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  async function finish(keep:boolean){const id=++requestId;busy=true;error='';try{const next=await desktop.resolutionFinish(keep);if(id===requestId){status=next;message=next.kept===true?'Display mode kept for this Windows session.':'Previous display mode restored.';}}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId){busy=false;void refresh();}}}
  onMount(()=>{void refresh();const poll=()=>{void desktop.resolutionStatus().then((next)=>{if(alive)status=next;}).catch(()=>{});};poll();const timer=setInterval(poll,500);return()=>clearInterval(timer);});onDestroy(()=>{alive=false;requestId++;});
</script>
<div class="tool" aria-busy={busy}>
  <Notice kind="info">Choose a mode reported by the connected display driver. Changes are tested first and automatically revert after 15 seconds unless kept. Saved Windows display settings are not overwritten.</Notice>
  <button class="btn" disabled={busy || !!status.pending} onclick={refresh}>Refresh displays</button>
  <label class="field"><span class="field-label">Display</span><select class="input" bind:value={device} disabled={busy || !!status.pending} onchange={()=>{modeIndex=0;}}>{#each displays as display}<option value={display.device}>{display.name} · {display.device}</option>{/each}</select></label>
  {#if current}<p>Current: {current.current.width} × {current.current.height} · {current.current.refresh || 'Default'} Hz</p><label class="field"><span class="field-label">Resolution and refresh rate</span><select class="input" bind:value={modeIndex} disabled={busy || !!status.pending}>{#each current.modes as mode,index}<option value={index}>{mode.width} × {mode.height} · {mode.refresh || 'Default'} Hz</option>{/each}</select></label>{/if}
  {#if status.pending}<Notice kind="info">Keep this mode? Automatically reverting in {remaining} seconds.</Notice><div class="row"><button class="btn btn-primary" disabled={busy || remaining===0} onclick={()=>finish(true)}>Keep</button><button class="btn" disabled={busy} onclick={()=>finish(false)}>Revert now</button></div>{:else}<button class="btn btn-primary" disabled={busy || !selected} onclick={apply}>Try selected mode…</button>{/if}
  {#if status.error}<Notice kind="info">{status.error}</Notice>{/if}{#if message}<Notice kind="success">{message}</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-3)}</style>

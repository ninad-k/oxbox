<script lang="ts">
  import {onMount} from 'svelte';
  import {ask} from '@tauri-apps/plugin-dialog';
  import {isPermissionGranted,requestPermission} from '@tauri-apps/plugin-notification';
  import {inDesktopApp} from '#lib/ipc.ts';
  import {roadmapTimeApi,countdownText,type Ambiguity,type Recurrence,type TimeSnapshot} from '#lib/roadmap-time-api.ts';
  import Notice from './Notice.svelte';
  let {mode}:{mode:'alarm'|'countdown'}=$props();
  function initialDate(){const value=new Date(Date.now()+3600000);const part=(number:number)=>String(number).padStart(2,'0');return `${value.getFullYear()}-${part(value.getMonth()+1)}-${part(value.getDate())}T${part(value.getHours())}:${part(value.getMinutes())}`;}
  let label=$state(''),localDateTime=$state(initialDate()),zone=$state(Intl.DateTimeFormat().resolvedOptions().timeZone||'UTC'),recurrence=$state<Recurrence>('once'),ambiguity=$state<Ambiguity>('earlier'),notify=$state(true),zones=$state<string[]>([]),snapshot=$state<TimeSnapshot>({alarms:[],countdowns:[],error:null}),busy=$state(false),loading=$state(true),error=$state(''),message=$state(''),permissionNotice=$state('');let requestId=0;
  async function refresh(id=requestId){const value=await roadmapTimeApi.roadmapTimeSnapshot();if(id===requestId)snapshot=value;}
  onMount(()=>{let disposed=false;let timer:ReturnType<typeof setTimeout>;
    void roadmapTimeApi.roadmapTimeZones().then(value=>{if(!disposed)zones=value;}).catch(e=>{if(!disposed)error=String(e);});
    async function poll(){const id=requestId;try{await refresh(id);}catch(e){if(!disposed&&id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(!disposed){loading=false;timer=setTimeout(poll,1000);}}}
    void poll();return()=>{disposed=true;requestId++;clearTimeout(timer);};
  });
  async function action(work:()=>Promise<unknown>){const id=++requestId;busy=true;error='';message='';try{await work();await refresh(id);}catch(e){if(id===requestId)error=e instanceof Error?e.message:String(e);}finally{if(id===requestId)busy=false;}}
  async function create(){await action(async()=>{
    if(inDesktopApp()&&(mode==='alarm'||notify)&&!await isPermissionGranted()&&await requestPermission()!=='granted')permissionNotice='System notifications are disabled. Due reminders remain visible here while Oxbox is open.';
    if(mode==='alarm')await roadmapTimeApi.alarmAdd({label,localDateTime,zone,recurrence,ambiguity});else await roadmapTimeApi.countdownAdd({label,localDateTime,zone,notify,ambiguity});
    label='';message=mode==='alarm'?'Alarm saved.':'Date countdown saved.';
  });}
  async function remove(id:number,name:string){await action(async()=>{if(!await ask(`Remove “${name}” from ${mode==='alarm'?'alarms':'date countdowns'}?`,{title:'Remove saved reminder?',kind:'warning',okLabel:'Remove',cancelLabel:'Keep'}))return;if(mode==='alarm')await roadmapTimeApi.alarmRemove(id);else await roadmapTimeApi.countdownRemove(id);});}
  function instant(timestamp:number,timeZone:string){return new Date(timestamp*1000).toLocaleString(undefined,{timeZone});}
</script>
<div class="tool" aria-busy={busy||loading}>
  <Notice kind="info">Saved reminders run in Rust while the launcher is hidden. Keep Oxbox running; it cannot wake a sleeping or powered-off computer. A missed alarm is delivered once when Oxbox resumes. Notification delivery follows your OS notification settings.</Notice>
  <form class="card form" onsubmit={event=>{event.preventDefault();void create();}}>
    <label class="field"><span class="field-label">Name</span><input class="input" bind:value={label} maxlength="200" required disabled={busy} placeholder={mode==='alarm'?'Morning reminder':'Project launch'}/></label>
    <label class="field"><span class="field-label">{mode==='alarm'?'First alarm date and time':'Target date and time'} (in the selected time zone)</span><input class="input" type="datetime-local" bind:value={localDateTime} required disabled={busy}/></label>
    <label class="field"><span class="field-label">IANA time zone</span><input class="input" list="reminder-zones" bind:value={zone} required disabled={busy}/><datalist id="reminder-zones">{#each zones as value}<option {value}></option>{/each}</datalist></label>
    {#if mode==='alarm'}<label class="field"><span class="field-label">Repeat</span><select class="input" bind:value={recurrence} disabled={busy}><option value="once">Once</option><option value="daily">Every day at this local time</option><option value="weekdays">Monday–Friday at this local time</option></select></label>{:else}<label><input type="checkbox" bind:checked={notify} disabled={busy}/> Notify when the date is reached</label>{/if}
    <label class="field"><span class="field-label">When daylight saving repeats this local time</span><select class="input" bind:value={ambiguity} disabled={busy}><option value="earlier">Use the first occurrence</option><option value="later">Use the second occurrence</option></select></label>
    <p>Nonexistent one-time dates are rejected. Repeating alarms skip a day if their local time is skipped by daylight saving.</p>
    <div><button class="btn btn-primary" disabled={busy||loading||!label.trim()||!localDateTime||!zone.trim()}>{busy?'Saving…':mode==='alarm'?'Add alarm':'Add countdown'}</button></div>
  </form>
  {#if permissionNotice}<Notice kind="info">{permissionNotice}</Notice>{/if}
  {#if mode==='alarm'}
    {#each snapshot.alarms as alarm (alarm.id)}<article class="card item"><div><h2>{alarm.input.label}</h2><p>{alarm.input.localDateTime.replace('T',' ')} · {alarm.input.zone} · {alarm.input.recurrence}</p><p>Next: {instant(alarm.nextAt,alarm.input.zone)} {alarm.input.zone} · {alarm.enabled?'Enabled':'Off'}</p></div>
      {#if alarm.ringing}<Notice kind="success">Alarm due: {alarm.input.label}</Notice><div class="row"><button class="btn" disabled={busy} onclick={()=>action(()=>roadmapTimeApi.alarmSnooze(alarm.id))}>Snooze 5 minutes</button><button class="btn" disabled={busy} onclick={()=>action(()=>roadmapTimeApi.alarmDismiss(alarm.id))}>Dismiss</button></div>{/if}
      <div class="row"><button class="btn" disabled={busy} onclick={()=>action(()=>roadmapTimeApi.alarmSetEnabled(alarm.id,!alarm.enabled))}>{alarm.enabled?'Disable':'Enable'}</button><button class="btn" disabled={busy} onclick={()=>remove(alarm.id,alarm.input.label)}>Remove</button></div>
    </article>{:else}{#if !loading}<Notice kind="info">No saved alarms.</Notice>{/if}{/each}
  {:else}
    {#each snapshot.countdowns as countdown (countdown.id)}<article class="card item"><h2>{countdown.input.label}</h2><strong class="mono">{countdownText(countdown.remainingSeconds)}</strong><p>{countdown.input.localDateTime.replace('T',' ')} · {countdown.input.zone}</p><p>{countdown.input.notify?(countdown.notified?'Date reached; notification handled.':'Notification enabled.'):'Notification off.'}</p><div><button class="btn" disabled={busy} onclick={()=>remove(countdown.id,countdown.input.label)}>Remove</button></div></article>{:else}{#if !loading}<Notice kind="info">Add a date to track a countdown.</Notice>{/if}{/each}
  {/if}
  {#if message}<Notice kind="success">{message}</Notice>{/if}{#if error||snapshot.error}<Notice kind="error">{error||snapshot.error}</Notice>{/if}
</div>
<style>.tool,.form,.item{display:grid;gap:var(--s-4)}.form,.item{padding:var(--s-4)}.row{display:flex;gap:var(--s-2);flex-wrap:wrap}h2{font-size:var(--text-lg)}p{color:var(--text-2);font-size:var(--text-sm)}.mono{font-size:var(--text-xl)}</style>

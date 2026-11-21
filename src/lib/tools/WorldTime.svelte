<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type WorldClock } from '#lib/api.ts';
  import { settings } from '#lib/settings.svelte.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let zones=$state<string[]>([]), query=$state(''), selected=$state(''), clocks=$state<WorldClock[]>([]), error=$state(''), busy=$state(false);
  let requestId=0;let disposed=false;
  const matches=$derived(zones.filter(z=>!settings.data.worldZones.includes(z)&&z.toLowerCase().replaceAll('_',' ').includes(query.toLowerCase())).slice(0,100));
  $effect(()=>{if(!matches.includes(selected))selected=matches[0]??'';});
  async function refresh(){if(disposed)return;const id=++requestId;busy=true;try{const values=await api.worldClocks([...settings.data.worldZones]);if(id===requestId){clocks=values;error='';}}catch(e){if(id===requestId)error=String(e);}finally{if(id===requestId)busy=false;}}
  onMount(()=>{
    let timer:ReturnType<typeof setTimeout>;
    async function poll(){await refresh();if(!disposed)timer=setTimeout(poll,1000);}
    void settings.init().then(async()=>{const all=await api.timeZones();if(disposed)return;zones=all;await poll();}).catch(e=>{if(!disposed)error=String(e);});
    return()=>{disposed=true;requestId++;clearTimeout(timer);};
  });
  async function setZones(change:(current:string[])=>string[]){try{await settings.update(current=>({worldZones:change([...current.worldZones])}));await refresh();}catch(e){error=String(e);}}
  function move(zone:string,delta:number){void setZones(list=>{const index=list.indexOf(zone);if(index<0||index+delta<0||index+delta>=list.length)return list;[list[index],list[index+delta]]=[list[index+delta],list[index]];return list;});}
  function offset(seconds:number){const minutes=Math.round(seconds/60);return `${minutes>=0?'+':'−'}${String(Math.floor(Math.abs(minutes)/60)).padStart(2,'0')}:${String(Math.abs(minutes)%60).padStart(2,'0')}`;}
</script>
<div class="tool" aria-busy={busy}>
  <div class="row"><label class="field"><span class="field-label">Search IANA time zones</span><input class="input" type="search" bind:value={query} placeholder="City or region"/></label>
    <label class="field"><span class="field-label">Time zone</span><select class="input" bind:value={selected}>{#each matches as z}<option value={z}>{z.replaceAll('_',' ')}</option>{/each}</select></label>
    <button class="btn" disabled={!matches.includes(selected) || settings.data.worldZones.length>=200} onclick={()=>{const zone=selected;void setZones(list=>list.includes(zone)?list:[...list,zone]);}}>Add clock</button></div>
  <p>Showing up to 100 matches. Offsets follow the bundled time-zone database, including daylight saving.</p>
  {#if !settings.data.worldZones.length}<Notice kind="info">Add a time zone to see its clock.</Notice>{/if}
  {#each settings.data.worldZones as zone,index (zone)}
    {@const clock=clocks.find(c=>c.zone===zone)}
    <article class="card clock"><div><h2>{zone.replaceAll('_',' ')}</h2>
      {#if clock}<p class="time mono">{clock.time}</p><p>{clock.date} · {clock.abbreviation} · UTC{offset(clock.offsetSeconds)} · {clock.daytime?'Day':'Night'}</p>
        <p>{offset(clock.offsetSeconds + new Date().getTimezoneOffset()*60)} from local time</p>{/if}</div>
      <div class="row"><button class="btn" aria-label={`Move ${zone} up`} disabled={index===0} onclick={()=>move(zone,-1)}>Up</button><button class="btn" aria-label={`Move ${zone} down`} disabled={index===settings.data.worldZones.length-1} onclick={()=>move(zone,1)}>Down</button><button class="btn" aria-label={`Remove ${zone}`} onclick={()=>setZones(list=>list.filter(z=>z!==zone))}>Remove</button></div>
    </article>
  {/each}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-2);align-items:end;flex-wrap:wrap}.row .field{flex:1}.clock{padding:var(--s-4);display:flex;justify-content:space-between;gap:var(--s-4);flex-wrap:wrap}.time{font-size:var(--s-7)}h2{font-size:var(--text-lg)}p{color:var(--text-2);font-size:var(--text-sm)}</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type ConversionUnit } from '#lib/api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  import CopyButton from '#lib/ui/CopyButton.svelte';
  import CurrencyConverter from './CurrencyConverter.svelte';
  let mode=$state('measurements');
  let units=$state<ConversionUnit[]>([]), category=$state('length'), from=$state('m'), to=$state('km'), value=$state('1'), result=$state(''), error=$state(''), busy=$state(false);
  let requestId=0;
  const choices=$derived(units.filter(u=>u.category===category));
  const categories=$derived([...new Set(units.map(u=>u.category))]);
  onMount(()=>{let disposed=false; api.conversionUnits().then(u=>{if(!disposed)units=u;}).catch(e=>{if(!disposed)error=String(e);});return()=>{disposed=true;requestId++;};});
  function selectCategory(next:string){category=next;const items=units.filter(u=>u.category===next);from=items[0]?.id??'';to=items[1]?.id??from;}
  $effect(()=>{
    const input=value, source=from, target=to; const id=++requestId;
    result='';error='';busy=false;
    if(!input.trim())return;
    const number=Number(input); if(!Number.isFinite(number)){error='Enter a finite number.';return;}
    busy=true;
    api.convertUnit(number,source,target).then(v=>{if(id===requestId)result=String(Number(v.toPrecision(12)));}).catch(e=>{if(id===requestId)error=String(e);}).finally(()=>{if(id===requestId)busy=false;});
  });
</script>
<div class="tool" aria-busy={busy}>
  <div class="row"><button class="btn" aria-pressed={mode==='measurements'} onclick={()=>{mode='measurements';}}>Measurement units</button><button class="btn" aria-pressed={mode==='currency'} onclick={()=>{mode='currency';}}>Currency reference rates</button></div>
  {#if mode==='currency'}<CurrencyConverter/>{:else}
  <label class="field"><span class="field-label">Category</span><select class="input" value={category} onchange={e=>selectCategory(e.currentTarget.value)}>{#each categories as cat}<option value={cat}>{cat}</option>{/each}</select></label>
  <label class="field"><span class="field-label">Value</span><input class="input mono" inputmode="decimal" bind:value /></label>
  <div class="row"><label class="field"><span class="field-label">From</span><select class="input" bind:value={from}>{#each choices as u}<option value={u.id}>{u.label}</option>{/each}</select></label>
  <button class="btn" onclick={()=>{[from,to]=[to,from];}}>Swap units</button>
  <label class="field"><span class="field-label">To</span><select class="input" bind:value={to}>{#each choices as u}<option value={u.id}>{u.label}</option>{/each}</select></label></div>
  <label class="field"><span class="field-label">Result</span><input class="input mono" readonly value={result} /></label><CopyButton text={result}/>
  {#if !value.trim()}<Notice kind="info">Enter a value to convert.</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
  {/if}
</div>
<style>.tool{display:grid;gap:var(--s-4);max-width:720px}.row{display:flex;align-items:end;gap:var(--s-3);flex-wrap:wrap}.row .field{flex:1}</style>

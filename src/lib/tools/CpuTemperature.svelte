<script lang="ts">
  import { onMount,onDestroy } from 'svelte';import { roadmapSystemApi as desktop,type TemperatureSensor } from '#lib/roadmap-system-api.ts';import Notice from '#lib/ui/Notice.svelte';
  let sensors=$state<TemperatureSensor[]>([]),loaded=$state(false),busy=$state(false),error=$state('');let alive=true;let polling=false;
  async function sample(){if(polling)return;polling=true;busy=true;try{const next=await desktop.cpuTemperatures();if(alive){sensors=next;loaded=true;error='';}}catch(e){if(alive)error=e instanceof Error?e.message:String(e);}finally{polling=false;if(alive)busy=false;}}
  onMount(()=>{void sample();const timer=setInterval(sample,2000);return()=>clearInterval(timer);});onDestroy(()=>{alive=false;});
</script>
<div class="tool" aria-busy={busy}><p>CPU sensor readings refresh every two seconds while this view is open.</p>{#each sensors as sensor}<div class="card sensor"><strong>{sensor.label}</strong><output>{sensor.temperature.toFixed(1)} °C</output>{#if sensor.critical!==null}<span>Critical: {sensor.critical.toFixed(1)} °C</span>{/if}</div>{/each}{#if loaded && !sensors.length}<Notice kind="info">No CPU temperature sensor is exposed by this operating system or hardware. This is common on Windows. Oxbox does not install monitoring drivers or invent a reading.</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}</div>
<style>.tool{display:grid;gap:var(--s-4)}.sensor{display:flex;align-items:center;gap:var(--s-4);padding:var(--s-4)}output{font-size:var(--text-xl);font-variant-numeric:tabular-nums}</style>

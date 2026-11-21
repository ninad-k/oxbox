<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { roadmapSystemApi, type FocusFrame } from '#lib/roadmap-system-api.ts';
  let frame=$state<FocusFrame|null>(null);
  onMount(()=>{let disposed=false;let stop:(()=>void)|undefined;listen<FocusFrame>('focus://frame',(event)=>{if(!disposed)frame=event.payload;}).then((unlisten)=>{if(disposed)unlisten();else stop=unlisten;});roadmapSystemApi.focusFrame().then((next)=>{if(!disposed)frame=next;}).catch(()=>{});return()=>{disposed=true;stop?.();};});
</script>
<svg aria-hidden="true" width="100%" height="100%">
  <defs><mask id="focus-cutout"><rect width="100%" height="100%" fill="white" />{#if frame?.cutout}<rect x={frame.cutout.x} y={frame.cutout.y} width={frame.cutout.width} height={frame.cutout.height} fill="black" />{/if}</mask></defs>
  {#if frame}<rect width="100%" height="100%" fill="var(--focus-shade)" opacity={frame.opacity} mask="url(#focus-cutout)" />{/if}
</svg>
<style>:global(html),:global(body){background:transparent!important;overflow:hidden}svg{display:block;position:fixed;inset:0;pointer-events:none}</style>

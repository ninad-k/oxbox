<script lang="ts">
  import { onMount } from 'svelte';import {roadmapMediaApi}from'#lib/roadmap-media-api.ts';import Notice from'#lib/ui/Notice.svelte';
  let muted=$state<boolean|null>(null),busy=$state(false),error=$state('');let id=0;
  async function refresh(){const request=++id;busy=true;error='';try{const value=await roadmapMediaApi.microphoneStatus();if(request===id)muted=value;}catch(cause){if(request===id)error=String(cause);}finally{if(request===id)busy=false;}}
  async function set(value:boolean){const request=++id;busy=true;error='';try{const next=await roadmapMediaApi.microphoneSet(value);if(request===id)muted=next;}catch(cause){if(request===id)error=String(cause);}finally{if(request===id)busy=false;}}
  onMount(()=>{void refresh();return()=>{id++;};});
</script>
<div class="tool" aria-busy={busy}><Notice kind="info">Controls the default communications microphone on Windows, or the default PulseAudio/PipeWire source on Linux. Other microphones may have separate controls. Your mute choice remains after Oxbox quits; Oxbox never unmutes automatically.</Notice><p>Default microphone: <strong>{muted===null?'Unknown':muted?'Muted':'Unmuted'}</strong></p><div class="row"><button class="btn btn-primary" onclick={()=>set(!muted)} disabled={busy||muted===null}>{muted?'Unmute default microphone':'Mute default microphone'}</button><button class="btn" onclick={refresh} disabled={busy}>Refresh status</button></div>{#if error}<Notice kind="error">{error}</Notice>{/if}</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-3);flex-wrap:wrap}</style>

<script lang="ts">
  import{onMount}from'svelte';import{listen}from'@tauri-apps/api/event';import{inDesktopApp}from'#lib/ipc.ts';import{roadmapMediaApi,type RecordingStatus}from'#lib/roadmap-media-api.ts';import Notice from './Notice.svelte';
  let status=$state<RecordingStatus|null>(null),busy=$state(false),error=$state(''),dismissed=$state('');let mounted=false,polling=false;
  async function refresh(){if(polling)return;polling=true;try{const next=await roadmapMediaApi.recordingStatus();if(mounted)status=next;}catch(cause){if(mounted)error=String(cause);}finally{polling=false;}}
  async function stop(){if(!status?.active)return;busy=true;error='';try{const next=await roadmapMediaApi.recordingStop(status.id);if(mounted)status=next;}catch(cause){if(mounted)error=String(cause);}finally{if(mounted)busy=false;}}
  onMount(()=>{mounted=true;if(!inDesktopApp())return()=>{mounted=false;};void refresh();const timer=setInterval(()=>void refresh(),500);let unlisten:(()=>void)|undefined;void listen<string>('tool://recording/error',event=>{if(mounted)error=event.payload;}).then(stop=>{if(mounted)unlisten=stop;else stop();});return()=>{mounted=false;clearInterval(timer);unlisten?.();};});
</script>
{#if status?.active}<div class="banner" aria-live="polite" aria-busy={busy}><Notice kind="error">Recording active: {status.kind}. {status.output}</Notice><button class="btn" onclick={stop} disabled={busy}>{busy?'Stopping…':'Stop and save recording'}</button></div>
{:else if status?.id&&status.id!==dismissed}<div class="banner"><Notice kind={status.error?'error':'success'}>{status.error??`Recording saved to ${status.output}.`}</Notice><button class="btn" onclick={()=>dismissed=status?.id??''}>Dismiss</button></div>{/if}
{#if error}<Notice kind="error">{error}</Notice>{/if}
<style>.banner{display:flex;gap:var(--s-3);align-items:center;flex-wrap:wrap;padding:var(--s-3);border-bottom:1px solid var(--border)}.banner :global(.notice){flex:1;overflow-wrap:anywhere}</style>

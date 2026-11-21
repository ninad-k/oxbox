<script lang="ts">
  import { onMount } from 'svelte';
  import { ask,save } from '@tauri-apps/plugin-dialog';
  import { guessOs } from '#lib/api.ts';
  import { mediaApi } from '#lib/media-api.ts';
  import { roadmapMediaApi,type RecordingStatus,type ScreenSource } from '#lib/roadmap-media-api.ts';
  import MediaProvider from './MediaProvider.svelte';
  import Notice from './Notice.svelte';
  let {kind}:{kind:'audio'|'screen'|'camera'}=$props();
  const os=guessOs();
  function initialDevice(){return os==='linux'?(kind==='audio'?'default':'/dev/video0'):os==='macos'?'0':'';}
  let device=$state(initialDevice());
  let output=$state(''),seconds=$state(300),fps=$state(15),mode=$state<'desktop'|'area'|'window'>('desktop'),title=$state('');
  let x=$state(0),y=$state(0),width=$state(1280),height=$state(720);
  let windows=$state<string[]>([]),status=$state<RecordingStatus|null>(null),busy=$state(false),error=$state('');
  let requestId=0,mounted=false,polling=false;
  const extension=$derived(kind==='audio'?'wav':'mp4');
  async function refresh(){if(polling)return;polling=true;try{const next=await roadmapMediaApi.recordingStatus();if(mounted)status=next;}catch(cause){if(mounted)error=String(cause);}finally{polling=false;}}
  async function chooseOutput(){const id=requestId;try{const chosen=await save({defaultPath:`oxbox-${kind}.${extension}`,filters:[{name:extension.toUpperCase(),extensions:[extension]}]});if(id===requestId&&chosen)output=chosen;}catch(cause){if(id===requestId)error=String(cause);}}
  async function listWindows(){const id=requestId;try{const targets=await mediaApi.captureTargets();if(id===requestId)windows=targets.filter(target=>target.kind==='window').map(target=>target.name);}catch(cause){if(id===requestId)error=String(cause);}}
  async function start(){
    const id=++requestId;busy=true;error='';
    try{
      const subject=kind==='audio'?`microphone “${device}”`:kind==='camera'?`camera “${device}” without microphone audio`:'screen without microphone audio';
      if(!await ask(`Record ${subject} for up to ${seconds} seconds?\n\nOutput: ${output}\n\nRecording continues if the launcher is hidden. Return to Oxbox to stop it.`,{title:'Start recording?',kind:'warning'})||id!==requestId)return;
      let next:RecordingStatus;
      if(kind==='audio')next=await roadmapMediaApi.recordAudioStart(device,seconds,output);
      else if(kind==='camera')next=await roadmapMediaApi.takeVideoStart(device,seconds,output);
      else {const source:ScreenSource=mode==='desktop'?{kind:'desktop'}:mode==='window'?{kind:'window',title}:{kind:'area',x,y,width,height};next=await roadmapMediaApi.recordScreenStart({source,fps,seconds},output);}
      if(id===requestId)status=next;
    }catch(cause){if(id===requestId)error=String(cause);}finally{if(id===requestId)busy=false;}
  }
  async function stop(){if(!status?.active)return;busy=true;error='';try{const next=await roadmapMediaApi.recordingStop(status.id);if(mounted)status=next;}catch(cause){if(mounted)error=String(cause);}finally{if(mounted)busy=false;}}
  onMount(()=>{mounted=true;void refresh();const timer=setInterval(()=>void refresh(),500);return()=>{mounted=false;requestId++;clearInterval(timer);};});
</script>
<div class="tool" aria-busy={busy}>
  <MediaProvider/>
  <Notice kind="info">Recording starts only after confirmation. One recording can run at a time. Stop saves the recording; Quit stops it. Existing files are never overwritten. Limit: 24 hours or 20 GiB. {kind==='audio'?'This captures the selected microphone.':'No microphone audio is captured.'}</Notice>
  {#if kind!=='screen'}<label class="field"><span class="field-label">{kind==='audio'?'Microphone':'Camera'} device {os==='windows'?'(exact FFmpeg DirectShow name)':os==='macos'?'(AVFoundation index)':'(local device)'}</span><input class="input" bind:value={device} disabled={busy||status?.active}/></label><p class="hint">{os==='windows'?'Find exact names with: ffmpeg -list_devices true -f dshow -i dummy':os==='macos'?'List indices with: ffmpeg -f avfoundation -list_devices true -i ""':kind==='audio'?'Use “default” for the local PulseAudio/PipeWire source.':'Use a local V4L2 device, for example /dev/video0.'}</p>
  {:else}
    <label class="field"><span class="field-label">Capture source</span><select class="input" bind:value={mode} disabled={busy||status?.active}><option value="desktop">Desktop</option><option value="area">Area (physical pixels)</option><option value="window" disabled={os!=='windows'}>Window (Windows)</option></select></label>
    {#if mode==='area'}<div class="row">{#each ['X','Y','Width','Height'] as label,index}<label class="field"><span class="field-label">{label}</span>{#if index===0}<input class="input" type="number" bind:value={x} disabled={busy||status?.active}/>{:else if index===1}<input class="input" type="number" bind:value={y} disabled={busy||status?.active}/>{:else if index===2}<input class="input" type="number" min="1" max="16384" bind:value={width} disabled={busy||status?.active}/>{:else}<input class="input" type="number" min="1" max="16384" bind:value={height} disabled={busy||status?.active}/>{/if}</label>{/each}</div>{/if}
    {#if mode==='window'}<label class="field"><span class="field-label">Exact window title</span><input class="input" list="recording-windows" bind:value={title} disabled={busy||status?.active}/><datalist id="recording-windows">{#each windows as name}<option value={name}></option>{/each}</datalist></label><button class="btn" onclick={listWindows} disabled={busy||status?.active}>Find windows</button>{/if}
    <label class="field"><span class="field-label">Frame rate</span><input class="input" type="number" min="1" max="60" bind:value={fps} disabled={busy||status?.active}/></label>
    {#if os==='linux'}<Notice kind="info">Requires a local X11 session. Wayland and Linux window capture are not available through this provider; select an area under X11.</Notice>{/if}
  {/if}
  <label class="field"><span class="field-label">Maximum duration (seconds)</span><input class="input" type="number" min="1" max="86400" bind:value={seconds} disabled={busy||status?.active}/></label>
  <div class="row"><button class="btn" onclick={chooseOutput} disabled={busy||status?.active}>Choose new output</button><span class="path">{output||`Choose a .${extension} filename`}</span></div>
  <div class="row"><button class="btn btn-primary" onclick={start} disabled={busy||status?.active||!output||(kind!=='screen'&&!device)||(kind==='screen'&&mode==='window'&&!title)}>Start recording</button>{#if status?.active}<button class="btn" onclick={stop} disabled={busy}>Stop and save recording</button>{/if}</div>
  {#if status?.active}<Notice kind="error">Recording active: {status.kind}. Started {new Date(status.startedAt*1000).toLocaleTimeString()}. Destination: {status.output}</Notice>{:else if status?.output&&!status.error}<Notice kind="success">Recording saved to {status.output}.</Notice>{/if}
  {#if status?.error}<Notice kind="error">{status.error}</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-3);align-items:center;flex-wrap:wrap}.row .field{flex:1;min-width:var(--s-8)}.path{overflow-wrap:anywhere;color:var(--text-2)}.hint{font-size:var(--text-sm);color:var(--text-3)}.btn{justify-self:start}</style>

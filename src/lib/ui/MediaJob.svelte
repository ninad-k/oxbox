<script lang="ts">
  import { onDestroy } from 'svelte';import {ask,open,save}from'@tauri-apps/plugin-dialog';import{guessOs}from'#lib/api.ts';
  import{roadmapMediaApi,type MediaFormat}from'#lib/roadmap-media-api.ts';import{createJob,type JobHandle,type JobProgress}from'#lib/jobs.ts';
  import MediaProvider from './MediaProvider.svelte';import Notice from './Notice.svelte';import ProgressBar from './ProgressBar.svelte';
  let{mode}:{mode:'convert'|'gif'|'photo'}=$props();
  let input=$state(''),output=$state(''),format=$state<MediaFormat>('mp4'),start=$state(0),seconds=$state(5),fps=$state(10),width=$state(640);
  const os=guessOs();let device=$state(os==='macos'?'0':os==='linux'?'/dev/video0':'');
  let busy=$state(false),error=$state(''),result=$state(''),progress=$state<JobProgress|null>(null);let job:JobHandle|null=null,requestId=0;
  const extension=$derived(mode==='gif'?'gif':mode==='photo'?'png':format);
  const toolId=$derived(mode==='gif'?'make-gif':mode==='photo'?'take-photo':'convert-audio-video');
  async function pickInput(){const id=requestId;try{const chosen=await open({multiple:false,title:'Choose local media file'});if(id===requestId&&typeof chosen==='string'){input=chosen;result='';}}catch(cause){if(id===requestId)error=String(cause);}}
  async function pickOutput(){const id=requestId;try{const chosen=await save({defaultPath:`oxbox-output.${extension}`,filters:[{name:extension.toUpperCase(),extensions:[extension]}]});if(id===requestId&&chosen)output=chosen;}catch(cause){if(id===requestId)error=String(cause);}}
  async function run(){const id=++requestId;busy=true;error='';result='';progress=null;
    try{
      if(mode==='photo'&&(!await ask(`Use camera “${device}” to take one PNG photo? No microphone audio is captured.\n\nOutput: ${output}`,{title:'Take photo?',kind:'warning'})||id!==requestId))return;
      job=createJob(toolId,next=>{if(id===requestId)progress=next;});
      const path=await job.run(jobId=>mode==='gif'?roadmapMediaApi.makeGif(jobId,input,output,{startSeconds:start,seconds,fps,width}):mode==='photo'?roadmapMediaApi.takePhoto(jobId,device,output):roadmapMediaApi.convertMedia(jobId,input,output,format));
      if(id===requestId)result=path;
    }catch(cause){if(id===requestId)error=String(cause);}finally{if(id===requestId){busy=false;job=null;}}
  }
  async function cancel(){try{await job?.cancel();}catch(cause){error=String(cause);}}
  onDestroy(()=>{requestId++;void job?.cancel().catch(()=>{});});
</script>
<div class="tool" aria-busy={busy}>
  <MediaProvider/>
  <Notice kind="info">{mode==='photo'?'The camera opens only after you confirm Take photo. One still image is captured; no microphone is opened.':'Uses local files and your installed FFmpeg. Network protocols and playlist inputs are blocked. Originals are kept.'} Outputs must use a new filename. Provider jobs support cancellation and are limited to two hours or 20 GiB.</Notice>
  {#if mode==='photo'}<label class="field"><span class="field-label">Camera device {os==='windows'?'(exact DirectShow name)':os==='macos'?'(AVFoundation index)':'(/dev/videoN)'}</span><input class="input" bind:value={device} disabled={busy}/></label><p class="hint">{os==='windows'?'List names manually with: ffmpeg -list_devices true -f dshow -i dummy':os==='macos'?'List indices manually with: ffmpeg -f avfoundation -list_devices true -i ""':'Choose an existing local V4L2 camera, such as /dev/video0.'}</p>
  {:else}<div class="row"><button class="btn" onclick={pickInput} disabled={busy}>Choose local media file</button><span class="path">{input||'No input selected'}</span></div>{/if}
  {#if mode==='convert'}<label class="field"><span class="field-label">Output format</span><select class="input" bind:value={format} onchange={()=>output=''} disabled={busy}><option value="mp4">MP4 (MPEG-4 video / AAC)</option><option value="webm">WebM (VP9 / Opus)</option><option value="mp3">MP3</option><option value="wav">WAV (PCM)</option><option value="flac">FLAC</option></select></label><p class="hint">Your FFmpeg build must include the selected encoder. Video presets keep the source dimensions and frame rate; audio presets discard video.</p>{/if}
  {#if mode==='gif'}<div class="row"><label class="field"><span class="field-label">Start (seconds)</span><input class="input" type="number" min="0" step="0.1" bind:value={start} disabled={busy}/></label><label class="field"><span class="field-label">Duration (0.1–60 seconds)</span><input class="input" type="number" min="0.1" max="60" step="0.1" bind:value={seconds} disabled={busy}/></label></div><div class="row"><label class="field"><span class="field-label">Frames per second (1–30)</span><input class="input" type="number" min="1" max="30" bind:value={fps} disabled={busy}/></label><label class="field"><span class="field-label">Width (16–1280 pixels)</span><input class="input" type="number" min="16" max="1280" bind:value={width} disabled={busy}/></label></div>{/if}
  <div class="row"><button class="btn" onclick={pickOutput} disabled={busy}>Choose new output</button><span class="path">{output||`Choose a .${extension} filename`}</span></div>
  <div class="row"><button class="btn btn-primary" onclick={run} disabled={busy||!output||(mode==='photo'?!device:!input)}>{busy?'Working…':mode==='photo'?'Take photo':mode==='gif'?'Make GIF':'Convert file'}</button>{#if busy}<button class="btn" onclick={cancel}>Cancel</button>{/if}</div>
  {#if busy && progress}<ProgressBar {progress}/>{/if}{#if result}<Notice kind="success">Saved to {result}.</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4)}.row{display:flex;gap:var(--s-3);align-items:center;flex-wrap:wrap}.row .field{flex:1;min-width:var(--s-8)}.path{overflow-wrap:anywhere;color:var(--text-2)}.hint{font-size:var(--text-sm);color:var(--text-3)}</style>

<script lang="ts">
  import { onDestroy } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import { api, type CodeKind } from '#lib/api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let text=$state(''),kind=$state<CodeKind>('qr'),correction=$state('M'),size=$state(512),svg=$state(''),error=$state(''),message=$state(''),busy=$state(false),saving=$state(false);
  let requestId=0;
  onDestroy(()=>{requestId++;});
  $effect(()=>{
    const args=[text,kind,correction,size] as const, id=++requestId;
    svg='';error='';message='';busy=false;
    if(!args[0])return;
    busy=true;
    const timer=setTimeout(()=>{api.generateCode(...args).then(value=>{if(id===requestId)svg=value;}).catch(e=>{if(id===requestId)error=String(e);}).finally(()=>{if(id===requestId)busy=false;});},180);
    return()=>{clearTimeout(timer);};
  });
  async function exportCode(asSvg:boolean){
    saving=true;error='';message=''; const args=[text,kind,correction,size] as const;
    try{const ext=asSvg?'svg':'png';const path=await save({defaultPath:`oxbox-code.${ext}`,filters:[{name:ext.toUpperCase(),extensions:[ext]}]});if(!path)return;await api.saveCode(...args,path,asSvg);message=`Saved to ${path}`;}
    catch(e){error=String(e);}finally{saving=false;}
  }
  async function copy(){saving=true;error='';try{await api.copyCode(text,kind,correction,size);message='Image copied.';}catch(e){error=String(e);}finally{saving=false;}}
</script>
<div class="tool" aria-busy={busy || saving}>
  <label class="field"><span class="field-label">Code type</span><select class="input" bind:value={kind} disabled={saving}><option value="qr">QR code</option><option value="code128">Code 128</option><option value="ean13">EAN-13</option></select></label>
  <label class="field"><span class="field-label">Text or barcode digits</span><textarea class="textarea" bind:value={text} maxlength="4096" disabled={saving} placeholder="Enter text, a link, or barcode digits"></textarea></label>
  <div class="row"><label class="field"><span class="field-label">QR error correction</span><select class="input" bind:value={correction} disabled={kind!=='qr' || saving}>{#each ['L','M','Q','H'] as ec}<option>{ec}</option>{/each}</select></label>
    <label class="field"><span class="field-label">Target size (pixels)</span><select class="input" bind:value={size} disabled={saving}>{#each [128,256,512,1024,2048] as pixels}<option value={pixels}>{pixels}</option>{/each}</select></label></div>
  {#if svg}<div class="preview card"><img src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`} alt="Generated code"/></div>{:else if !text}<Notice kind="info">Enter text to generate a QR code or barcode offline.</Notice>{/if}
  <div class="row"><button class="btn" disabled={!svg || busy || saving} onclick={()=>exportCode(false)}>Save PNG</button><button class="btn" disabled={!svg || busy || saving} onclick={()=>exportCode(true)}>Save SVG</button><button class="btn" disabled={!svg || busy || saving} onclick={copy}>Copy image</button></div>
  {#if message}<Notice kind="success">{message}</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>
<style>.tool{display:grid;gap:var(--s-4);max-width:800px}.row{display:flex;gap:var(--s-3);flex-wrap:wrap}.row .field{flex:1}.preview{padding:var(--s-4);overflow:auto}.preview img{display:block;max-width:100%;height:auto;max-height:var(--preview-height,512px)}</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { roadmapMediaApi,type MediaProvider } from '#lib/roadmap-media-api.ts';
  import Notice from './Notice.svelte';
  let {provider='ffmpeg'}:{provider?:'ffmpeg'|'tesseract'}=$props();
  let providers=$state<MediaProvider[]>([]),error=$state(''),loading=$state(true);
  const current=$derived(providers.find(item=>item.name===provider));
  onMount(()=>{let disposed=false;void roadmapMediaApi.mediaProviders().then(next=>{if(!disposed)providers=next;}).catch(cause=>{if(!disposed)error=String(cause);}).finally(()=>{if(!disposed)loading=false;});return()=>{disposed=true;};});
</script>
{#if loading}<Notice kind="info">Checking the local {provider} provider…</Notice>
{:else if error}<Notice kind="error">{error}</Notice>
{:else if current?.error}<Notice kind="error">{current.error}</Notice>
{:else}<Notice kind="info">Local provider: {current?.version ?? provider}. Oxbox uses your installed executable; it never downloads or bundles a provider.</Notice>{/if}

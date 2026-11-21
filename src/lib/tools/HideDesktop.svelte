<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { systemApi } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let hidden = $state(false); let busy = $state(true); let error = $state(''); let requestId = 0;
  onMount(async () => { const id = ++requestId; try { const value = await systemApi.desktopStatus(); if (id === requestId) hidden = value; } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; } });
  onDestroy(() => requestId++);
  async function toggle() { const id = ++requestId; busy = true; error = ''; try { const value = await systemApi.desktopSetHidden(!hidden); if (id === requestId) hidden = value; } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; } }
</script>
<div class="tool" aria-busy={busy}><p>Temporarily hide desktop icons for a presentation or screenshot. Files stay where they are.</p><Notice kind={hidden ? 'success' : 'info'}>{hidden ? 'Desktop icons are hidden by Oxbox. Quitting restores their original visibility.' : 'Oxbox has not hidden your desktop icons.'}</Notice><div><button class="btn btn-primary" onclick={toggle} disabled={busy}>{busy ? 'Updating…' : hidden ? 'Restore desktop icons' : 'Hide desktop icons'}</button></div><Notice kind="info">On Windows, this affects Explorer's desktop icon view. On macOS, Finder restarts to apply the change. Linux desktop environments are not supported.</Notice>{#if error}<Notice kind="error">{error}</Notice>{/if}</div>
<style>.tool { display: grid; gap: var(--s-4); }</style>

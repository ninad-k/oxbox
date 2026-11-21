<script lang="ts">
  import { onDestroy } from 'svelte';
  import { systemApi } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let busy = $state(false); let error = $state(''); let success = $state(false); let requestId = 0;
  onDestroy(() => requestId++);
  async function lock() { const id = ++requestId; busy = true; error = ''; success = false; try { await systemApi.lockScreen(); if (id === requestId) success = true; } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; } }
</script>
<div class="tool" aria-busy={busy}><p>Lock this session immediately. Your running applications stay open.</p><Notice kind="info">On macOS, this puts the display to sleep. Set “Require password immediately” in system settings to secure the session. Linux requires a session manager that supports loginctl.</Notice><div><button class="btn btn-primary" onclick={lock} disabled={busy}>{busy ? 'Locking…' : 'Lock screen now'}</button></div>{#if success}<Notice kind="success">The lock request was sent to your operating system.</Notice>{/if}{#if error}<Notice kind="error">{error}</Notice>{/if}</div>
<style>.tool { display: grid; gap: var(--s-4); }</style>

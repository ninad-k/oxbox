<script lang="ts">
  import { onMount } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { inDesktopApp } from '#lib/ipc.ts';
  import { systemApi, type PowerAction, type SleepInfo } from '#lib/system-api.ts';
  import Notice from '#lib/ui/Notice.svelte';
  let action = $state<PowerAction>('sleep'); let minutes = $state(25); let busy = $state(false); let loading = $state(true); let error = $state(''); let info = $state<SleepInfo>({ timer: null, error: null, remainingSeconds: 0 }); let requestId = 0;
  const remaining = $derived(info.remainingSeconds);
  onMount(() => { let stopped = false; let timer: ReturnType<typeof setTimeout>; async function poll() { const id = requestId; try { const value = await systemApi.sleepStatus(); if (!stopped && id === requestId) info = value; } catch (e) { if (!stopped && id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (!stopped) { loading = false; timer = setTimeout(poll, 1000); } } } void poll(); return () => { stopped = true; requestId++; clearTimeout(timer); }; });
  async function arm() {
    const id = ++requestId; busy = true; error = '';
    try {
      if (!inDesktopApp()) throw new Error('Open Oxbox as the desktop app to schedule a power action.');
      const seconds = Math.round(minutes * 60);
      if (!Number.isFinite(seconds) || seconds < 1) throw new Error('Choose a delay of at least one second.');
      const confirmed = await ask(`In ${minutes} minutes, Oxbox will ${action.replace('-', ' ')} this computer. Save your work first. You can cancel from the banner or tray before the countdown ends.`, { title: 'Arm Sleep Timer?', kind: 'warning', okLabel: 'Arm timer', cancelLabel: 'Keep working' });
      if (!confirmed || id !== requestId) return;
      await systemApi.sleepArm(seconds, action, true); const value = await systemApi.sleepStatus(); if (id === requestId) info = value;
    } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; }
  }
  async function cancel() { const id = ++requestId; busy = true; error = ''; try { await systemApi.sleepCancel(); if (id === requestId) info = { timer: null, error: null, remainingSeconds: 0 }; } catch (e) { if (id === requestId) error = e instanceof Error ? e.message : String(e); } finally { if (id === requestId) busy = false; } }
</script>
<div class="tool" aria-busy={busy || loading}>
  <Notice kind="info">The countdown runs while Oxbox is hidden. Quitting Oxbox cancels it. Save your work before arming shutdown, restart, or sign out.</Notice>
  <label class="field"><span class="field-label">Action</span><select class="input" bind:value={action} disabled={busy || !!info.timer}><option value="sleep">Sleep</option><option value="shutdown">Shut down</option><option value="restart">Restart</option><option value="sign-out">Sign out</option></select></label>
  <label class="field"><span class="field-label">After (minutes)</span><input class="input" type="number" min="0.02" max="44640" step="any" bind:value={minutes} disabled={busy || !!info.timer} /></label>
  {#if info.timer}<Notice kind="success">{info.timer.action.replace('-', ' ')} is scheduled in {Math.floor(remaining / 60)}m {remaining % 60}s.</Notice><div><button class="btn" onclick={cancel} disabled={busy}>Cancel timer</button></div>{:else}<div><button class="btn btn-primary" onclick={arm} disabled={busy || loading}>{busy ? 'Arming…' : 'Arm timer…'}</button></div>{/if}
  {#if error || info.error}<Notice kind="error">{error || info.error}</Notice>{/if}
</div>
<style>.tool { display: grid; gap: var(--s-4); }</style>

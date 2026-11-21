<script lang="ts">
  import { onDestroy } from 'svelte';
  import { ask, open, save } from '@tauri-apps/plugin-dialog';
  import { mediaApi } from '#lib/media-api.ts';
  import { fileApi } from '#lib/file-api.ts';
  import { createJob, type JobHandle, type JobProgress } from '#lib/jobs.ts';
  import Notice from '#lib/ui/Notice.svelte';
  import ProgressBar from '#lib/ui/ProgressBar.svelte';

  let decrypt = $state(false);
  let source = $state('');
  let output = $state('');
  let passphrase = $state('');
  let confirmation = $state('');
  let trashOriginal = $state(false);
  let busy = $state(false);
  let error = $state('');
  let success = $state('');
  let progress = $state<JobProgress | null>(null);
  let job: JobHandle | null = null;
  let requestId = 0;
  const strength = $derived(passphrase.length >= 24 ? 'Long passphrase. Prefer several randomly chosen words.' : passphrase.length >= 12 ? 'Consider a longer passphrase with unrelated words.' : 'Short passphrase. Use at least 12 characters, preferably several unrelated words.');

  async function pickSource() {
    const id = requestId;
    try {
      const chosen = await open({ multiple: false, title: decrypt ? 'Choose an age-encrypted file' : 'Choose a file to encrypt' });
      if (id === requestId && typeof chosen === 'string') { source = chosen; output = ''; success = ''; }
    } catch (cause) { if (id === requestId) error = String(cause); }
  }

  async function pickOutput() {
    const id = requestId;
    try {
      const defaultPath = decrypt ? source.replace(/\.age$/i, '') + '.decrypted' : source + '.age';
      const chosen = await save({ title: 'Choose a new output filename', defaultPath });
      if (id === requestId && chosen) output = chosen;
    } catch (cause) { if (id === requestId) error = String(cause); }
  }

  async function run() {
    if (!decrypt && passphrase !== confirmation) { error = 'The passphrases do not match.'; return; }
    const id = ++requestId;
    busy = true; error = ''; success = ''; progress = null;
    job = createJob('encrypt-files', (next) => { if (id === requestId) progress = next; });
    const secret = passphrase;
    passphrase = ''; confirmation = '';
    try {
      await job.run((jobId) => mediaApi.cryptFile(jobId, source, output, secret, decrypt));
      if (id !== requestId) return;
      success = `${decrypt ? 'Decrypted' : 'Encrypted'} file saved to ${output}. Original file kept.`;
      if (trashOriginal && await ask(`The output was saved successfully. Move the original to the Recycle Bin/Trash?\n\n${source}`, { title: 'Move original to Trash?', kind: 'warning' })) {
        if (id !== requestId) return;
        await fileApi.trashFiles([source], true);
        if (id === requestId) success = `Output saved to ${output}. Original moved to the Recycle Bin/Trash.`;
      }
    }
    catch (cause) { if (id === requestId) error = String(cause); }
    finally { if (id === requestId) { busy = false; job = null; } }
  }

  async function cancel() { try { await job?.cancel(); } catch (cause) { error = String(cause); } }
  onDestroy(() => { requestId++; passphrase = ''; confirmation = ''; void job?.cancel().catch(() => {}); });
</script>

<div class="tool" aria-busy={busy}>
  <Notice kind="info">Uses the age format with scrypt passphrase encryption. Originals are kept and existing outputs are never overwritten. Passphrases are sent only to the local Rust process and are never logged or stored. Losing your passphrase means losing access to the encrypted file.</Notice>
  <label class="field"><span class="field-label">Operation</span><select class="input" bind:value={decrypt} disabled={busy} onchange={() => { source = ''; output = ''; success = ''; }}><option value={false}>Encrypt</option><option value={true}>Decrypt</option></select></label>
  <div class="row"><button class="btn" onclick={pickSource} disabled={busy}>Choose source file</button><span class="path">{source || 'No source selected'}</span></div>
  <div class="row"><button class="btn" onclick={pickOutput} disabled={busy || !source}>Choose output filename</button><span class="path">{output || 'Choose a new output filename'}</span></div>
  <label class="field"><span class="field-label">Passphrase</span><input class="input" type="password" autocomplete="off" maxlength="4096" bind:value={passphrase} disabled={busy} /></label>
  {#if !decrypt}<label class="field"><span class="field-label">Confirm passphrase</span><input class="input" type="password" autocomplete="off" maxlength="4096" bind:value={confirmation} disabled={busy} /></label><p class="hint">{strength}</p>{/if}
  <label><input type="checkbox" bind:checked={trashOriginal} disabled={busy} /> Offer to move the original to Trash after success (confirmation required)</label>
  <div class="row"><button class="btn btn-primary" onclick={run} disabled={busy || !source || !output || !passphrase || (!decrypt && passphrase !== confirmation)}>{busy ? 'Processing…' : decrypt ? 'Decrypt file' : 'Encrypt file'}</button>{#if busy}<button class="btn" onclick={cancel}>Cancel</button>{/if}</div>
  {#if busy && progress}<ProgressBar {progress} />{/if}
  {#if success}<Notice kind="success">{success}</Notice>{/if}
  {#if error}<Notice kind="error">{error}</Notice>{/if}
</div>

<style>
  .tool { display: grid; gap: var(--s-4); }
  .row { display: flex; gap: var(--s-3); align-items: center; flex-wrap: wrap; }
  .path { overflow-wrap: anywhere; color: var(--text-2); }
  .hint { font-size: var(--text-sm); color: var(--text-3); }
</style>

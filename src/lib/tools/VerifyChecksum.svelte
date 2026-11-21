<!--
  Tool view: Verify Checksum (`verify-checksum`).
  Reference for a file tool: native file picker → long-running Rust command → result + comparison.
-->
<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import FileCheck from '@lucide/svelte/icons/file-check';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import { api, inDesktopApp, ToolError, type HashAlgorithm } from '$lib/api';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import Notice from '$lib/ui/Notice.svelte';
  import Segmented from '$lib/ui/Segmented.svelte';
  import { digestsMatch, fileName } from './checksum';

  const algorithms: { value: HashAlgorithm; label: string }[] = [
    { value: 'sha256', label: 'SHA-256' },
    { value: 'sha1', label: 'SHA-1' },
    { value: 'md5', label: 'MD5' },
    { value: 'blake3', label: 'BLAKE3' },
  ];

  let path = $state<string | null>(null);
  let algorithm = $state<HashAlgorithm>('sha256');
  let digest = $state('');
  let expected = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let requestId = 0;

  const comparison = $derived(digest && expected.trim() ? digestsMatch(digest, expected) : null);

  async function pickFile() {
    error = null;
    if (!inDesktopApp()) {
      error = 'Open Oxbox as the desktop app to choose a file.';
      return;
    }
    const selected = await open({ multiple: false, directory: false, title: 'Choose a file to hash' });
    if (typeof selected === 'string') path = selected;
  }

  // Recompute whenever the file or algorithm changes.
  $effect(() => {
    const p = path;
    const algo = algorithm;
    if (!p) return;
    const id = ++requestId;
    busy = true;
    digest = '';
    error = null;
    api
      .checksumFile(p, algo)
      .then((d) => {
        if (id === requestId) digest = d;
      })
      .catch((e: unknown) => {
        if (id === requestId) error = e instanceof ToolError ? e.message : String(e);
      })
      .finally(() => {
        if (id === requestId) busy = false;
      });
  });
</script>

<div class="tool">
  <div class="card file">
    <span class="file-icon" aria-hidden="true"><FileCheck size={22} /></span>
    <div class="file-text">
      {#if path}
        <strong>{fileName(path)}</strong>
        <span class="path" title={path}>{path}</span>
      {:else}
        <strong>No file selected</strong>
        <span class="path">Choose any file to calculate its checksum.</span>
      {/if}
    </div>
    <button class="btn btn-primary" onclick={pickFile} disabled={busy}>
      <FolderOpen size={16} aria-hidden="true" />
      {path ? 'Choose another' : 'Choose file'}
    </button>
  </div>

  <div class="field">
    <span class="field-label">Algorithm</span>
    <Segmented label="Hash algorithm" options={algorithms} bind:value={algorithm} />
  </div>

  <div class="field">
    <div class="row">
      <span class="field-label" id="digest-label">Checksum</span>
      <CopyButton text={digest} />
    </div>
    <output class="digest mono" aria-labelledby="digest-label" aria-busy={busy}>
      {#if busy}
        <LoaderCircle size={16} class="spin" aria-hidden="true" /> Calculating…
      {:else if digest}
        {digest}
      {:else}
        <span class="placeholder">The checksum appears here.</span>
      {/if}
    </output>
  </div>

  <label class="field">
    <span class="field-label">Compare with (optional)</span>
    <input class="input mono" bind:value={expected} placeholder="Paste the checksum published by the download site" />
  </label>

  {#if comparison === true}
    <Notice kind="success">Match. The file's checksum is identical to the one you pasted.</Notice>
  {:else if comparison === false}
    <Notice kind="error">No match. The file may be corrupted, tampered with, or hashed with another algorithm.</Notice>
  {/if}

  {#if error}
    <Notice kind="error">{error}</Notice>
  {/if}
</div>

<style>
  .tool {
    display: grid;
    gap: var(--s-5);
  }
  .file {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-4);
  }
  .file-icon {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    border-radius: var(--r-md);
    background: var(--surface-sunken);
    color: var(--text-2);
  }
  .file-text {
    display: grid;
    flex: 1;
    min-width: 0;
  }
  .path {
    font-size: var(--text-sm);
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .digest {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 44px;
    padding: var(--s-3);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-sunken);
    word-break: break-all;
  }
  .placeholder {
    font-family: var(--font-ui);
    color: var(--text-3);
  }
  .digest :global(.spin) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>

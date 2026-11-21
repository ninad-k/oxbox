<script lang="ts">
  import { onDestroy } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { mediaApi, type ResizeMode, type ResizeRequest } from '#lib/media-api.ts';
  import { createJob, type JobHandle, type JobProgress } from '#lib/jobs.ts';
  import Notice from '#lib/ui/Notice.svelte';
  import ProgressBar from '#lib/ui/ProgressBar.svelte';

  let paths = $state<string[]>([]);
  let outputDir = $state('');
  let mode = $state<'percent' | 'fit' | 'exact'>('fit');
  let percent = $state(50);
  let width = $state(1920);
  let height = $state(1080);
  let format = $state<ResizeRequest['format']>('png');
  let jpegQuality = $state(85);
  let busy = $state(false);
  let error = $state('');
  let outputs = $state<string[]>([]);
  let progress = $state<JobProgress | null>(null);
  let job: JobHandle | null = null;
  let requestId = 0;

  async function pickImages() {
    const id = requestId;
    try { const chosen = await open({ multiple: true, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] }); if (id === requestId && chosen) paths = Array.isArray(chosen) ? chosen : [chosen]; }
    catch (cause) { if (id === requestId) error = String(cause); }
  }

  async function pickOutput() {
    const id = requestId;
    try { const chosen = await open({ directory: true, title: 'Choose resized image output folder' }); if (id === requestId && typeof chosen === 'string') outputDir = chosen; }
    catch (cause) { if (id === requestId) error = String(cause); }
  }

  async function run() {
    const id = ++requestId;
    busy = true; error = ''; outputs = []; progress = null;
    job = createJob('resize-images', (next) => { if (id === requestId) progress = next; });
    const selectedMode: ResizeMode = mode === 'percent' ? { kind: mode, percent } : { kind: mode, width, height };
    try { const next = await job.run((jobId) => mediaApi.resizeImages(jobId, { paths: [...paths], outputDir, mode: selectedMode, format, jpegQuality })); if (id === requestId) outputs = next; }
    catch (cause) { if (id === requestId) error = String(cause); }
    finally { if (id === requestId) { busy = false; job = null; } }
  }

  async function cancel() { try { await job?.cancel(); } catch (cause) { error = String(cause); } }
  onDestroy(() => { requestId++; void job?.cancel().catch(() => {}); });
</script>

<div class="tool" aria-busy={busy}>
  <Notice kind="info">Originals are kept. Outputs use “-resized” filenames and existing files are never overwritten. JPEG/PNG/WebP orientation is applied before resizing. Images are limited to 40 million pixels.</Notice>
  <div class="row"><button class="btn" onclick={pickImages} disabled={busy}>Choose images</button><span>{paths.length ? `${paths.length} images selected` : 'Choose PNG, JPEG or WebP images.'}</span></div>
  {#if paths.length}<p class="paths">{paths.join('\n')}</p>{/if}
  <div class="row"><button class="btn" onclick={pickOutput} disabled={busy}>Choose output folder</button><span class="paths">{outputDir || 'No output folder selected'}</span></div>
  <label class="field"><span class="field-label">Resize mode</span><select class="input" bind:value={mode} disabled={busy}><option value="fit">Fit within dimensions (keep aspect ratio)</option><option value="percent">Percentage (keep aspect ratio)</option><option value="exact">Exact dimensions (can stretch)</option></select></label>
  {#if mode === 'percent'}<label class="field"><span class="field-label">Scale (%)</span><input class="input" type="number" min="0.01" max="10000" step="0.01" bind:value={percent} disabled={busy} /></label>
  {:else}<div class="row"><label class="field"><span class="field-label">Width (px)</span><input class="input" type="number" min="1" max="32768" bind:value={width} disabled={busy} /></label><label class="field"><span class="field-label">Height (px)</span><input class="input" type="number" min="1" max="32768" bind:value={height} disabled={busy} /></label></div>{/if}
  <label class="field"><span class="field-label">Output format</span><select class="input" bind:value={format} disabled={busy}><option value="png">PNG</option><option value="jpeg">JPEG</option><option value="webp">WebP (lossless)</option></select></label>
  {#if format === 'jpeg'}<label class="field"><span class="field-label">JPEG quality (1–100)</span><input class="input" type="number" min="1" max="100" bind:value={jpegQuality} disabled={busy} /></label>{/if}
  <div class="row"><button class="btn btn-primary" onclick={run} disabled={busy || !paths.length || !outputDir}>{busy ? 'Resizing…' : 'Resize images'}</button>{#if busy}<button class="btn" onclick={cancel}>Cancel</button>{/if}</div>
  {#if busy && progress}<ProgressBar {progress} />{/if}
  {#if outputs.length}<Notice kind="success">Saved {outputs.length} images in {outputDir}.</Notice>{/if}
  {#if error}<Notice kind="error">{error} Any completed outputs are kept; an incomplete output is removed.</Notice>{/if}
</div>

<style>
  .tool { display: grid; gap: var(--s-4); }
  .row { display: flex; gap: var(--s-3); align-items: center; flex-wrap: wrap; }
  .paths { white-space: pre-wrap; overflow-wrap: anywhere; color: var(--text-2); max-height: 10em; overflow-y: auto; }
</style>

<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Star from '@lucide/svelte/icons/star';
  import { categoryById, type Os, type ToolDef } from '#lib/registry.ts';
  import { favorites } from '#lib/favorites.svelte.ts';
  import Notice from './Notice.svelte';
  import PlannedTool from './PlannedTool.svelte';

  interface Props {
    tool: ToolDef;
    os: Os | 'other';
    onback: () => void;
  }
  let { tool, os, onback }: Props = $props();

  const category = $derived(categoryById.get(tool.category));
  const isFavorite = $derived(favorites.has(tool.id));
  const supported = $derived(os === 'other' || tool.platforms.includes(os));
  const osName = { windows: 'Windows', macos: 'macOS', linux: 'Linux', other: 'this platform' };

  let heading: HTMLHeadingElement | undefined = $state();
  $effect(() => {
    // Move focus to the tool title when a tool opens (screen readers announce it).
    void tool.id;
    heading?.focus();
  });
</script>

<section class="view" aria-labelledby="tool-title">
  <header class="header">
    <button class="btn btn-ghost btn-icon" onclick={onback} aria-label="Back to tools">
      <ArrowLeft size={18} />
    </button>
    <span class="chip" aria-hidden="true">
      <tool.icon size={20} strokeWidth={1.8} />
    </span>
    <div class="titles">
      <p class="crumb">{category?.label}</p>
      <h1 id="tool-title" tabindex="-1" bind:this={heading}>{tool.name}</h1>
    </div>
    <button
      class="btn btn-ghost btn-icon fav"
      aria-pressed={isFavorite}
      aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
      title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
      onclick={() => favorites.toggle(tool.id)}
    >
      <Star size={18} fill={isFavorite ? 'currentColor' : 'none'} />
    </button>
  </header>

  <div class="content">
    <p class="lede">{tool.description}.</p>

    {#if !supported}
      <Notice kind="info">This tool is not available on {osName[os]} yet.</Notice>
    {:else if tool.status === 'planned' || !tool.load}
      <PlannedTool {tool} />
    {:else}
      {#await tool.load()}
        <div class="loading" aria-busy="true" aria-label="Loading tool"></div>
      {:then mod}
        <mod.default />
      {:catch err}
        <Notice kind="error">Could not load this tool: {String(err)}</Notice>
      {/await}
    {/if}
  </div>
</section>

<style>
  .view {
    display: flex;
    flex-direction: column;
    min-height: 100%;
  }
  .header {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-6);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .chip {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--r-md);
    color: var(--text-2);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .crumb {
    font-size: var(--text-xs);
    color: var(--text-3);
  }
  h1 {
    font-size: var(--text-xl);
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  h1:focus {
    outline: none;
  }
  .fav[aria-pressed='true'] {
    color: var(--accent);
  }
  .content {
    display: grid;
    gap: var(--s-5);
    align-content: start;
    width: 100%;
    max-width: 880px;
    padding: var(--s-6);
  }
  .lede {
    color: var(--text-2);
  }
  .loading {
    height: 180px;
    border-radius: var(--r-lg);
    background: var(--surface-sunken);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.5;
    }
  }
</style>

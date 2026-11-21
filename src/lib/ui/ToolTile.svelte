<script lang="ts">
  import Star from '@lucide/svelte/icons/star';
  import { categoryById, type ToolDef } from '$lib/registry';

  interface Props {
    tool: ToolDef;
    favorite: boolean;
    onopen: (id: string) => void;
  }
  let { tool, favorite, onopen }: Props = $props();
  const tone = $derived(categoryById.get(tool.category)?.tone ?? 'var(--text-2)');
</script>

<button class="tile" onclick={() => onopen(tool.id)} data-tool={tool.id}>
  <span class="chip" style:--tone={tone} aria-hidden="true">
    <tool.icon size={20} strokeWidth={1.8} />
  </span>
  <span class="body">
    <span class="name">
      {tool.name}
      {#if favorite}<Star size={12} class="fav" aria-label="Favorite" />{/if}
    </span>
    <span class="desc">{tool.description}</span>
  </span>
  {#if tool.status === 'planned'}
    <span class="badge">Planned</span>
  {/if}
</button>

<style>
  .tile {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: var(--s-3);
    width: 100%;
    min-height: 76px;
    padding: var(--s-3) var(--s-4);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      background 120ms ease;
  }
  .tile:hover {
    border-color: var(--border-strong);
    background: var(--surface-hover);
  }
  .chip {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: var(--r-md);
    color: var(--tone);
    background: color-mix(in srgb, var(--tone) 13%, transparent);
  }
  .body {
    display: grid;
    gap: 2px;
    min-width: 0;
    padding-right: var(--s-7);
  }
  .name {
    display: flex;
    align-items: center;
    gap: var(--s-1);
    font-weight: 600;
    color: var(--text);
  }
  .name :global(.fav) {
    color: var(--accent);
    fill: currentColor;
  }
  .desc {
    font-size: var(--text-sm);
    color: var(--text-2);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .badge {
    position: absolute;
    top: var(--s-3);
    right: var(--s-3);
    padding: 1px var(--s-2);
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-sunken);
    font-size: var(--text-xs);
    color: var(--text-3);
  }
</style>

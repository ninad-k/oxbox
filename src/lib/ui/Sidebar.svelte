<script lang="ts">
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import Star from '@lucide/svelte/icons/star';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import { categories } from '#lib/registry.ts';
  import type { View } from '#lib/search.ts';

  interface Props {
    view: View;
    counts: Record<string, number>;
    version: string;
    onselect: (view: View) => void;
    onsettings: () => void;
  }
  let { view, counts, version, onselect, onsettings }: Props = $props();
</script>

<nav class="sidebar" aria-label="Tool categories">
  <div class="brand">
    <img src="/oxbox.svg" alt="" width="28" height="28" />
    <span class="brand-name">Oxbox</span>
  </div>

  <ul class="nav">
    <li>
      <button class="item" aria-current={view === 'all' ? 'page' : undefined} onclick={() => onselect('all')}>
        <LayoutGrid size={17} aria-hidden="true" />
        <span class="label">All tools</span>
        <span class="count">{counts.all}</span>
      </button>
    </li>
    <li>
      <button
        class="item"
        aria-current={view === 'favorites' ? 'page' : undefined}
        onclick={() => onselect('favorites')}
      >
        <Star size={17} aria-hidden="true" />
        <span class="label">Favorites</span>
        <span class="count">{counts.favorites}</span>
      </button>
    </li>
  </ul>

  <p class="section" id="categories-heading">Categories</p>
  <ul class="nav" aria-labelledby="categories-heading">
    {#each categories as cat (cat.id)}
      <li>
        <button
          class="item"
          style:--tone={cat.tone}
          title={cat.label}
          aria-current={view === cat.id ? 'page' : undefined}
          onclick={() => onselect(cat.id)}
        >
          <cat.icon size={17} aria-hidden="true" class="cat-icon" />
          <span class="label">{cat.label}</span>
          <span class="count">{counts[cat.id] ?? 0}</span>
        </button>
      </li>
    {/each}
  </ul>

  <div class="footer">
    <button class="btn btn-icon" aria-label="Settings" onclick={onsettings}><SettingsIcon size={17}/></button>
    <span>v{version}</span>
    <span aria-hidden="true">·</span>
    <span>Open source</span>
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
    width: 232px;
    flex-shrink: 0;
    padding: var(--s-4) var(--s-3);
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 0 var(--s-2) var(--s-4);
  }
  .brand-name {
    font-size: var(--text-lg);
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .nav {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .section {
    margin: var(--s-4) var(--s-2) var(--s-1);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 100%;
    height: 34px;
    padding: 0 var(--s-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text-2);
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .item[aria-current='page'] {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    box-shadow: var(--shadow-sm), inset 0 0 0 1px var(--border);
  }
  .item :global(.cat-icon) {
    color: var(--tone);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: var(--text-xs);
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .footer {
    margin-top: auto;
    display: flex;
    gap: var(--s-2);
    padding: var(--s-4) var(--s-2) 0;
    font-size: var(--text-xs);
    color: var(--text-3);
  }

  /* Narrow windows: icon-only rail. */
  @media (max-width: 760px) {
    .sidebar {
      width: 60px;
      padding: var(--s-3) var(--s-2);
      align-items: center;
    }
    .brand {
      padding: 0 0 var(--s-3);
    }
    .brand-name,
    .count,
    .section {
      display: none;
    }
    .footer { padding: var(--s-3) 0 0; }
    .footer > span { display: none; }
    /* Visually hidden but still the button's accessible name. */
    .label {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
      white-space: nowrap;
    }
    .item {
      justify-content: center;
      width: 40px;
      padding: 0;
    }
  }
</style>

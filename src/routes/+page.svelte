<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Search from '@lucide/svelte/icons/search';
  import SearchX from '@lucide/svelte/icons/search-x';
  import Star from '@lucide/svelte/icons/star';
  import X from '@lucide/svelte/icons/x';
  import { api, guessOs, inDesktopApp } from '#lib/api.ts';
  import { favorites } from '#lib/favorites.svelte.ts';
  import { categories, categoryById, toolById, tools } from '#lib/registry.ts';
  import { filterTools, type View } from '#lib/search.ts';
  import Sidebar from '#lib/ui/Sidebar.svelte';
  import ToolTile from '#lib/ui/ToolTile.svelte';
  import ToolView from '#lib/ui/ToolView.svelte';
  import Settings from '#lib/ui/Settings.svelte';
  import SleepTimerBanner from '#lib/ui/SleepTimerBanner.svelte';
  import RecordingBanner from '#lib/ui/RecordingBanner.svelte';
  import Notice from '#lib/ui/Notice.svelte';
  import { settings } from '#lib/settings.svelte.ts';
  import { clocks } from '#lib/clocks.svelte.ts';
  import { listen } from '@tauri-apps/api/event';

  let view = $state<View>('all');
  let showSettings = $state(false);
  let systemError = $state('');
  let query = $state('');
  let activeId = $state<string | null>(null);
  let os = $state(guessOs());
  let version = $state('0.1.0');
  let searchInput: HTMLInputElement | undefined = $state();

  const active = $derived(activeId ? toolById.get(activeId) : undefined);
  const searching = $derived(query.trim().length > 0);
  const results = $derived(
    filterTools(tools, {
      view,
      query,
      favorites: favorites.ids,
      categoryLabel: (id) => categoryById.get(id)?.label ?? id,
    }),
  );
  const counts = $derived({
    all: tools.length,
    favorites: favorites.ids.filter((id) => toolById.has(id)).length,
    ...Object.fromEntries(categories.map((c) => [c.id, tools.filter((t) => t.category === c.id).length])),
  });
  const readyCount = tools.filter((t) => t.status === 'ready').length;
  const title = $derived(
    searching
      ? 'Search results'
      : view === 'all'
        ? 'All tools'
        : view === 'favorites'
          ? 'Favorites'
          : (categoryById.get(view)?.label ?? ''),
  );
  const shortcut = $derived(os === 'macos' ? '⌘K' : 'Ctrl K');

  onMount(async () => {
    if (!inDesktopApp()) return;
    try {
      const info = await api.appInfo();
      os = info.os;
      version = info.version;
    } catch {
      // Keep the guesses; the launcher still works.
    }
  });

  onMount(() => {
    void settings.init();
    const stopClocks = clocks.startTicks();
    let disposed = false;
    const stops: (() => void)[] = [];
    if (inDesktopApp()) {
      void listen<{message:string}>('tool://system/error', event => {systemError=event.payload.message;}).then(stop=>{if(disposed)stop();else stops.push(stop);});
      for (const [event, handler] of [['oxbox://settings', () => {showSettings=true;}], ['oxbox://focus-search', () => {showSettings=false; void focusSearch();}]] as const) {
        void listen(event, handler).then(stop => {if(disposed)stop();else stops.push(stop);});
      }
    }
    return () => { disposed=true; stopClocks?.(); stops.forEach(stop=>stop()); };
  });

  function select(next: View) {
    showSettings = false;
    view = next;
    query = '';
    activeId = null;
  }

  async function closeTool() {
    const id = activeId;
    activeId = null;
    await tick();
    // Return focus to the tile the user came from.
    document.querySelector<HTMLElement>(`[data-tool="${id}"]`)?.focus();
  }

  async function focusSearch() {
    showSettings = false;
    activeId = null;
    await tick();
    searchInput?.focus();
    searchInput?.select();
  }

  function isTyping(target: EventTarget | null) {
    return target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));
  }

  function onWindowKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      focusSearch();
    } else if (e.key === '/' && !isTyping(e.target)) {
      e.preventDefault();
      focusSearch();
    } else if (e.key === 'Escape') {
      if (activeId) closeTool();
      else if (query) query = '';
    }
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && results.length > 0) activeId = results[0].id;
  }
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="app">
  <Sidebar view={searching ? 'all' : view} {counts} {version} onselect={select} onsettings={() => {showSettings=true;activeId=null;}} />

  <main class="main">
    <SleepTimerBanner />
    <RecordingBanner />
    {#if systemError}<Notice kind="error">{systemError}<button class="btn" onclick={()=>{systemError='';}}>Dismiss</button></Notice>{/if}
    {#if showSettings}
      <Settings onback={() => {showSettings=false;}} />
    {:else if active}
      <ToolView tool={active} {os} onback={closeTool} />
    {:else}
      <header class="topbar">
        <div class="search">
          <Search size={16} aria-hidden="true" />
          <input
            bind:this={searchInput}
            bind:value={query}
            type="search"
            placeholder="Search tools"
            aria-label="Search tools"
            autocomplete="off"
            spellcheck="false"
            onkeydown={onSearchKey}
          />
          {#if query}
            <button class="clear" aria-label="Clear search" onclick={() => (query = '')}><X size={14} /></button>
          {:else}
            <kbd aria-hidden="true">{shortcut}</kbd>
          {/if}
        </div>
      </header>

      <div class="browse">
        <div class="heading">
          <h1>{title}</h1>
          <p class="sub" aria-live="polite">
            {results.length}
            {results.length === 1 ? 'tool' : 'tools'}{#if view === 'all' && !searching}&nbsp;· {readyCount} ready, more
              arriving each release{/if}
          </p>
        </div>

        {#if results.length === 0}
          <div class="empty" role="status">
            {#if searching}
              <SearchX size={28} aria-hidden="true" />
              <h2>No tools match “{query.trim()}”</h2>
              <p>Try another word, such as “hash”, “zip” or “screenshot”.</p>
            {:else}
              <Star size={28} aria-hidden="true" />
              <h2>No favorites yet</h2>
              <p>Open a tool and select the star to pin it here.</p>
            {/if}
          </div>
        {:else if view === 'all' && !searching}
          {#each categories as cat (cat.id)}
            {@const inCat = results.filter((t) => t.category === cat.id)}
            <section class="group" aria-labelledby="group-{cat.id}">
              <h2 id="group-{cat.id}">{cat.label}</h2>
              <div class="grid">
                {#each inCat as tool (tool.id)}
                  <ToolTile {tool} favorite={favorites.has(tool.id)} onopen={(id) => (activeId = id)} />
                {/each}
              </div>
            </section>
          {/each}
        {:else}
          <div class="grid">
            {#each results as tool (tool.id)}
              <ToolTile {tool} favorite={favorites.has(tool.id)} onopen={(id) => (activeId = id)} />
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
  }
  .topbar {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: var(--s-4) var(--s-6);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    max-width: 560px;
    height: 36px;
    padding: 0 var(--s-2) 0 var(--s-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--text-3);
  }
  .search:focus-within {
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    background: transparent;
    color: var(--text);
    outline: none;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  kbd {
    padding: 1px var(--s-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface-sunken);
    font-family: var(--font-ui);
    font-size: var(--text-xs);
    color: var(--text-3);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
    cursor: pointer;
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .browse {
    display: grid;
    gap: var(--s-6);
    padding: var(--s-6);
  }
  .heading h1 {
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .sub {
    margin-top: 2px;
    font-size: var(--text-sm);
    color: var(--text-3);
  }
  .group {
    display: grid;
    gap: var(--s-3);
  }
  .group h2 {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text-2);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--s-3);
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: var(--s-2);
    padding: var(--s-8) var(--s-4);
    color: var(--text-3);
    text-align: center;
  }
  .empty h2 {
    font-size: var(--text-lg);
    color: var(--text);
  }
</style>

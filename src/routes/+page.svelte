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
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import Keyboard from '@lucide/svelte/icons/keyboard';
  import { defaultView, favoriteShortcut } from '#lib/launcher.ts';
  import MouseActivityBanner from '#lib/ui/MouseActivityBanner.svelte';

  let view = $state<View>(defaultView);
  let showSettings = $state(false);
  let systemError = $state('');
  let query = $state('');
  let activeId = $state<string | null>(null);
  let os = $state(guessOs());
  let version = $state('0.1.0');
  let launcherHotkey = $state('Ctrl + Alt + O');
  let shortcutError = $state('');
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
  const favoriteIds = $derived(favorites.ids.filter(id=>toolById.has(id)));
  const title = $derived(
    searching
      ? 'Search results'
      : view === 'all'
        ? 'Tool library'
        : view === 'favorites'
          ? 'Dashboard'
          : (categoryById.get(view)?.label ?? ''),
  );
  const shortcut = $derived(os === 'macos' ? '⌘K' : 'Ctrl K');

  onMount(async () => {
    if (!inDesktopApp()) return;
    try {
      const info = await api.appInfo();
      os = info.os;
      version = info.version;
      const shortcutInfo=await api.shortcutStatus();
      launcherHotkey=shortcutInfo.shortcut.replace('CommandOrControl',info.os==='macos'?'⌘':'Ctrl').replaceAll('+',' + ');
      shortcutError=shortcutInfo.error ?? '';
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
    const favorite = favoriteShortcut(e.code,{alt:e.altKey,ctrl:e.ctrlKey,meta:e.metaKey,shift:e.shiftKey},favoriteIds);
    if (favorite && (!isTyping(e.target) || (e.target===searchInput && !query))) {
      e.preventDefault(); showSettings=false; activeId=favorite;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'q' && inDesktopApp()) {
      e.preventDefault(); void quit();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
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
  async function quit() {try {await api.quitApp();}catch(error){systemError=String(error);}}
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="app">
  <Sidebar view={searching ? 'all' : view} {counts} {version} settingsOpen={showSettings} desktop={inDesktopApp()} onselect={select} onsettings={() => {showSettings=true;activeId=null;}} onquit={()=>{void quit();}} />

  <main class="main">
    <SleepTimerBanner />
    <RecordingBanner />
    <MouseActivityBanner />
    {#if shortcutError}<Notice kind="error">Launcher shortcut unavailable. Choose another in Settings.<button class="btn" onclick={()=>{showSettings=true;activeId=null;}}>Settings</button></Notice>{/if}
    {#if settings.error}<Notice kind="error">{settings.error}</Notice>{/if}
    {#if systemError}<Notice kind="error">{systemError}<button class="btn" onclick={()=>{systemError='';}}>Dismiss</button></Notice>{/if}
    {#if showSettings}
      <Settings onback={() => {showSettings=false;}} onshortcut={value => {launcherHotkey=value.replace('CommandOrControl',os==='macos'?'⌘':'Ctrl').replaceAll('+',' + ');shortcutError='';}} />
    {:else if active}
      <ToolView tool={active} {os} onback={closeTool} />
    {:else}
      <header class="topbar">
        <div class="search">
          <Search size={18} aria-hidden="true" />
          <input
            bind:this={searchInput}
            bind:value={query}
            type="search"
            placeholder="Find a tool…"
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
          <div><p class="eyebrow">{view==='favorites'&&!searching?'YOUR WORKSPACE':'EXPLORE OXBOX'}</p><h1>{title}</h1>
          <p class="sub" aria-live="polite">{view==='favorites'&&!searching?'Just the tools you choose. Always within reach.':results.length+' tools for the everyday details.'}</p></div>
          {#if view==='favorites'&&!searching}<button class="btn" onclick={()=>select('all')}>Manage favorites</button>{/if}
        </div>

        {#if results.length === 0}
          <div class="empty" role="status">
            {#if searching}
              <SearchX size={28} aria-hidden="true" />
              <h2>No tools match “{query.trim()}”</h2>
              <p>Try another word, such as “hash”, “zip” or “screenshot”.</p>
            {:else}
              <span class="empty-mark"><LayoutGrid size={30} strokeWidth={1.5} aria-hidden="true" /></span>
              <h2>Make room for your essentials</h2>
              <p>Star a tool in the library to add it to your dashboard.<br/>Everything else stays neatly out of the way.</p>
              <button class="btn btn-primary" onclick={()=>select('all')}>Explore the tools</button>
            {/if}
          </div>
        {:else if view === 'all' && !searching}
          {#each categories as cat (cat.id)}
            {@const inCat = results.filter((t) => t.category === cat.id)}
            <section class="group" aria-labelledby="group-{cat.id}">
              <h2 id="group-{cat.id}">{cat.label}</h2>
              <div class="grid">
                {#each inCat as tool (tool.id)}
                  <ToolTile {tool} favorite={favorites.has(tool.id)} onfavorite={id=>favorites.toggle(id)} onopen={(id) => (activeId = id)} />
                {/each}
              </div>
            </section>
          {/each}
        {:else}
          <div class="grid">
            {#each results as tool, index (tool.id)}
              <ToolTile {tool} slot={view==='favorites'&&!searching?index+1:undefined} favorite={favorites.has(tool.id)} onfavorite={id=>favorites.toggle(id)} onopen={(id) => (activeId = id)} />
            {/each}
          </div>
        {/if}
        {#if view==='favorites'&&!searching}<div class="keyboard-note"><Keyboard size={16}/><span><kbd>{shortcut}</kbd> search · <kbd>Alt 1–9</kbd> favorites</span></div><button class="launcher-key" onclick={()=>{showSettings=true;activeId=null;}}><span>Open Oxbox from anywhere</span><kbd>{launcherHotkey}</kbd></button>{/if}
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
    font-size: var(--text-2xl);
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .sub {
    margin-top: var(--s-2);
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
    grid-template-columns: repeat(auto-fill, minmax(min(220px,100%), 1fr));
    gap: var(--s-3);
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: var(--s-4);
    padding: var(--s-8) var(--s-4);
    min-height:360px;
    align-content:center;
    border:1px dashed var(--border-strong);
    border-radius:var(--r-lg);
    color: var(--text-3);
    text-align: center;
  }
  .empty h2 {
    font-size: var(--text-lg);
    color: var(--text);
  }
  .heading{display:flex;align-items:center;justify-content:space-between;gap:var(--s-4)}
  .eyebrow{font-size:var(--text-xs);font-weight:650;letter-spacing:.08em;color:var(--text-3);margin-bottom:var(--s-2)}
  .empty-mark{display:grid;place-items:center;width:64px;height:64px;background:var(--accent-soft);color:var(--accent-text);border-radius:var(--r-lg)}
  .keyboard-note{display:flex;align-items:center;justify-content:center;gap:var(--s-2);color:var(--text-3);font-size:var(--text-xs);padding:var(--s-3)}
  .launcher-key{display:flex;flex-wrap:wrap;align-items:center;justify-content:center;gap:var(--s-3);padding:var(--s-3);border:0;background:transparent;color:var(--text-3);font-size:var(--text-xs);cursor:pointer}.launcher-key:hover{color:var(--text)}
  @media(max-width:820px){.heading{align-items:flex-start;flex-wrap:wrap}.heading .btn{font-size:var(--text-xs)}.browse{padding:var(--s-5)}.topbar{padding:var(--s-4) var(--s-5)}}
</style>

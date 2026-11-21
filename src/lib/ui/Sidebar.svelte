<script lang="ts">
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import Grid2x2 from '@lucide/svelte/icons/grid-2x2';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import Power from '@lucide/svelte/icons/power';
  import { categories } from '#lib/registry.ts';
  import type { View } from '#lib/search.ts';
  interface Props {view:View;counts:Record<string,number>;version:string;desktop:boolean;settingsOpen:boolean;onselect:(view:View)=>void;onsettings:()=>void;onquit:()=>void}
  let {view,counts,version,desktop,settingsOpen,onselect,onsettings,onquit}:Props=$props();
</script>
<nav class="sidebar" aria-label="Tool navigation">
  <div class="brand"><img src="/oxbox.svg" alt="" width="34" height="34"/><div><span class="brand-name">Oxbox</span><span class="brand-note">Your everyday toolkit</span></div></div>
  <div class="nav">
    <button class="item" aria-current={!settingsOpen&&view==='favorites'?'page':undefined} onclick={()=>onselect('favorites')}><LayoutGrid size={18}/><span class="label">Dashboard</span><span class="count">{counts.favorites}</span></button>
    <button class="item" aria-current={!settingsOpen&&view==='all'?'page':undefined} onclick={()=>onselect('all')}><Grid2x2 size={18}/><span class="label">All tools</span><span class="count">{counts.all}</span></button>
  </div>
  <p class="section">Explore</p>
  <div class="nav">
    {#each categories as cat (cat.id)}
      <button class="item" title={cat.label} aria-current={!settingsOpen&&view===cat.id?'page':undefined} onclick={()=>onselect(cat.id)}><cat.icon size={18} strokeWidth={1.7}/><span class="label">{cat.label}</span></button>
    {/each}
  </div>
  <div class="footer">
    <button class="item" aria-current={settingsOpen?'page':undefined} onclick={onsettings}><SettingsIcon size={18}/><span class="label">Settings</span></button>
    {#if desktop}<button class="item" onclick={onquit}><Power size={18}/><span class="label">Quit Oxbox</span></button>{/if}
    <span class="version">Oxbox {version}<span>Open source</span></span>
  </div>
</nav>
<style>
  .sidebar{display:flex;flex-direction:column;width:208px;flex-shrink:0;padding:var(--s-5) var(--s-3);background:var(--sidebar);border-right:1px solid var(--border);overflow-y:auto}
  .brand{display:flex;align-items:center;gap:var(--s-2);padding:var(--s-1) var(--s-2) var(--s-7)}.brand-name{display:block;font-size:var(--text-xl);font-weight:750;letter-spacing:-.04em}.brand-note{display:block;font-size:var(--text-xs);color:var(--text-3);margin-top:var(--s-1)}
  .nav{display:grid;gap:var(--s-1)}.section{margin:var(--s-7) var(--s-3) var(--s-3);font-size:var(--text-xs);font-weight:600;color:var(--text-3)}
  .item{display:flex;align-items:center;gap:var(--s-3);width:100%;min-height:40px;padding:var(--s-2) var(--s-3);border:1px solid transparent;border-radius:var(--r-md);background:transparent;color:var(--text-2);font-size:var(--text-sm);text-align:left;cursor:pointer}
  .item:hover{background:var(--surface-hover);color:var(--text)}.item[aria-current='page']{background:var(--surface);color:var(--accent-text);border-color:var(--border);font-weight:650;box-shadow:var(--shadow-sm)}
  .label{flex:1;min-width:0}.count{font-size:var(--text-xs);font-variant-numeric:tabular-nums;color:var(--text-3)}.footer{margin-top:auto;padding-top:var(--s-7)}.version{display:flex;justify-content:space-between;gap:var(--s-2);padding:var(--s-4) var(--s-3) 0;font-size:var(--text-xs);color:var(--text-3)}
  @media(max-width:700px){.sidebar{width:64px;padding:var(--s-4) var(--s-2)}.brand{padding:0 0 var(--s-5);justify-content:center}.brand>div,.count,.section,.version{display:none}.nav{margin-bottom:var(--s-4)}.label{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)}.item{justify-content:center;padding:var(--s-2)}.footer{padding-top:var(--s-4)}}
</style>

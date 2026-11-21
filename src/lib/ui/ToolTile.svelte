<script lang="ts">
  import Star from '@lucide/svelte/icons/star';
  import { categoryById, type ToolDef } from '#lib/registry.ts';
  interface Props { tool: ToolDef; favorite: boolean; slot?: number; onopen: (id:string)=>void; onfavorite: (id:string)=>void }
  let { tool, favorite, slot, onopen, onfavorite }: Props = $props();
</script>
<article class="tile">
  <button class="open" onclick={()=>onopen(tool.id)} data-tool={tool.id}>
    <span class="chip" aria-hidden="true"><tool.icon size={23} strokeWidth={1.7}/></span>
    <span class="name">{tool.name}</span>
    <span class="desc">{tool.description}</span>
    <span class="meta">{categoryById.get(tool.category)?.label}{#if slot && slot<=9}<kbd>Alt {slot}</kbd>{/if}</span>
  </button>
  <button class="pin" class:pinned={favorite} aria-pressed={favorite} aria-label={(favorite?'Remove ':'Add ')+tool.name+(favorite?' from favorites':' to favorites')} title={favorite?'Remove from dashboard':'Add to dashboard'} onclick={()=>onfavorite(tool.id)}>
    <Star size={16} strokeWidth={1.7} fill={favorite?'currentColor':'none'}/>
  </button>
</article>
<style>
  .tile{position:relative;min-width:0;border:1px solid var(--border);border-radius:var(--r-lg);background:var(--surface);box-shadow:var(--shadow-sm);transition:background 140ms,border-color 140ms}
  .tile:hover{border-color:var(--border-strong);background:var(--surface-hover)}
  .open{display:flex;flex-direction:column;align-items:flex-start;gap:var(--s-2);width:100%;height:100%;min-height:180px;padding:var(--s-5);border:0;border-radius:inherit;background:transparent;text-align:left;cursor:pointer}
  .chip{display:grid;place-items:center;width:42px;height:42px;border:1px solid var(--border);border-radius:var(--r-md);background:var(--surface-sunken);color:var(--text-2);margin-bottom:var(--s-2)}
  .name{font-weight:650;font-size:var(--text-md);letter-spacing:-.015em;color:var(--text);padding-right:var(--s-3)}
  .desc{font-size:var(--text-sm);line-height:1.5;color:var(--text-2);display:-webkit-box;-webkit-line-clamp:2;line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
  .meta{display:flex;align-items:center;justify-content:space-between;gap:var(--s-2);width:100%;margin-top:auto;padding-top:var(--s-3);font-size:var(--text-xs);color:var(--text-3)}
  kbd{font-family:var(--font-ui);border:1px solid var(--border);padding:0 var(--s-1);border-radius:var(--r-sm);white-space:nowrap}
  .pin{position:absolute;right:var(--s-3);top:var(--s-3);display:grid;place-items:center;width:32px;height:32px;border:0;border-radius:var(--r-sm);background:transparent;color:var(--text-3);cursor:pointer}
  .pin:hover{background:var(--accent-soft);color:var(--accent-text)}.pin.pinned{color:var(--accent-text)}
</style>

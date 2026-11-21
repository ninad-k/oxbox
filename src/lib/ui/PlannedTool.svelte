<script lang="ts">
  import Hammer from '@lucide/svelte/icons/hammer';
  import type { ToolDef } from '$lib/registry';

  let { tool }: { tool: ToolDef } = $props();
</script>

<div class="planned card" role="status">
  <span class="icon" aria-hidden="true"><Hammer size={22} /></span>
  <div>
    <h2>Coming in phase {tool.phase}</h2>
    <p>
      {tool.name} is on the roadmap but not built yet. The plan, acceptance criteria and suggested
      crates for it are in <code>tasks/plan.md</code> in the Oxbox repository. Contributions are welcome.
    </p>
  </div>
</div>

<style>
  .planned {
    display: flex;
    gap: var(--s-4);
    padding: var(--s-6);
    background: var(--surface-sunken);
  }
  .icon {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    border-radius: var(--r-md);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  h2 {
    font-size: var(--text-lg);
    margin-bottom: var(--s-1);
  }
  p {
    color: var(--text-2);
    max-width: 60ch;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.92em;
  }
</style>

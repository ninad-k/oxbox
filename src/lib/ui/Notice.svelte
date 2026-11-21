<script lang="ts">
  import type { Snippet } from 'svelte';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import Info from '@lucide/svelte/icons/info';

  interface Props {
    kind: 'info' | 'success' | 'error';
    children: Snippet;
  }
  let { kind, children }: Props = $props();
  const Icon = $derived(kind === 'success' ? CircleCheck : kind === 'error' ? CircleX : Info);
</script>

<!-- Icon + text, so state is never conveyed by colour alone. -->
<div class="notice {kind}" role={kind === 'error' ? 'alert' : 'status'}>
  <Icon size={18} aria-hidden="true" />
  <div>{@render children()}</div>
</div>

<style>
  .notice {
    display: flex;
    gap: var(--s-2);
    align-items: flex-start;
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-md);
    font-size: var(--text-sm);
    background: var(--info-soft);
    color: var(--text);
  }
  .notice :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
  }
  .success {
    background: var(--success-soft);
  }
  .success :global(svg) {
    color: var(--success);
  }
  .error {
    background: var(--danger-soft);
  }
  .error :global(svg) {
    color: var(--danger);
  }
</style>

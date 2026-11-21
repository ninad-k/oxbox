<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import Copy from '@lucide/svelte/icons/copy';

  let { text, label = 'Copy' }: { text: string; label?: string } = $props();
  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      clearTimeout(timer);
      timer = setTimeout(() => (copied = false), 1500);
    } catch {
      copied = false;
    }
  }
</script>

<button type="button" class="btn" onclick={copy} disabled={!text}>
  {#if copied}
    <Check size={15} aria-hidden="true" /> Copied
  {:else}
    <Copy size={15} aria-hidden="true" /> {label}
  {/if}
</button>
<span class="visually-hidden" aria-live="polite">{copied ? 'Copied to clipboard' : ''}</span>

<script lang="ts" generics="T extends string">
  /** Accessible single-choice button group (radio group semantics, arrow-key navigation). */
  interface Props {
    label: string;
    options: { value: T; label: string; hint?: string }[];
    value: T;
  }
  let { label, options, value = $bindable() }: Props = $props();

  let group: HTMLDivElement | undefined = $state();

  function onkeydown(e: KeyboardEvent) {
    const i = options.findIndex((o) => o.value === value);
    let next = i;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = (i + 1) % options.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = (i - 1 + options.length) % options.length;
    else return;
    e.preventDefault();
    value = options[next].value;
    group?.querySelectorAll<HTMLButtonElement>('[role=radio]')[next]?.focus();
  }
</script>

<div class="seg" role="radiogroup" aria-label={label} bind:this={group} tabindex="-1" {onkeydown}>
  {#each options as opt (opt.value)}
    <button
      type="button"
      role="radio"
      aria-checked={opt.value === value}
      tabindex={opt.value === value ? 0 : -1}
      title={opt.hint}
      onclick={() => (value = opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 3px;
    width: fit-content;
    max-width: 100%;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-sunken);
  }
  .seg:focus {
    outline: none;
  }
  button {
    height: 28px;
    padding: 0 var(--s-3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text-2);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  button:hover {
    color: var(--text);
  }
  button[aria-checked='true'] {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    box-shadow: var(--shadow-sm), inset 0 0 0 1px var(--border);
  }
</style>

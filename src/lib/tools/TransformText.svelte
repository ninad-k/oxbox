<!--
  Tool view: Transform Text (`transform-text`).
  Reference for a simple tool: input → Rust command → output, recomputed as the user types.
-->
<script lang="ts">
  import { api, ToolError, type TextCase } from '$lib/api';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import Notice from '$lib/ui/Notice.svelte';
  import Segmented from '$lib/ui/Segmented.svelte';

  const cases: { value: TextCase; label: string; hint: string }[] = [
    { value: 'title', label: 'Title Case', hint: 'Hello World' },
    { value: 'sentence', label: 'Sentence case', hint: 'Hello world' },
    { value: 'upper', label: 'UPPER', hint: 'HELLO WORLD' },
    { value: 'lower', label: 'lower', hint: 'hello world' },
    { value: 'camel', label: 'camelCase', hint: 'helloWorld' },
    { value: 'pascal', label: 'PascalCase', hint: 'HelloWorld' },
    { value: 'snake', label: 'snake_case', hint: 'hello_world' },
    { value: 'kebab', label: 'kebab-case', hint: 'hello-world' },
    { value: 'constant', label: 'CONSTANT_CASE', hint: 'HELLO_WORLD' },
  ];

  let input = $state('');
  let textCase = $state<TextCase>('title');
  let output = $state('');
  let error = $state<string | null>(null);
  let requestId = 0;

  $effect(() => {
    const text = input;
    const target = textCase;
    const id = ++requestId;
    if (!text) {
      output = '';
      error = null;
      return;
    }
    api
      .transformText(text, target)
      .then((result) => {
        if (id !== requestId) return; // a newer request superseded this one
        output = result;
        error = null;
      })
      .catch((e: unknown) => {
        if (id !== requestId) return;
        error = e instanceof ToolError ? e.message : String(e);
      });
  });
</script>

<div class="tool">
  <div class="field">
    <span class="field-label" id="case-label">Convert to</span>
    <Segmented label="Convert to" options={cases} bind:value={textCase} />
  </div>

  <div class="panes">
    <label class="field">
      <span class="field-label">Text</span>
      <textarea class="textarea" bind:value={input} placeholder="Type or paste text here" spellcheck="false"
      ></textarea>
    </label>
    <div class="field">
      <div class="row">
        <span class="field-label" id="result-label">Result</span>
        <CopyButton text={output} />
      </div>
      <textarea class="textarea result" aria-labelledby="result-label" readonly value={output}></textarea>
    </div>
  </div>

  {#if error}
    <Notice kind="error">{error}</Notice>
  {/if}
</div>

<style>
  .tool {
    display: grid;
    gap: var(--s-5);
  }
  .panes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--s-4);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 32px;
  }
  .field > .field-label {
    min-height: 32px;
    display: flex;
    align-items: center;
  }
  .textarea {
    min-height: 220px;
  }
  .result {
    background: var(--surface-sunken);
  }
</style>

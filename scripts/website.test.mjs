import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const catalogue = JSON.parse(fs.readFileSync(path.join(root, 'website/dist/tools.json'), 'utf8'));

function page() {
  const listeners = new Map();
  const controls = new Map();
  for (const name of ['tool-filters', 'tool-search', 'tool-category', 'tool-platform', 'result-count', 'empty-state', 'empty-reset']) {
    controls.set(`#${name}`, {
      value: ['tool-category', 'tool-platform'].includes(name) ? 'all' : '',
      hidden: true,
      addEventListener(event, handler) { listeners.set(`${name}:${event}`, handler); },
      focus() { this.focused = true; },
    });
  }
  const cards = catalogue.tools.map(tool => ({ hidden: false, dataset: {
    search: [tool.name, tool.description, ...tool.keywords, tool.note].join(' ').toLowerCase(),
    category: tool.category,
    platforms: tool.platforms.join(' '),
  } }));
  const form = controls.get('#tool-filters');
  const search = controls.get('#tool-search');
  const category = controls.get('#tool-category');
  const platform = controls.get('#tool-platform');
  // Native reset is cancelable and defaults are not cleared before reset listeners.
  // A microtask checkpoint can precede that default action (observed in browser QA).
  // Preserve that ordering so a queued filter of old values cannot pass this test.
  form.reset = async () => {
    const event = new Event('reset', { cancelable: true });
    listeners.get('tool-filters:reset')(event);
    await Promise.resolve();
    if (!event.defaultPrevented) {
      search.value = '';
      category.value = 'all';
      platform.value = 'all';
    }
  };
  vm.runInNewContext(fs.readFileSync(path.join(root, 'website/app.js'), 'utf8'), {
    document: { querySelector: key => controls.get(key), querySelectorAll: () => cards },
    queueMicrotask,
  });
  return { controls, listeners, cards, form, search, category, platform,
    filter: () => listeners.get('tool-filters:input')(),
    visible: () => cards.filter(card => !card.hidden),
  };
}

test('Reset clears incompatible search/platform filters and restores catalogue and status', async () => {
  const ui = page();
  ui.search.value = 'mouse';
  ui.filter();
  assert.equal(ui.visible().length, 1);
  ui.platform.value = 'macos';
  ui.filter();
  assert.equal(ui.visible().length, 0);
  await ui.form.reset();
  assert.equal(ui.search.value, '');
  assert.equal(ui.category.value, 'all');
  assert.equal(ui.platform.value, 'all');
  assert.equal(ui.visible().length, catalogue.tools.length);
  assert.equal(ui.controls.get('#result-count').textContent, `Showing all ${catalogue.tools.length} tools`);
  assert.equal(ui.controls.get('#empty-state').hidden, true);
});

test('Reset from the empty state clears all fields and returns focus to search', async () => {
  const ui = page();
  ui.search.value = 'missing-tool-fixture';
  ui.category.value = 'files';
  ui.platform.value = 'windows';
  ui.filter();
  ui.listeners.get('empty-reset:click')();
  await Promise.resolve();
  assert.equal(ui.visible().length, catalogue.tools.length);
  assert.equal(ui.category.value, 'all');
  assert.equal(ui.platform.value, 'all');
  assert.equal(ui.search.focused, true);
});

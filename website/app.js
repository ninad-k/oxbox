/* Progressive enhancement: the complete generated catalogue also works without JavaScript. */
const form = document.querySelector('#tool-filters');
const search = document.querySelector('#tool-search');
const category = document.querySelector('#tool-category');
const platform = document.querySelector('#tool-platform');
const cards = [...document.querySelectorAll('.tool-card')];
const count = document.querySelector('#result-count');
const empty = document.querySelector('#empty-state');
function filterTools() {
  const terms = search.value.trim().toLocaleLowerCase('en').split(/\s+/).filter(Boolean);
  let shown = 0;
  for (const card of cards) {
    const visible = terms.every(term => card.dataset.search.includes(term)) &&
      (category.value === 'all' || category.value === card.dataset.category) &&
      (platform.value === 'all' || card.dataset.platforms.split(' ').includes(platform.value));
    card.hidden = !visible;
    if (visible) shown++;
  }
  count.textContent = shown === cards.length ? `Showing all ${cards.length} tools` : `Showing ${shown} of ${cards.length} tools`;
  empty.hidden = shown !== 0;
}
if (form && search && category && platform && count && empty) {
  form.hidden = false;
  form.addEventListener('submit', event => event.preventDefault());
  form.addEventListener('input', filterTools);
  form.addEventListener('change', filterTools);
  form.addEventListener('reset', event => {
    // Reset listeners run before the browser's default value restoration.
    // Own the transition so fields, cards and the live status update together.
    event.preventDefault();
    search.value = '';
    category.value = 'all';
    platform.value = 'all';
    filterTools();
  });
  document.querySelector('#empty-reset').addEventListener('click', () => { form.reset(); search.focus(); });
  filterTools();
}

#!/usr/bin/env node
/** Static Pages build from the shared JSON catalogue. No application imports execute. */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = path.join(root, 'website');
const output = path.join(source, 'dist');
const catalogue = JSON.parse(fs.readFileSync(path.join(root, 'catalog/tools.json'), 'utf8'));
const iconCatalogue = JSON.parse(fs.readFileSync(path.join(root, 'catalog/icons.json'), 'utf8'));
const { categories, tools } = catalogue;
const notes = JSON.parse(fs.readFileSync(path.join(source, 'tool-notes.json'), 'utf8'));
if (!Array.isArray(categories) || !categories.length || !Array.isArray(tools) || !tools.length) throw new Error('The canonical catalogue needs categories and tools.');
if (!iconCatalogue.icons || typeof iconCatalogue.icons !== 'object' || Array.isArray(iconCatalogue.icons)) throw new Error('Missing canonical icon bindings.');
if (!notes || typeof notes !== 'object' || Array.isArray(notes)) throw new Error('Website note overrides must be an object.');
const stableId = value => typeof value === 'string' && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value);
const text = value => typeof value === 'string' && value.trim().length > 0;
const icon = value => stableId(value) && Object.hasOwn(iconCatalogue.icons, value);
const categoryById = new Map();
for (const category of categories) {
  if (!category || !stableId(category.id) || categoryById.has(category.id) || !text(category.label) || !icon(category.icon)) throw new Error('Invalid or duplicate canonical category: ' + category?.id);
  categoryById.set(category.id, category.label);
}
const ids = new Set();
for (const tool of tools) {
  if (!tool || !stableId(tool.id) || ids.has(tool.id)) throw new Error('Invalid or duplicate tool ID: ' + tool?.id);
  ids.add(tool.id);
  if (!text(tool.name) || !text(tool.description) || !categoryById.has(tool.category) || !icon(tool.icon)
    || !['ready', 'planned'].includes(tool.status) || !Number.isInteger(tool.phase) || tool.phase < 0
    || !Array.isArray(tool.keywords) || tool.keywords.some(keyword => !text(keyword))
    || !Array.isArray(tool.platforms) || !tool.platforms.length || new Set(tool.platforms).size !== tool.platforms.length
    || tool.platforms.some(os => !['windows', 'macos', 'linux'].includes(os))
    || (tool.note !== undefined && typeof tool.note !== 'string')) throw new Error('Incomplete canonical catalogue entry: ' + tool.id);
}
for (const [id, note] of Object.entries(notes)) {
  if (!ids.has(id)) throw new Error('Website note has no matching canonical tool: ' + id);
  if (typeof note !== 'string') throw new Error('Website note must be text: ' + id);
}
const noteFor = tool => Object.hasOwn(notes, tool.id) ? notes[tool.id] : tool.note ?? '';
const escape = value => String(value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const osNames = { windows: 'Windows', macos: 'macOS', linux: 'Linux' };
const cards = tools.map(tool => {
  const category = categoryById.get(tool.category);
  const search = [tool.name, tool.description, category, ...tool.keywords, noteFor(tool)].join(' ').toLowerCase();
  return `<article class="tool-card" id="tool-${escape(tool.id)}" data-category="${escape(tool.category)}" data-platforms="${escape(tool.platforms.join(' '))}" data-search="${escape(search)}"><div class="tool-card-top"><span class="tool-glyph" aria-hidden="true">${escape(tool.name.split(/\s+/).map(word => word[0]).slice(0, 2).join(''))}</span><span class="tool-category">${escape(category)}</span></div><h3>${escape(tool.name)}</h3><p>${escape(tool.description)}</p>${noteFor(tool) ? `<p class="tool-note">${escape(noteFor(tool))}</p>` : ''}<div class="tool-platforms"><span class="sr-only">Platforms in source: </span>${escape(tool.platforms.map(os => osNames[os]).join(' · '))}${tool.status === 'planned' ? ' · Planned' : ''}</div></article>`;
}).join('\n');
const substitutions = {
  TOOL_COUNT: tools.length,
  CATEGORY_COUNT: categories.length,
  TOOL_CARDS: cards,
  CATEGORY_OPTIONS: categories.map(category => `<option value="${escape(category.id)}">${escape(category.label)}</option>`).join(''),
};
let html = fs.readFileSync(path.join(source, 'index.html'), 'utf8');
for (const [key, value] of Object.entries(substitutions)) html = html.replaceAll(`{{${key}}}`, String(value));
if (/\{\{[A-Z_]+\}\}/.test(html)) throw new Error('Unresolved website template token.');
// Replace only this known generated directory, using one filesystem API and an absolute containment check.
if (path.dirname(output) !== source || path.basename(output) !== 'dist') throw new Error('Unsafe website output path.');
fs.rmSync(output, { recursive: true, force: true });
fs.mkdirSync(output, { recursive: true });
for (const file of ['styles.css', 'app.js']) fs.copyFileSync(path.join(source, file), path.join(output, file));
if (fs.existsSync(path.join(source, 'assets'))) fs.cpSync(path.join(source, 'assets'), path.join(output, 'assets'), { recursive: true });
fs.mkdirSync(path.join(output, 'assets'), { recursive: true });
fs.copyFileSync(path.join(root, 'static', 'oxbox.svg'), path.join(output, 'assets', 'oxbox.svg'));
fs.writeFileSync(path.join(output, 'index.html'), html);
fs.writeFileSync(path.join(output, 'tools.json'), `${JSON.stringify({ categories, tools: tools.map(tool => ({ ...tool, note: noteFor(tool) })) }, null, 2)}\n`);
fs.writeFileSync(path.join(output, '.nojekyll'), '');
// Reject missing local resources and root-relative URLs that would break a project Pages path.
for (const match of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
  const url = match[1];
  if (/^(?:https?:|mailto:|#)/.test(url)) continue;
  if (url.startsWith('/') || !fs.existsSync(path.join(output, url.split(/[?#]/)[0]))) throw new Error(`Missing or non-portable website resource: ${url}`);
}
const anchors = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]));
for (const match of html.matchAll(/href="#([^"]+)"/g)) if (!anchors.has(match[1])) throw new Error(`Missing website anchor: ${match[1]}`);
console.log(`Built website/dist: ${tools.length} canonical tools, ${categories.length} categories; local resources and anchors verified.`);

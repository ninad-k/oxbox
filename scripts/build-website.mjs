#!/usr/bin/env node
/** Static Pages build. Read registry literals through TypeScript's AST; never execute app imports. */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = path.join(root, 'website');
const output = path.join(source, 'dist');
const declarations = new Map();
for (const relative of ['src/lib/registry.ts', 'src/lib/roadmap-registry.ts']) {
  const file = ts.createSourceFile(relative, fs.readFileSync(path.join(root, relative), 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  for (const statement of file.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (ts.isIdentifier(declaration.name) && declaration.initializer) declarations.set(declaration.name.text, declaration.initializer);
    }
  }
}
const omitted = new Set(['icon', 'load', 'tone']);
function literal(node, locals = new Map(), stack = new Set()) {
  if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node) || ts.isSatisfiesExpression(node)) return literal(node.expression, locals, stack);
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
  if (ts.isNumericLiteral(node)) return Number(node.text);
  if (node.kind === ts.SyntaxKind.TrueKeyword) return true;
  if (node.kind === ts.SyntaxKind.FalseKeyword) return false;
  if (ts.isIdentifier(node)) {
    if (locals.has(node.text)) return locals.get(node.text);
    if (!declarations.has(node.text) || stack.has(node.text)) throw new Error(`Unsupported or cyclic catalogue identifier: ${node.text}`);
    return literal(declarations.get(node.text), locals, new Set([...stack, node.text]));
  }
  if (ts.isArrayLiteralExpression(node)) return node.elements.flatMap(item => ts.isSpreadElement(item) ? literal(item.expression, locals, stack) : [literal(item, locals, stack)]);
  if (ts.isObjectLiteralExpression(node)) {
    const value = {};
    for (const property of node.properties) {
      if (ts.isSpreadAssignment(property)) { Object.assign(value, literal(property.expression, locals, stack)); continue; }
      if (!ts.isPropertyAssignment(property) || !('text' in property.name)) throw new Error('Catalogue properties must be static named assignments.');
      if (!omitted.has(property.name.text)) value[property.name.text] = literal(property.initializer, locals, stack);
    }
    return value;
  }
  if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression) && node.expression.name.text === 'map') {
    const callback = node.arguments[0];
    if (node.arguments.length !== 1 || !ts.isArrowFunction(callback) || callback.parameters.length !== 1 || !ts.isIdentifier(callback.parameters[0].name)) throw new Error('Unsupported catalogue mapper.');
    return literal(node.expression.expression, locals, stack).map(item => literal(callback.body, new Map([...locals, [callback.parameters[0].name.text, item]]), stack));
  }
  throw new Error(`Unsupported catalogue syntax: ${ts.SyntaxKind[node.kind]}`);
}
const categories = literal(declarations.get('categories'));
const tools = literal(declarations.get('tools'));
const notes = JSON.parse(fs.readFileSync(path.join(source, 'tool-notes.json'), 'utf8'));
const categoryById = new Map(categories.map(item => [item.id, item.label]));
const ids = new Set();
for (const tool of tools) {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(tool.id) || ids.has(tool.id)) throw new Error(`Invalid or duplicate tool ID: ${tool.id}`);
  ids.add(tool.id);
  if (!tool.name || !tool.description || !categoryById.has(tool.category) || !['ready', 'planned'].includes(tool.status) || !Number.isInteger(tool.phase) || !Array.isArray(tool.keywords) || !tool.platforms?.length || tool.platforms.some(os => !['windows', 'macos', 'linux'].includes(os))) throw new Error(`Incomplete catalogue entry: ${tool.id}`);
}
for (const id of Object.keys(notes)) if (!ids.has(id)) throw new Error(`Website note has no matching registry tool: ${id}`);
const escape = value => String(value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const osNames = { windows: 'Windows', macos: 'macOS', linux: 'Linux' };
const cards = tools.map(tool => {
  const category = categoryById.get(tool.category);
  const search = [tool.name, tool.description, category, ...tool.keywords, notes[tool.id] || ''].join(' ').toLowerCase();
  return `<article class="tool-card" id="tool-${escape(tool.id)}" data-category="${escape(tool.category)}" data-platforms="${escape(tool.platforms.join(' '))}" data-search="${escape(search)}"><div class="tool-card-top"><span class="tool-glyph" aria-hidden="true">${escape(tool.name.split(/\s+/).map(word => word[0]).slice(0, 2).join(''))}</span><span class="tool-category">${escape(category)}</span></div><h3>${escape(tool.name)}</h3><p>${escape(tool.description)}</p>${notes[tool.id] ? `<p class="tool-note">${escape(notes[tool.id])}</p>` : ''}<div class="tool-platforms"><span class="sr-only">Platforms in source: </span>${escape(tool.platforms.map(os => osNames[os]).join(' · '))}${tool.status === 'planned' ? ' · Planned' : ''}</div></article>`;
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
fs.writeFileSync(path.join(output, 'tools.json'), `${JSON.stringify({ categories, tools: tools.map(tool => ({ ...tool, note: notes[tool.id] || '' })) }, null, 2)}\n`);
fs.writeFileSync(path.join(output, '.nojekyll'), '');
// Reject missing local resources and root-relative URLs that would break a project Pages path.
for (const match of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
  const url = match[1];
  if (/^(?:https?:|mailto:|#)/.test(url)) continue;
  if (url.startsWith('/') || !fs.existsSync(path.join(output, url.split(/[?#]/)[0]))) throw new Error(`Missing or non-portable website resource: ${url}`);
}
const anchors = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]));
for (const match of html.matchAll(/href="#([^"]+)"/g)) if (!anchors.has(match[1])) throw new Error(`Missing website anchor: ${match[1]}`);
console.log(`Built website/dist: ${tools.length} registry tools, ${categories.length} categories; local resources and anchors verified.`);

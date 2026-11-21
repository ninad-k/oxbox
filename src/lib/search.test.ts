import { describe, expect, it } from 'vitest';
import { filterTools, tokenize } from './search';
import { categories, tools } from './registry';

const label = (id: string) => categories.find((c) => c.id === id)?.label ?? id;
const ids = (list: { id: string }[]) => list.map((t) => t.id);

describe('filterTools', () => {
  it('returns every tool for the "all" view', () => {
    expect(filterTools(tools, { view: 'all', query: '', favorites: [] })).toHaveLength(tools.length);
  });

  it('filters by category', () => {
    const files = filterTools(tools, { view: 'files', query: '', favorites: [] });
    expect(files.length).toBeGreaterThan(0);
    expect(files.every((t) => t.category === 'files')).toBe(true);
  });

  it('keeps favourites in the order they were added and skips unknown ids', () => {
    const favs = ['verify-checksum', 'gone-tool', 'transform-text'];
    expect(ids(filterTools(tools, { view: 'favorites', query: '', favorites: favs }))).toEqual([
      'verify-checksum',
      'transform-text',
    ]);
  });

  it('searches every tool, ignoring the selected view', () => {
    const res = filterTools(tools, { view: 'time', query: 'sha256', favorites: [] });
    expect(ids(res)).toEqual(['verify-checksum']);
  });

  it('ranks name prefix matches before other matches', () => {
    const res = filterTools(tools, { view: 'all', query: 'screen', favorites: [] });
    expect(res[0].id).toBe('screenshot');
  });

  it('requires every token to match', () => {
    const res = filterTools(tools, { view: 'all', query: 'lock instantly', favorites: [] });
    expect(ids(res)).toEqual(['lock-screen']);
  });

  it('matches category labels', () => {
    const res = filterTools(tools, {
      view: 'all',
      query: 'privacy',
      favorites: [],
      categoryLabel: label as never,
    });
    expect(res.every((t) => t.category === 'privacy')).toBe(true);
    expect(res.length).toBeGreaterThan(0);
  });

  it('returns nothing for a query with no matches', () => {
    expect(filterTools(tools, { view: 'all', query: 'zzzz', favorites: [] })).toEqual([]);
  });
});

describe('registry', () => {
  it('has unique ids', () => {
    expect(new Set(ids(tools)).size).toBe(tools.length);
  });

  it('gives every ready tool a view', () => {
    for (const t of tools.filter((t) => t.status === 'ready')) {
      expect(t.load, t.id).toBeTypeOf('function');
    }
  });

  it('uses only known categories', () => {
    const known = new Set(categories.map((c) => c.id));
    expect(tools.every((t) => known.has(t.category))).toBe(true);
  });
});

describe('tokenize', () => {
  it('lowercases and splits on whitespace', () => {
    expect(tokenize('  Hash  FILE ')).toEqual(['hash', 'file']);
  });
});

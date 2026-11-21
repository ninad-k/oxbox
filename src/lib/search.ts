import type { CategoryId, ToolDef } from './registry';

export type View = 'all' | 'favorites' | CategoryId;

export interface FilterOptions {
  view: View;
  query: string;
  favorites: readonly string[];
  /** Used to match category names in search. */
  categoryLabel?: (id: CategoryId) => string;
}

/**
 * Tools to show for the current sidebar view and search box.
 * A non-empty query searches every tool, whatever view is selected.
 * Results are ranked: name prefix, then name contains, then other fields.
 */
export function filterTools(tools: readonly ToolDef[], opts: FilterOptions): ToolDef[] {
  const tokens = tokenize(opts.query);

  if (tokens.length === 0) {
    if (opts.view === 'all') return [...tools];
    if (opts.view === 'favorites') {
      // Keep the order the user favourited them in.
      return opts.favorites
        .map((id) => tools.find((t) => t.id === id))
        .filter((t): t is ToolDef => t !== undefined);
    }
    return tools.filter((t) => t.category === opts.view);
  }

  const scored: { tool: ToolDef; score: number }[] = [];
  for (const tool of tools) {
    const name = tool.name.toLowerCase();
    const haystack = [
      name,
      tool.description.toLowerCase(),
      tool.keywords.join(' ').toLowerCase(),
      opts.categoryLabel?.(tool.category).toLowerCase() ?? tool.category,
    ].join(' ');

    if (!tokens.every((tok) => haystack.includes(tok))) continue;

    const first = tokens[0];
    const score = name.startsWith(first) ? 0 : name.includes(first) ? 1 : 2;
    scored.push({ tool, score });
  }
  // Array.prototype.sort is stable, so equal scores keep catalogue order.
  return scored.sort((a, b) => a.score - b.score).map((s) => s.tool);
}

export function tokenize(query: string): string[] {
  return query.toLowerCase().split(/\s+/).filter(Boolean);
}
